mod file_system;
mod types;

use chrono::{DateTime, Datelike, Utc};
use eframe::egui::{
    Align, Button, CentralPanel, Color32, Context, Frame, Grid, Id, Key, Label, Modal, RichText,
    ScrollArea, Sense, Stroke, TextEdit, Ui, Vec2,
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
    sync::Mutex,
};

use crate::{file_system::BANNED_CHARACTERS, types::*};

#[derive(PartialEq, Default)]
pub enum Property {
    #[default]
    Name,
    Accessed,
    Created,
    Kind,
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
    toasts: Vec<Mutex<Toast>>, // mhm toasts
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
            toasts: Vec::with_capacity(5),
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
                    Property::Kind => displaying.par_sort_by(|a, b| {
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

        self.close_modal(ModalKind::CreateFile);
        self.close_modal(ModalKind::CreateFolder);
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
        self.close_modal(ModalKind::Rename);
        self.fetch_entries(None);
    }

    fn new_modal(&mut self, modal: ModalKind) {
        match modal {
            ModalKind::Rename => {
                let path = &self
                    .entries
                    .entry(&self.current_index.unwrap())
                    .unwrap()
                    .path;
                self.modals.rename = Some(InputModal {
                    content: path.file_name().unwrap().to_str().unwrap().to_string(),
                    path: Some(path.to_path_buf()),
                    error: String::new(),
                })
            }
            ModalKind::CreateFile => {
                self.modals.create_file = Some(InputModal {
                    content: String::new(),
                    path: None,
                    error: String::new(),
                })
            }
            ModalKind::CreateFolder => {
                self.modals.create_folder = Some(InputModal {
                    content: String::new(),
                    path: None,
                    error: String::new(),
                })
            }
            ModalKind::Paste => self.modals.paste = Some(ChoiceModal {}),
            ModalKind::Delete => self.modals.delete = Some(ChoiceModal {}),
            ModalKind::Metadata => self.modals.metadata = Some(InfoModal {}),
        }
    }

    fn update_modal(&mut self, modal: ModalKind, new_content: String) {
        match modal {
            ModalKind::Rename => self.modals.rename.as_mut().unwrap().content = new_content,
            ModalKind::CreateFile => {
                self.modals.create_file.as_mut().unwrap().content = new_content
            }
            ModalKind::CreateFolder => {
                self.modals.create_folder.as_mut().unwrap().content = new_content
            }
            _ => {}
        }
    }

    fn close_modal(&mut self, modal: ModalKind) {
        match modal {
            ModalKind::Rename => self.modals.rename = None,
            ModalKind::CreateFile => self.modals.create_file = None,
            ModalKind::CreateFolder => self.modals.create_folder = None,
            ModalKind::Paste => self.modals.paste = None,
            ModalKind::Delete => self.modals.delete = None,
            ModalKind::Metadata => self.modals.metadata = None,
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

    fn paste(&mut self, paste_type: PasteKind) {
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

        self.close_modal(ModalKind::Paste);
        self.fetch_entries(None);
    }

    fn swap_selected(&mut self, index: usize) {
        let entry_index_opt = self.entries.displaying.get(index);
        if let Some(entry_index) = entry_index_opt
            && !self.selected.contains(entry_index)
        {
            self.selected.insert(*entry_index);
        }

        self.current_index = Some(index);
    }

    fn modify_selected(&mut self, index: usize, is_ctrled: bool, is_shifted: bool) {
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

        if is_shifted {
            for i in index.min(end_index)..=end_index.max(index) {
                self.selected
                    .insert(*self.entries.displaying.get(i).unwrap());
            } // selecting everything between the two indicies
        }

        let entry_index = self.entries.displaying.get(index).unwrap();
        if is_ctrled {
            if self.selected.contains(entry_index) {
                self.selected.remove(entry_index);
                self.current_index = Some(index);
                return;
            } else {
                self.selected.insert(*entry_index);
            }
        }

        self.current_index = Some(index);
        self.selected.insert(*entry_index);
    }

    fn clear_selected(&mut self) {
        self.selected.clear();
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let mut i = 0;

        CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                let mut button =
                    ui.add(Button::new(RichText::new("<").size(14.0)).fill(Color32::TRANSPARENT));
                button.set_intrinsic_size(Vec2::new(400.0, 20.0));

                if button.clicked() {
                    self.nav_back();
                }

                ui.label(format!("{}", self.current_path.display()));
            });

            ui.heading("da buoyant file explorer!! :o");
            ui.separator();

            ui.horizontal(|ui| {
                ui.allocate_space(Vec2::new(2.0, 0.0));

                let mut grid = Grid::new(i);
                i += 1;
                grid = grid.min_col_width(150.0);

                grid.show(ui, |ui| {
                    ui.add(Label::new("name").halign(Align::Min));
                    ui.add(Label::new("file size").halign(Align::Min));
                    ui.add(Label::new("accessed").halign(Align::Min));
                    ui.add(Label::new("created").halign(Align::Min));
                });
            });

            let bg_response = ScrollArea::vertical().max_width(700.0).show(ui, |ui| {
                let bg_response = ui.interact(
                    ui.available_rect_before_wrap(),
                    Id::new(format!("explorer-area{}", i)),
                    Sense::click(),
                );
                i += 1;

                let (
                    mut pending_rename,
                    mut pending_delete_modal,
                    mut pending_clipboard,
                    mut pending_swap_selected,
                    mut pending_modify_selected,
                    mut pending_metadata_modal,
                ) = (None, None, None, None, None, None);

                for (index, entry_index) in self.entries.displaying.clone().into_iter().enumerate()
                {
                    let is_selected = self.selected.contains(&entry_index);
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
                    ui.horizontal(|ui| {
                        let mut frame = Frame::NONE
                            .inner_margin(8.0)
                            .stroke(Stroke::new(1.0, Color32::TRANSPARENT));
                        if is_selected {
                            frame.fill = Color32::LIGHT_GREEN.gamma_multiply(0.3);
                        }
                        if is_current_index {
                            frame.stroke.color = Color32::WHITE.linear_multiply(0.3);
                        }

                        let fr = frame.show(ui, |f| {
                            let (mut name, mut accessed, mut created, mut file_size) = (
                                RichText::new(&entry.name).size(14.0),
                                RichText::new(format_date(entry.accessed)).size(14.0),
                                RichText::new(format_date(entry.created)).size(14.0),
                                RichText::new(if let Some(size) = &entry.folder_size {
                                    format!("{} items", size)
                                } else {
                                    file_system::bytes_to_string(entry.file_size.unwrap())
                                })
                                .size(14.0),
                            );

                            if entry.is_hidden {
                                name = name.color(Color32::WHITE.gamma_multiply(0.3));
                                accessed = accessed.color(Color32::WHITE.gamma_multiply(0.3));
                                created = created.color(Color32::WHITE.gamma_multiply(0.3));
                                file_size = file_size.color(Color32::WHITE.gamma_multiply(0.3));
                            }

                            let mut grid = Grid::new(Id::new(i));
                            i += 1;

                            grid = grid.min_col_width(150.0);
                            grid.show(f, |g| {
                                g.add(Label::new(name).selectable(false).halign(Align::Min));
                                g.add(Label::new(file_size).selectable(false).halign(Align::Min));
                                g.add(Label::new(accessed).selectable(false).halign(Align::Min));
                                g.add(Label::new(created).selectable(false).halign(Align::Min));
                            });
                        });

                        let btn_response =
                            ui.interact(fr.response.rect, Id::new(i), Sense::click());
                        i += 1;

                        btn_response.context_menu(|ui| {
                            ui.label(entry.name.clone());
                            if ui.button("rename").clicked() {
                                pending_rename = Some(());
                            }
                            if ui.button("delete").clicked() {
                                pending_delete_modal = Some(());
                            }
                            if ui.button("cut").clicked() {
                                pending_clipboard = Some(ClipboardMode::Cut);
                            }
                            if ui.button("copy").clicked() {
                                pending_clipboard = Some(ClipboardMode::Copy);
                            }
                            if ui.button("info").clicked() {
                                pending_metadata_modal = Some(());
                            }
                        });

                        let mut double_clicking = false;
                        if btn_response.double_clicked() {
                            double_clicking = true;
                            pending_nav = Some(entry.path.clone());
                        }

                        if btn_response.clicked() && !double_clicking {
                            pending_modify_selected = Some(index);
                        }

                        if btn_response.secondary_clicked() {
                            pending_swap_selected = Some(index);
                        }
                    });

                    if let Some(path) = pending_nav {
                        self.nav(&path);
                    }
                }

                if let Some(index) = pending_swap_selected {
                    self.swap_selected(index);
                }

                if pending_rename.is_some() {
                    self.new_modal(ModalKind::Rename);
                }

                if pending_delete_modal.is_some() {
                    self.new_modal(ModalKind::Delete);
                }

                if let Some(index) = pending_modify_selected {
                    let ctrl_pressed =
                        ui.input(|i| i.key_down(Key::ControlLeft) || i.key_down(Key::ControlRight));
                    let shift_pressed =
                        ui.input(|i| i.key_down(Key::ShiftLeft) || i.key_down(Key::ShiftRight));

                    self.modify_selected(index, ctrl_pressed, shift_pressed);
                }

                if let Some(mode) = pending_clipboard {
                    self.add_to_clipboard(mode);
                }

                if pending_metadata_modal.is_some() {
                    self.new_modal(ModalKind::Metadata);
                }

                bg_response
            });

            if bg_response.inner.clicked()
                && !(ui.input(|i| {
                    i.key_pressed(Key::ControlLeft)
                        && i.key_pressed(Key::ControlRight)
                        && i.key_pressed(Key::ShiftLeft)
                        && i.key_pressed(Key::ShiftRight)
                }))
            {
                self.clear_selected();
            }

            bg_response.inner.context_menu(|ui| {
                ui.label("create");
                if ui.button("create file").clicked() {
                    self.new_modal(ModalKind::CreateFile);
                }
                if ui.button("create folder").clicked() {
                    self.new_modal(ModalKind::CreateFolder);
                }

                ui.separator();
                ui.label("clipboard");

                let (mut del_label, mut cut_label, mut copy_label, mut clear_s_label) = (
                    RichText::new("delete"),
                    RichText::new("cut"),
                    RichText::new("copy"),
                    RichText::new("clear selection"),
                );

                if self.selected.is_empty() {
                    del_label = del_label.color(Color32::WHITE.gamma_multiply(0.5));
                    cut_label = cut_label.color(Color32::WHITE.gamma_multiply(0.5));
                    copy_label = copy_label.color(Color32::WHITE.gamma_multiply(0.5));
                    clear_s_label = clear_s_label.color(Color32::WHITE.gamma_multiply(0.5));
                }

                let (mut del_button, mut cut_button, mut copy_button, mut clear_s_button) = (
                    Button::new(del_label).stroke(Stroke::NONE),
                    Button::new(cut_label).stroke(Stroke::NONE),
                    Button::new(copy_label).stroke(Stroke::NONE),
                    Button::new(clear_s_label).stroke(Stroke::NONE),
                );

                if self.selected.is_empty() {
                    del_button = del_button.sense(Sense::empty());
                    cut_button = cut_button.sense(Sense::empty());
                    copy_button = copy_button.sense(Sense::empty());
                    clear_s_button = clear_s_button.sense(Sense::empty());
                }

                if ui.add(del_button).clicked() {
                    self.new_modal(ModalKind::Delete);
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
                    self.new_modal(ModalKind::Paste);
                }
                if ui.add(clearcp_button).clicked() {
                    self.clear_clipboard();
                }
            });
        });

