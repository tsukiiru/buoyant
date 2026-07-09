mod file_system;
mod types;

use chrono::{DateTime, Datelike, Utc};
use eframe::egui::{
    self, Align, Button, Checkbox, Color32, Context, Id, Label, Modal, RichText, Sense, Stroke,
    TextEdit, Vec2,
};
use rayon::{
    iter::{IntoParallelRefIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use std::{
    collections::HashSet,
    env,
    ops::Sub,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{file_system::BANNED_CHARACTERS, types::*};

#[derive(PartialEq, Default)]
pub enum Property {
    #[default]
    Name,
    Accessed,
    Created,
    Type,
    Size,
}

fn main() -> eframe::Result {
    let native_opts = eframe::NativeOptions::default();

    eframe::run_native(
        "buoyant - egui",
        native_opts,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

struct App {
    ctx: Context,
    current_path: PathBuf,
    entries: Entries,
    current_index: Option<usize>,
    selected: HashSet<usize>,
    clipboard: Clipboard,
    modals: Modals,
    view_hidden: bool,
    sorting_by: Property,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = App {
            ctx: cc.egui_ctx.clone(),
            current_path: env::home_dir().unwrap(),
            modals: Modals::default(),
            entries: Entries::default(),
            clipboard: Clipboard::default(),
            selected: HashSet::with_capacity(20),
            current_index: None,
            view_hidden: true,
            sorting_by: Property::Name,
        };
        app.fetch_entries(None);

        app
    }

    fn fetch_entries(&mut self, prev_path: Option<PathBuf>) {
        // clear entries
        self.entries.children.iter_mut().for_each(|e| {
            e.name.clear();
            e.path = PathBuf::new();
            e.using = false;
            e.accessed = None;
            e.created = None;
            e.folder_size = None;
        });

        let fetch_current_path = file_system::read_dir(&self.current_path);

        if let Err(err) = &fetch_current_path {
            println!("{}", err);
        }

        let mut index: usize = 0;

        for path in fetch_current_path.unwrap() {
            let (accessed, created) = file_system::accessed_and_created(&path, &true, &true);

            self.push_entry(
                &TempEntry {
                    name: path.file_name().unwrap().to_str().unwrap(),
                    is_hidden: file_system::is_hidden(&path),
                    path: &path,
                    accessed,
                    created,
                    folder_size: file_system::folder_size(&path, &true),
                    file_size: file_system::file_size(&path, &true),
                },
                index,
            );
            index += 1;
        }

        self.entries.children.truncate(index);

        // freed some mem from the greedy alloc
        unsafe {
            unsafe extern "C" {
                fn malloc_trim(pad: usize) -> i32;
            }
            malloc_trim(0);
        }

        self.filter_entries(prev_path);
    }

    fn push_entry(&mut self, entry: &TempEntry, index: usize) {
        let (file_size, accessed, created, name, is_hidden, path, folder_size) = (
            entry.file_size,
            entry.accessed,
            entry.created,
            entry.name,
            entry.is_hidden,
            entry.path,
            entry.folder_size,
        );

        let entry_opt = self.entries.children.get_mut(index);

        if let Some(entry) = entry_opt {
            entry.is_hidden = is_hidden;
            entry.using = true;
            entry.file_size = file_size;
            entry.accessed = accessed;
            entry.created = created;
            entry.folder_size = folder_size;

            entry.name.push_str(name);
            entry.path.push(path);
        } else {
            let mut entry = Entry {
                is_hidden,
                folder_size,
                file_size,
                accessed,
                created,
                using: true,
                ..Default::default()
            };
            entry.name.push_str(name);
            entry.path.push(path);

            self.entries.children.push(entry);
        }
    }

    fn filter_entries(&mut self, prev_path: Option<PathBuf>) {
        self.entries.displaying.clear();
        self.current_index = None;
        self.selected.clear();

        for (i, entry) in self.entries.children.iter().enumerate() {
            if !entry.using || (!self.view_hidden && entry.is_hidden) {
                continue;
            }
            /*
            if let Some(modal) = &self.states.modals.search
            && !entry.name.contains(&modal.content.trim())
            {
            continue;
            }*/

            self.entries.displaying.push(i);
        }

        self.entries.displaying.par_sort_by(|a, b| {
            let (x, y) = (
                &self.entries.children[*a].is_hidden,
                &self.entries.children[*b].is_hidden,
            );
            y.cmp(x)
        });

        let mut last_hidden_index: usize = 0;

        for (index, entry_index) in self.entries.displaying.iter().enumerate() {
            if !self.entries.children[*entry_index].is_hidden {
                last_hidden_index = index;
                break;
            }
        }

        self.sort(last_hidden_index, true);
        self.sort(last_hidden_index, false);

        // highlight from lower directory if provided
        if let Some(path) = prev_path {
            self.entries
                .displaying
                .iter()
                .enumerate()
                .for_each(|(index, entry_index)| {
                    if let Some(entry) = self.entries.children.get(*entry_index)
                        && entry.path == path
                    {
                        self.current_index = Some(index.clone());
                    }
                });
        }
    }

    fn sort(&mut self, index: usize, is_from_start: bool) {
        let sorting_by = &self.sorting_by;
        let reference = &self.entries.children;
        let displaying = if is_from_start {
            &mut self.entries.displaying[..index]
        } else {
            &mut self.entries.displaying[index..]
        };

        match sorting_by {
            Property::Name => {
                let mut lowercased: Vec<(usize, String)> = displaying
                    .iter()
                    .map(|&entry_index| {
                        (
                            entry_index,
                            self.entries.children[entry_index].name.to_lowercase(),
                        )
                    })
                    .collect();

                lowercased.par_sort_by(|a, b| a.1.cmp(&b.1));
                displaying
                    .iter_mut()
                    .zip(lowercased.iter())
                    .for_each(|(d, (i, _))| *d = *i);
            }
            Property::Size => displaying.par_sort_by(|a, b| {
                let (x, y) = (&reference[*a].file_size, &reference[*b].file_size);
                x.cmp(y)
            }),
            Property::Created => displaying.par_sort_by(|a, b| {
                let (x, y) = (&reference[*a].created, &reference[*b].created);
                x.cmp(y)
            }),
            Property::Accessed => displaying.par_sort_by(|a, b| {
                let (x, y) = (&reference[*a].accessed, &reference[*b].accessed);
                x.cmp(y)
            }),
            _ => {} /*
                    Property::Type => displaying.par_sort_by(|a, b| {
                        let (x, y) = (&reference[*a].file_type, &reference[*b].file_type);
                        x.cmp(y)
                    }),*/
        }

        // check if reversed
    }

    fn nav(&mut self, to: &Path) {
        if to.is_file() {
            let res = Command::new("xdg-open").arg(to).spawn();
            if let Err(err) = res {
                println!("{}", err);
            }
            return;
        }

        if !to.is_dir() {
            return;
        }
        self.current_path = to.to_path_buf();
        self.fetch_entries(None);
    }

    fn nav_back(&mut self) {
        self.current_path.pop();
        self.fetch_entries(Some(self.current_path.clone()));
    }

    fn delete(&mut self) {
        let paths = self
            .selected
            .par_iter()
            .map(|entry_index| {
                self.entries
                    .children
                    .get(*entry_index)
                    .unwrap()
                    .path
                    .as_ref()
            })
            .collect::<Vec<&Path>>();

        file_system::delete(paths);
        self.fetch_entries(None);
    }

    fn create(&mut self, mode: bool) {
        // true: file
        // false: folder

        let overlay = if mode {
            self.modals.create_file.as_mut().unwrap()
        } else {
            self.modals.create_folder.as_mut().unwrap()
        };

        let mut content = overlay.content.trim();
        if content.starts_with("/") {
            content = &content[1..];
        }

        let try_create = file_system::create(&self.current_path, Path::new(content), mode);

        if let Some(error) = try_create {
            overlay.error.clear();
            overlay.error.push_str(error);
            return;
        }

        self.close_modal(ModalType::CreateFile);
        self.close_modal(ModalType::CreateFolder);
        self.fetch_entries(None);
        return;
    }

    fn rename(&mut self) {
        let rename_modal_opt = self.modals.rename.as_mut();
        if rename_modal_opt.is_none() {
            return;
        }

        let rename_modal = rename_modal_opt.unwrap();

        let content = rename_modal.content.trim();
        for char in BANNED_CHARACTERS {
            if content.contains(char) {
                rename_modal.error.clear();
                rename_modal
                    .error
                    .push_str("Containing invalid characters!");
                return;
            }
        }

        file_system::rename(&rename_modal.path.as_ref().unwrap(), &rename_modal.content);
        self.close_modal(ModalType::Rename);
        self.fetch_entries(None);
    }

    fn new_modal(&mut self, modal: ModalType, entry_index: Option<usize>) {
        match modal {
            ModalType::Rename => {
                let path = &self
                    .entries
                    .children
                    .get(entry_index.unwrap())
                    .unwrap()
                    .path;
                self.modals.rename = Some(InputModal {
                    content: path.file_name().unwrap().to_str().unwrap().to_string(),
                    path: Some(path.to_path_buf()),
                    error: String::new(),
                })
            }
            ModalType::CreateFile => {
                self.modals.create_file = Some(InputModal {
                    content: String::new(),
                    path: None,
                    error: String::new(),
                })
            }
            ModalType::CreateFolder => {
                self.modals.create_folder = Some(InputModal {
                    content: String::new(),
                    path: None,
                    error: String::new(),
                })
            }
            ModalType::Paste => self.modals.paste = Some(ChoiceModal {}),
        }
    }

    fn update_modal(&mut self, modal: ModalType, new_content: String) {
        match modal {
            ModalType::Rename => self.modals.rename.as_mut().unwrap().content = new_content,
            ModalType::CreateFile => {
                self.modals.create_file.as_mut().unwrap().content = new_content
            }
            ModalType::CreateFolder => {
                self.modals.create_folder.as_mut().unwrap().content = new_content
            }
            _ => {}
        }
    }

    fn close_modal(&mut self, modal: ModalType) {
        match modal {
            ModalType::Rename => self.modals.rename = None,
            ModalType::CreateFile => self.modals.create_file = None,
            ModalType::CreateFolder => self.modals.create_folder = None,
            ModalType::Paste => self.modals.paste = None,
        };
    }

    fn add_to_clipboard(&mut self, clipboard_mode: ClipboardMode) {
        let clipboard = &mut self.clipboard;
        clipboard.entries.clear();

        self.selected.iter().for_each(|i| {
            let _ = clipboard
                .entries
                .insert(self.entries.children.get(*i).unwrap().path.clone());
        });

        clipboard.mode = Some(clipboard_mode);
    }

    fn clear_clipboard(&mut self) {
        let clipboard = &mut self.clipboard;
        clipboard.entries.clear();
        clipboard.mode = None;
    }

    fn paste(&mut self, paste_type: PasteType) {
        let clipboard = &mut self.clipboard;
        let clipboard_mode = clipboard.mode.as_ref();

        if clipboard.entries.is_empty() || clipboard_mode.is_none() {
            return;
        }

        let mode = clipboard_mode.unwrap();

        match mode {
            ClipboardMode::Copy => {
                file_system::copy_dir(&clipboard.entries, &self.current_path, &paste_type);
            }
            ClipboardMode::Cut => {
                file_system::move_dir(&clipboard.entries, &self.current_path, &paste_type);

                clipboard.entries.clear();
                clipboard.mode = None;
            }
        }

        self.close_modal(ModalType::Paste);
        self.fetch_entries(None);
    }

    fn add_to_selected(&mut self, index: usize, is_ctrled: bool, is_shifted: bool) {
        if !is_shifted && !is_ctrled {
            self.selected.clear();
        }

        let end_index = if let Some(current_index) = self.current_index
            && is_shifted
        {
            current_index
        } else {
            index
        };

        let entry_index = self.entries.displaying[index];
        for i in index.min(end_index)..=end_index.max(index) {
            self.selected.insert(self.entries.displaying[i]);
        } // selecting everything between the two indicies

        if is_ctrled {
            if self.selected.contains(&index) {
                self.selected.remove(&entry_index);
            } else {
                self.selected.insert(entry_index);
            }
        }

        self.current_index = Some(index);
        self.selected.insert(entry_index);
    }

    fn remove_from_selected(&mut self, entry_index: usize) {
        self.selected.remove(&entry_index);
    }

    fn clear_selected(&mut self) {
        self.selected.clear();
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut i = 0;
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                let mut button = ui.add(
                    Button::new(RichText::new("<").size(14.0)).fill(egui::Color32::TRANSPARENT),
                );
                button.set_intrinsic_size(Vec2::new(400.0, 20.0));

                if button.clicked() {
                    self.nav_back();
                }

                ui.label(format!("{}", self.current_path.display()));
            });

            ui.heading("da buoyant file explorer!! :o");
            ui.separator();

            ui.horizontal(|ui| {
                ui.allocate_space(Vec2::new(45.0, 15.0));

                let mut grid = egui::Grid::new(i);
                i += 1;
                grid = grid.min_col_width(150.0);

                grid.show(ui, |ui| {
                    ui.add(Label::new("name").halign(Align::Min));
                    ui.add(Label::new("file size").halign(Align::Min));
                    ui.add(Label::new("accessed").halign(Align::Min));
                    ui.add(Label::new("created").halign(Align::Min));
                });
            });

            let bg_response = egui::ScrollArea::vertical().show(ui, |ui| {
                let bg_response = ui.interact(
                    ui.available_rect_before_wrap(),
                    Id::new(format!("explorer-area{}", i)),
                    Sense::click(),
                );
                i += 1;

                let (
                    mut pending_rename,
                    mut pending_delete,
                    mut pending_clipboard,
                    mut pending_add_selected,
                    mut pending_remove_selected,
                ) = (None, None, None, None, None);

                for (index, entry_index) in self.entries.displaying.clone().into_iter().enumerate()
                {
                    let mut is_selected = self.selected.contains(&entry_index);
                    let is_current_index = if let Some(i) = self.current_index {
                        index == i
                    } else {
                        false
                    };

                    let entry_opt = self.entries.children.get(entry_index);

                    if entry_opt.is_none() {
                        continue;
                    }

                    let entry = entry_opt.unwrap();

                    let mut pending_nav = None;
                    let mut frame = egui::Frame::NONE.inner_margin(8);
                    if is_selected {
                        frame = frame.fill(Color32::LIGHT_GREEN.gamma_multiply(0.3));
                    }
                    if is_current_index {
                        frame = frame.stroke(Stroke::new(1.0, Color32::WHITE.linear_multiply(0.3)));
                    }

                    frame.show(ui, |ui| {
                        let btn_response = ui.horizontal(|ui| {
                            let btn_response = ui.interact(
                                ui.available_rect_before_wrap(),
                                Id::new(format!("button{}", i)),
                                Sense::click(),
                            );

                            i += 1;

                            if ui
                                .add_sized([40.0, 15.0], Checkbox::new(&mut is_selected, ""))
                                .clicked()
                            {
                                if is_selected {
                                    pending_add_selected = Some(index);
                                } else {
                                    pending_remove_selected = Some(entry_index.clone());
                                }
                            }

                            let (mut name, mut accessed, mut created, mut file_size) = (
                                RichText::new(&entry.name).size(14.0),
                                RichText::new(format_date(entry.accessed)).size(14.0),
                                RichText::new(format_date(entry.created)).size(14.0),
                                RichText::new(file_system::bytes_to_string(
                                    entry.file_size.unwrap(),
                                ))
                                .size(14.0),
                            );

                            if entry.is_hidden {
                                name = name.color(Color32::WHITE.gamma_multiply(0.3));
                                accessed = accessed.color(Color32::WHITE.gamma_multiply(0.3));
                                created = created.color(Color32::WHITE.gamma_multiply(0.3));
                                file_size = file_size.color(Color32::WHITE.gamma_multiply(0.3));
                            }

                            let mut grid = egui::Grid::new(Id::new(i));
                            i += 1;

                            grid = grid.min_col_width(150.0);
                            grid.show(ui, |ui| {
                                ui.add(Label::new(name).selectable(false).halign(egui::Align::Min));
                                ui.add(
                                    Label::new(file_size)
                                        .selectable(false)
                                        .halign(egui::Align::Min),
                                );
                                ui.add(
                                    Label::new(accessed)
                                        .selectable(false)
                                        .halign(egui::Align::Min),
                                );
                                ui.add(
                                    Label::new(created)
                                        .selectable(false)
                                        .halign(egui::Align::Min),
                                );
                            });
                            btn_response
                        });

                        btn_response.inner.context_menu(|ui| {
                            ui.label(entry.name.clone());
                            if ui.button("rename").clicked() {
                                pending_rename = Some(entry_index);
                            }
                            if ui.button("delete").clicked() {
                                pending_delete = Some(entry_index);
                            }
                            if ui.button("cut").clicked() {
                                pending_add_selected = Some(index);
                                pending_clipboard = Some(ClipboardMode::Cut);
                            }
                            if ui.button("copy").clicked() {
                                pending_add_selected = Some(index);
                                pending_clipboard = Some(ClipboardMode::Copy);
                            }
                        });
                        if btn_response.inner.clicked() {
                            pending_add_selected = Some(index);
                        }

                        if btn_response.inner.double_clicked() {
                            pending_nav = Some(entry.path.clone());
                        }
                    });

                    if let Some(path) = pending_nav {
                        self.nav(&path);
                    }
                }

                if let Some(entry_index) = pending_rename {
                    self.new_modal(ModalType::Rename, Some(entry_index));
                }

                if let Some(entry_index) = pending_delete {
                    self.add_to_selected(entry_index, false, false);
                    self.delete();
                }

                if let Some(index) = pending_add_selected {
                    let ctrl_pressed = ui.input(|i| {
                        i.key_down(egui::Key::ControlLeft) || i.key_down(egui::Key::ControlRight)
                    });
                    let shift_pressed = ui.input(|i| {
                        i.key_down(egui::Key::ShiftLeft) || i.key_down(egui::Key::ShiftRight)
                    });

                    self.add_to_selected(index, ctrl_pressed, shift_pressed);
                }

                if let Some(entry_index) = pending_remove_selected {
                    self.remove_from_selected(entry_index);
                }

                if let Some(mode) = pending_clipboard {
                    self.add_to_clipboard(mode);
                }

                bg_response
            });

            if bg_response.inner.clicked()
                && !(ui.input(|i| {
                    i.key_pressed(egui::Key::ControlLeft)
                        && i.key_pressed(egui::Key::ControlRight)
                        && i.key_pressed(egui::Key::ShiftLeft)
                        && i.key_pressed(egui::Key::ShiftRight)
                }))
            {
                self.clear_selected();
            }

            bg_response.inner.context_menu(|ui| {
                ui.label("create");
                if ui.button("create file").clicked() {
                    self.new_modal(ModalType::CreateFile, None);
                }
                if ui.button("create folder").clicked() {
                    self.new_modal(ModalType::CreateFolder, None);
                }

                ui.separator();
                ui.label("clipboard");

                let (mut cut_label, mut copy_label, mut clear_s_label) = (
                    RichText::new("cut"),
                    RichText::new("copy"),
                    RichText::new("clear selection"),
                );

                if self.selected.is_empty() {
                    cut_label = cut_label.color(Color32::WHITE.gamma_multiply(0.5));
                    copy_label = copy_label.color(Color32::WHITE.gamma_multiply(0.5));
                    clear_s_label = clear_s_label.color(Color32::WHITE.gamma_multiply(0.5));
                }

                let (mut cut_button, mut copy_button, mut clear_s_button) = (
                    Button::new(cut_label).stroke(Stroke::NONE),
                    Button::new(copy_label).stroke(Stroke::NONE),
                    Button::new(clear_s_label).stroke(Stroke::NONE),
                );

                if self.selected.is_empty() {
                    cut_button = cut_button.sense(Sense::empty());
                    copy_button = copy_button.sense(Sense::empty());
                    clear_s_button = clear_s_button.sense(Sense::empty());
                }

                if ui.add(cut_button).clicked() {
                    self.add_to_clipboard(ClipboardMode::Cut);
                }
                if ui.add(copy_button).clicked() {
                    self.add_to_clipboard(ClipboardMode::Copy);
                }
                if ui.add(clear_s_button).clicked() {
                    self.clear_selected();
                }

                let (mut p_text, mut cp_text) =
                    (RichText::new("paste"), RichText::new("clear clipboard"));

                if self.clipboard.entries.is_empty() {
                    p_text = p_text.color(Color32::WHITE.gamma_multiply(0.5));
                    cp_text = cp_text.color(Color32::WHITE.gamma_multiply(0.5));
                }

                let (mut paste_button, mut clearcp_button) = (
                    Button::new(p_text).stroke(Stroke::NONE),
                    Button::new(cp_text).stroke(Stroke::NONE),
                );

                if self.clipboard.entries.is_empty() {
                    paste_button = paste_button.sense(Sense::empty());
                    clearcp_button = clearcp_button.sense(Sense::empty());
                }

                if ui.add(paste_button).clicked() {
                    self.new_modal(ModalType::Paste, None);
                }
                if ui.add(clearcp_button).clicked() {
                    self.clear_clipboard();
                }
            });
        });

        if let Some(modal) = &self.modals.rename {
            let modal_widget = Modal::new(Id::new("rename_modal"));
            let mut content = modal.content.clone();
            let error = &modal.error.clone();

            modal_widget.show(&self.ctx.clone(), |ui| {
                ui.heading("renaming");
                let input = ui.add(TextEdit::singleline(&mut content));
                ui.add(Label::new(RichText::new(error).color(Color32::LIGHT_RED)));

                if input.changed() {
                    self.update_modal(ModalType::Rename, content);
                }

                if input.lost_focus() {
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.rename();
                    } else {
                        self.close_modal(ModalType::Rename);
                    }
                }

                input.request_focus();
            });
        }

        if let Some(modal) = &self.modals.create_file {
            let modal_widget = Modal::new(Id::new("create_file_modal"));
            let mut content = modal.content.clone();
            let error = &modal.error.clone();

            modal_widget.show(&self.ctx.clone(), |ui| {
                ui.label(format!("creating file at {}", self.current_path.display()));
                let input = ui.add(TextEdit::singleline(&mut content));
                ui.add(Label::new(RichText::new(error).color(Color32::LIGHT_RED)));

                if input.changed() {
                    self.update_modal(ModalType::CreateFile, content);
                }

                if input.lost_focus() {
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.create(true);
                    } else {
                        self.close_modal(ModalType::CreateFile);
                    }
                }

                input.request_focus();
            });
        }

        if let Some(modal) = &self.modals.create_folder {
            let modal_widget = Modal::new(Id::new("create_folder_modal"));
            let mut content = modal.content.clone();
            let error = &modal.error.clone();

            modal_widget.show(&self.ctx.clone(), |ui| {
                ui.label(format!(
                    "creating folder at {}",
                    self.current_path.display()
                ));
                let input = ui.add(TextEdit::singleline(&mut content));
                ui.add(Label::new(RichText::new(error).color(Color32::LIGHT_RED)));

                if input.changed() {
                    self.update_modal(ModalType::CreateFolder, content);
                }

                if input.lost_focus() {
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.create(false);
                    } else {
                        self.close_modal(ModalType::CreateFolder);
                    }
                }

                input.request_focus();
            });
        }

        let mut pending_paste = None;

        if let Some(_modal) = &self.modals.paste {
            let modal_widget = Modal::new(Id::new("paste_modal"));

            modal_widget.show(&self.ctx.clone(), |ui| {
                ui.heading("choose pasting type");

                ui.vertical(|ui| {
                    if ui.button("replace").clicked() {
                        pending_paste = Some(PasteType::Replace);
                    }
                    ui.label("replace if file(s) with the same name already existed");
                });
                ui.vertical(|ui| {
                    if ui.button("duplicate").clicked() {
                        pending_paste = Some(PasteType::Duplicate);
                    }
                    ui.label("make a duplicate if file(s) with the same name already existed");
                });

                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.close_modal(ModalType::Paste);
                }
            });
        };

        if let Some(paste_type) = pending_paste {
            self.paste(paste_type);
        }
    }
}

fn format_date(date: Option<i64>) -> String {
    let current_date = Utc::now();
    let given_date = DateTime::from_timestamp_secs(date.unwrap_or_default()).unwrap_or_default();

    let current_day = current_date.day();
    let given_day = given_date.day();

    let time_delta = current_date.sub(given_date);
    let delta_day = time_delta.num_hours() / 24;

    // today
    if delta_day < 1 && current_day == given_day {
        return format!("Today, {}", given_date.format("%I:%M %p"));
    }
    // yesterday
    else if delta_day < 2 {
        return format!("Yesterday, {}", given_date.format("%I:%M %p"));
    }
    // this week
    else if delta_day <= 7 {
        return format!("{} days ago", delta_day);
    }
    // last week
    else if delta_day <= 14 {
        return String::from("Last week");
    }
    // this month
    else if delta_day <= 31 {
        return format!("{} weeks ago", delta_day / 7);
    }
    // last month
    else if delta_day <= 62 {
        return String::from("Last month");
    }
    // this year
    else if delta_day <= 365 {
        return format!("{} months ago", delta_day / 31);
    }
    // last year
    else if delta_day <= 730 {
        return String::from("Last year");
    }
    // blah blah blah
    else {
        return format!("{} years ago", delta_day / 365);
    }
}