        // modals

        if let Some(modal) = &self.modals.rename {
            let modal_widget = Modal::new(Id::new("rename_modal"));
            let mut content = modal.content.clone();
            let error = &modal.error.clone();

            modal_widget.show(&self.ctx.clone(), |ui| {
                ui.heading("renaming");
                let input = ui.add(TextEdit::singleline(&mut content));
                ui.add(Label::new(RichText::new(error).color(Color32::LIGHT_RED)));

                if input.changed() {
                    self.update_modal(ModalKind::Rename, content);
                }

                if input.lost_focus() {
                    if ui.input(|i| i.key_pressed(Key::Enter)) {
                        self.rename();
                    } else {
                        self.close_modal(ModalKind::Rename);
                    }
                }

                input.request_focus();
            });
        }

        if let Some(modal) = &self.modals.create_file {
            let modal_widget = Modal::new(Id::new("create_file_modal"));
            let mut content = modal.content.clone();
            let error = modal.error.clone();

            modal_widget.show(&self.ctx.clone(), |ui| {
                ui.label(format!("creating file at {}", self.current_path.display()));
                let input = ui.add(TextEdit::singleline(&mut content));
                ui.add(Label::new(RichText::new(error).color(Color32::LIGHT_RED)));

                if input.changed() {
                    self.update_modal(ModalKind::CreateFile, content);
                }

                if input.lost_focus() {
                    if ui.input(|i| i.key_pressed(Key::Enter)) {
                        self.create(true);
                    } else {
                        self.close_modal(ModalKind::CreateFile);
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
                    self.update_modal(ModalKind::CreateFolder, content);
                }

                if input.lost_focus() {
                    if ui.input(|i| i.key_pressed(Key::Enter)) {
                        self.create(false);
                    } else {
                        self.close_modal(ModalKind::CreateFolder);
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
                        pending_paste = Some(PasteKind::Replace);
                    }
                    ui.label("replace if file(s) with the same name already existed");
                });
                ui.vertical(|ui| {
                    if ui.button("duplicate").clicked() {
                        pending_paste = Some(PasteKind::Duplicate);
                    }
                    ui.label("make a duplicate if file(s) with the same name already existed");
                });

                if ui.input(|i| i.key_pressed(Key::Escape)) {
                    self.close_modal(ModalKind::Paste);
                }
            });
        };

        if let Some(paste_type) = pending_paste {
            self.paste(paste_type);
        }

        let mut pending_delete = false;
        let mut pending_close_delete = false;

        if let Some(_modal) = &self.modals.delete {
            let modal_widget = Modal::new(Id::new("delete_modal"));
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

            modal_widget.show(&self.ctx.clone(), |w| {
                w.label("are you sure you wanna delete these?");

                Frame::new()
                    .fill(Color32::BLACK.gamma_multiply(0.7))
                    .corner_radius(4.0)
                    .inner_margin(2.0)
                    .show(w, |u| {
                        ScrollArea::vertical().max_height(200.0).show(u, |b| {
                            paths.iter().for_each(|path| {
                                b.label(format!("{}", path.display()));
                            });
                        });
                    });

                w.separator();
                w.horizontal(|u| {
                    if u.button("yeah").clicked() {
                        pending_delete = true;
                    }
                    if u.button("no").clicked() || u.input(|i| i.key_pressed(Key::Escape)) {
                        pending_close_delete = true;
                    }
                })
            });
        }

        if pending_delete {
            self.delete();
        }
        if pending_close_delete {
            self.close_modal(ModalKind::Delete);
        }

        let mut pending_close_metadata = false;

        if let Some(_modal) = &self.modals.metadata {
            let modal_widget = Modal::new(Id::new("metadata_modal"));
            let entry = &self.entries.entry(&self.current_index.unwrap()).unwrap();

            modal_widget.show(&self.ctx.clone(), |m| {
                m.label(format!("showing metadata for {}", entry.name));
                m.separator();
                m.label(format!("full path: {}", entry.path.display()));
                m.label(format!(
                    "last accessed date: {}",
                    format_date(entry.accessed)
                ));
                m.label(format!("created date: {}", format_date(entry.created)));
                m.label(if let Some(size) = &entry.folder_size {
                    format!("folder size: {} items", size)
                } else {
                    format!(
                        "file size: {}",
                        file_system::bytes_to_string(entry.file_size.unwrap_or_default())
                    )
                });

                if m.input(|i| i.key_pressed(Key::Escape)) {
                    pending_close_metadata = true;
                }
            });
        }

        if pending_close_metadata {
            self.close_modal(ModalKind::Metadata);
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
