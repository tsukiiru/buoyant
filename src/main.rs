mod config;
mod file_system;
mod file_types;
mod types;

use chrono::{DateTime, Datelike, Utc};
use eframe::egui::{
    Align, Align2, Button, CentralPanel, Color32, Context, Event, Frame, Grid, Id, Key,
    KeyboardShortcut, Label, Modal, ProgressBar, RichText, ScrollArea, Sense, Stroke, TextEdit, Ui,
    Vec2, Window,
};
use rayon::{
    iter::{
        IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator,
        ParallelIterator,
    },
    slice::ParallelSliceMut,
};
use std::{
    collections::HashSet,
    env,
    ops::Sub,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio;

use crate::{config::*, file_system::BANNED_CHARACTERS, types::*};

#[derive(PartialEq, Default)]
pub enum Property {
    #[default]
    Name,
    Accessed,
    Created,
    Type,
    Size,
}

#[tokio::main]
async fn main() -> eframe::Result {
    let native_opts = eframe::NativeOptions::default();

    eframe::run_native(
        "buoyant - egui",
        native_opts,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

type Toasts = Arc<Mutex<Vec<Toast>>>;

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
    config: Config,
    actions: Actions,
    toasts: Toasts, // mhm toasts
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
            config: Config::default(),
            actions: Actions::default(),
            toasts: Arc::new(Mutex::new(Vec::with_capacity(5))),
        };
        app.fetch_entries(None);
        config::fetch(&mut app.config);
        app.bind_keybinds();

        app
    }

    fn bind_keybinds(&mut self) {
        let keybinds_list = &mut self.config.keybinds_list;
        let actions = &mut self.actions;
        let copy_sc = KeyboardShortcut::new(CTRL, Key::C);
        let cut_sc = KeyboardShortcut::new(CTRL, Key::X);
        let paste_sc = KeyboardShortcut::new(CTRL, Key::V);

        for (action, sc) in keybinds_list {
            if sc.modifiers.matches_logically(copy_sc.modifiers)
                && sc.logical_key == copy_sc.logical_key
            {
                actions.copy = action.clone();
            }

            if sc.modifiers.matches_logically(cut_sc.modifiers)
                && sc.logical_key == cut_sc.logical_key
            {
                actions.cut = action.clone();
            }

            if sc.modifiers.matches_logically(paste_sc.modifiers)
                && sc.logical_key == paste_sc.logical_key
            {
                actions.paste = action.clone();
            }
        }
    }

    fn handle_actions(&mut self, action: &KeybindAction, is_ctrled: bool, is_shifted: bool) {
        match action {
            KeybindAction::NavigateUp => {
                self.navigate_index(&NavigateDirection::Up, is_ctrled, is_shifted)
            }

            KeybindAction::NavigateDown => {
                self.navigate_index(&NavigateDirection::Down, is_ctrled, is_shifted);
            }
            KeybindAction::NavigateForward => self.nav_forward(),
            KeybindAction::NavigateBackward => self.nav_back(),

            KeybindAction::Copy => {
                if self.selected.len() < 1 {
                    self.new_toast(
                        String::from("Clipboard"),
                        String::from("nothing is selected to be copied!"),
                        ToastKind::Info,
                        Duration::from_secs(3),
                    );
                    return;
                }

                self.new_toast(
                    String::from("Clipboard"),
                    format!(
                        "successfully add {} items for copying!",
                        self.selected.len()
                    ),
                    ToastKind::Success,
                    Duration::from_secs(3),
                );
                self.add_to_clipboard(ClipboardMode::Copy);
            }
            KeybindAction::Cut => {
                if self.selected.len() < 1 {
                    self.new_toast(
                        String::from("Clipboard"),
                        String::from("nothing is selected to be cut!"),
                        ToastKind::Info,
                        Duration::from_secs(3),
                    );
                    return;
                }

                self.new_toast(
                    String::from("Clipboard"),
                    format!("successfully cut {} items!", self.selected.len()),
                    ToastKind::Success,
                    Duration::from_secs(3),
                );
                self.add_to_clipboard(ClipboardMode::Cut);
            }
            KeybindAction::Paste => {
                if self.clipboard.entries.len() < 1 {
                    self.new_toast(
                        String::from("Clipboard"),
                        String::from("nothing is in clipboard to be pasted!"),
                        ToastKind::Info,
                        Duration::from_secs(3),
                    );
                    return;
                }

                self.new_toast(
                    String::from("Clipboard"),
                    format!("successfully pasted {} items!", self.selected.len()),
                    ToastKind::Success,
                    Duration::from_secs(3),
                );
                self.new_modal(ModalKind::Paste);
            }

            KeybindAction::Delete => {
                if self.selected.len() < 1 {
                    self.new_toast(
                        String::from("Delete"),
                        String::from("nothing is selected to be deleted!"),
                        ToastKind::Info,
                        Duration::from_secs(3),
                    );
                    return;
                }

                self.new_modal(ModalKind::Delete);
            }
            KeybindAction::Rename => {
                if self.current_index.is_none() {
                    self.new_toast(
                        String::from("Rename"),
                        String::from("nothing is selected to rename!"),
                        ToastKind::Info,
                        Duration::from_secs(3),
                    );
                    return;
                }

                self.new_modal(ModalKind::Rename);
            }

            KeybindAction::ClearClipboard => {
                self.clear_clipboard();
                self.new_toast(
                    String::from("Success!"),
                    String::from("successfully cleared clipboard!"),
                    ToastKind::Success,
                    Duration::from_secs(3),
                );
            }

            KeybindAction::ToggleHidden => {
                self.view_hidden = !self.view_hidden;
                self.fetch_entries(None);
            }
            KeybindAction::CreateFile => self.new_modal(ModalKind::CreateFile),
            KeybindAction::CreateFolder => self.new_modal(ModalKind::CreateFolder),
            KeybindAction::Info => {
                if self.current_index.is_none() {
                    self.new_toast(
                        String::from("Metadata"),
                        String::from("failed to open metadata modal (nothing is selected!)"),
                        ToastKind::Danger,
                        Duration::from_secs(3),
                    );
                    return;
                }

                self.new_modal(ModalKind::Metadata);
            }
            KeybindAction::Search => self.new_modal(ModalKind::Search),
            _ => {}
        }
    }

    fn fetch_entries(&mut self, prev_path: Option<PathBuf>) {
        self.modals.search = None;
        // clear entries
        self.entries.children.iter_mut().for_each(|e| {
            e.name.clear();
            e.file_type = "";
            e.path = PathBuf::new();
            e.using = false;
            e.accessed = None;
            e.created = None;
            e.folder_size = None;
        });

        let fetch_current_path = file_system::read_dir(&self.current_path);

        if let Err(err) = &fetch_current_path {
            self.new_toast(
                String::from("Error"),
                err.to_owned(),
                ToastKind::Danger,
                Duration::from_millis(5000),
            );
        }

        let mut index: usize = 0;

        for path in fetch_current_path.unwrap() {
            let (accessed, created) = file_system::accessed_and_created(&path, &true, &true);

            self.push_entry(
                &TempEntry {
                    name: path.file_name().unwrap().to_str().unwrap(),
                    file_type: &file_system::file_type(&path),
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
        let (file_size, file_type, accessed, created, name, is_hidden, path, folder_size) = (
            entry.file_size,
            entry.file_type,
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
            entry.file_type = file_type;

            entry.name.push_str(name);
            entry.path.push(path);
        } else {
            let mut entry = Entry {
                is_hidden,
                folder_size,
                file_size,
                accessed,
                created,
                file_type,
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

            if let Some(modal) = &self.modals.search
                && !entry.name.contains(&modal.content.trim())
            {
                continue;
            }

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
                    .par_iter()
                    .map(|&entry_index| {
                        (
                            entry_index,
                            self.entries.children[entry_index].name.to_lowercase(),
                        )
                    })
                    .collect();

                lowercased.par_sort_by(|a, b| a.1.cmp(&b.1));
                displaying
                    .par_iter_mut()
                    .zip(lowercased.par_iter())
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
            Property::Type => displaying.par_sort_by(|a, b| {
                let (x, y) = (&reference[*a].file_type, &reference[*b].file_type);
                x.cmp(y)
            }),
        }

        // check if reversed
    }

    fn nav_forward(&mut self) {
        let cur_index = &self.current_index;

        if cur_index.is_none() {
            return;
        }

        let cur_index = cur_index.unwrap();
        let to = &self.entries.entry(&cur_index).unwrap().path;

        if to.is_file() {
            let res = Command::new("xdg-open").arg(to).spawn();
            if let Err(err) = res {
                self.new_toast(
                    String::from("Error"),
                    err.to_string(),
                    ToastKind::Danger,
                    Duration::from_millis(5000),
                );
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
        let old_path = self.current_path.clone();
        self.current_path.pop();
        self.fetch_entries(Some(old_path));
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
                /*
                self.new_toast(
                    String::from("Renaming"),
                    String::from("Containing invalid characters"),
                    ToastKind::Danger,
                    Duration::from_millis(10000),
                );
                */
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
            ModalKind::Search => {
                self.modals.search = Some(SearchModal {
                    content: String::new(),
                    focused: true,
                });
                self.filter_entries(None);
            }
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
            ModalKind::Search => {
                self.modals.search.as_mut().unwrap().content = new_content;
                self.filter_entries(None);
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
            ModalKind::Search => self.modals.search = None,
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

    fn navigate_index(&mut self, direction: &NavigateDirection, is_ctrled: bool, is_shifted: bool) {
        let index_opt = self.current_index.as_mut();
        let mut current_index: usize = 0;

        if index_opt.is_none() {
            self.modify_selected(0, is_ctrled, is_shifted);
            return;
        } else if let Some(index) = index_opt {
            current_index = *index;
        }

        match direction {
            NavigateDirection::Down => {
                if current_index < self.entries.displaying.len() - 1 {
                    current_index += 1;
                }
            }
            NavigateDirection::Up => {
                if !(current_index == 0) {
                    current_index -= 1;
                }
            }
        }

        self.modify_selected(current_index, is_ctrled, is_shifted);
    }

    fn swap_selected(&mut self, index: &usize) {
        // exclusively for right clicking
        // - 1 selected: swapping
        // - >= 2 selected: add to the selected
        let entry_index_opt = self.entries.displaying.get(*index);
        let selected = &mut self.selected;

        if let Some(entry_index) = entry_index_opt
            && !selected.contains(entry_index)
        {
            if selected.len() == 1 {
                selected.clear();
            }
            selected.insert(*entry_index);
        }

        self.current_index = Some(*index);
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

    fn new_toast(&self, title: String, content: String, kind: ToastKind, duration: Duration) {
        let toasts = Arc::clone(&self.toasts);

        tokio::spawn(async move {
            let id = {
                let mut list = toasts.lock().unwrap();
                let toast = Toast {
                    title,
                    content,
                    kind,
                    duration,
                    ..Default::default()
                };
                let instant = toast.start_time.clone();
                list.push(toast);

                instant
            };

            tokio::time::sleep(duration).await;

            let mut list = toasts.lock().unwrap();
            list.retain(|t| t.start_time != id);
        });
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        let mut action_to_handle: Option<KeybindAction> = None;
        let (mut is_ctrled, mut is_shifted) = (false, false);
        ctx.clone().input_mut(|i| {
            let actions = self.actions.clone();
            is_ctrled = i.modifiers.ctrl;
            is_shifted = i.modifiers.shift;

            for event in &i.events {
                if let Event::Copy = event {
                    action_to_handle = Some(actions.copy);
                }
                if let Event::Cut = event {
                    action_to_handle = Some(actions.cut);
                }
                if let Event::Paste { .. } = event {
                    action_to_handle = Some(actions.paste);
                }
            }

            for (action, shortcut) in &self.config.keybinds_list {
                if i.consume_shortcut(shortcut) {
                    action_to_handle = Some(*action);
                }
            }
        });

        if let Some(action) = action_to_handle {
            self.handle_actions(&action, is_ctrled, is_shifted);
        }
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let mut i = 0;
        let ctx = self.ctx.clone();

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

            let mut pending_close_search = false;
            let mut pending_upd_search = None;

            if let Some(modal) = &mut self.modals.search {
                let mut content = modal.content.clone();
                let input = ui.add(
                    TextEdit::singleline(&mut content)
                        .background_color(Color32::TRANSPARENT)
                        .hint_text("input search entry :3")
                        .frame(Frame::NONE),
                );

                if input.changed() {
                    pending_upd_search = Some(content);
                }

                if input.lost_focus() {
                    modal.focused = false;
                }

                if ui.input(|i| i.key_pressed(Key::Escape)) {
                    pending_close_search = true;
                }

                if modal.focused {
                    input.request_focus();
                }
            }

            if pending_close_search {
                self.close_modal(ModalKind::Search);
            }
            if let Some(content) = pending_upd_search {
                self.update_modal(ModalKind::Search, content);
            }

            ui.heading("da buoyant file explorer!! :o");
            ui.separator();

            ui.horizontal(|ui| {
                ui.allocate_space(Vec2::new(2.0, 0.0));

                let mut grid = Grid::new(i);
                i += 1;
                grid = grid.min_col_width(200.0);

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
                    mut pending_nav,
                ) = (None, None, None, None, None, None, false);

                let keybinds = &self.config.keybinds;
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

                            grid = grid.min_col_width(200.0);
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

                        if is_current_index {
                            btn_response.scroll_to_me(None);
                        }

                        btn_response.context_menu(|ui| {
                            ui.label(entry.name.clone());
                            if ui
                                .add(
                                    Button::new("rename")
                                        .shortcut_text(ctx.format_shortcut(&keybinds.rename_file)),
                                )
                                .clicked()
                            {
                                pending_rename = Some(());
                            }
                            if ui
                                .add(Button::new("delete").shortcut_text(
                                    ctx.format_shortcut(&keybinds.delete_selections),
                                ))
                                .clicked()
                            {
                                pending_delete_modal = Some(());
                            }
                            if ui
                                .add(
                                    Button::new("cut").shortcut_text(
                                        ctx.format_shortcut(&keybinds.cut_to_clipboard),
                                    ),
                                )
                                .clicked()
                            {
                                pending_clipboard = Some(ClipboardMode::Cut);
                            }
                            if ui
                                .add(Button::new("copy").shortcut_text(
                                    ctx.format_shortcut(&keybinds.copy_to_clipboard),
                                ))
                                .clicked()
                            {
                                pending_clipboard = Some(ClipboardMode::Copy);
                            }
                            if ui
                                .add(
                                    Button::new("info")
                                        .shortcut_text(ctx.format_shortcut(&keybinds.view_info)),
                                )
                                .clicked()
                            {
                                pending_metadata_modal = Some(());
                            }
                        });

                        if btn_response.clicked() {
                            pending_modify_selected = Some(index);
                        }

                        if btn_response.double_clicked() {
                            pending_nav = true;
                        }

                        if btn_response.secondary_clicked() {
                            pending_swap_selected = Some(index);
                        }
                    });
                }

                if let Some(index) = pending_swap_selected {
                    self.swap_selected(&index);
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

                if pending_nav {
                    self.nav_forward();
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

            let mut pending_new_modal = None;
            let mut pending_add_to_cb = None;
            let mut pending_clear_cb = None;
            let mut pending_clear_selected = None;

            bg_response.inner.context_menu(|ui| {
                let keybinds = &self.config.keybinds;
                ui.label("create");

                if ui
                    .add(
                        Button::new("create file")
                            .shortcut_text(ctx.format_shortcut(&keybinds.create_file_path)),
                    )
                    .clicked()
                {
                    pending_new_modal = Some(ModalKind::CreateFile);
                }
                if ui
                    .add(
                        Button::new("create folder")
                            .shortcut_text(ctx.format_shortcut(&keybinds.create_folder_path)),
                    )
                    .clicked()
                {
                    pending_new_modal = Some(ModalKind::CreateFolder);
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
                    Button::new(del_label)
                        .stroke(Stroke::NONE)
                        .shortcut_text(ctx.format_shortcut(&keybinds.delete_selections)),
                    Button::new(cut_label)
                        .stroke(Stroke::NONE)
                        .shortcut_text(ctx.format_shortcut(&keybinds.cut_to_clipboard)),
                    Button::new(copy_label)
                        .stroke(Stroke::NONE)
                        .shortcut_text(ctx.format_shortcut(&keybinds.copy_to_clipboard)),
                    Button::new(clear_s_label).stroke(Stroke::NONE),
                );

                if self.selected.is_empty() {
                    del_button = del_button.sense(Sense::empty());
                    cut_button = cut_button.sense(Sense::empty());
                    copy_button = copy_button.sense(Sense::empty());
                    clear_s_button = clear_s_button.sense(Sense::empty());
                }

                if ui.add(del_button).clicked() {
                    pending_new_modal = Some(ModalKind::Delete);
                }
                if ui.add(cut_button).clicked() {
                    pending_add_to_cb = Some(ClipboardMode::Cut);
                }
                if ui.add(copy_button).clicked() {
                    pending_add_to_cb = Some(ClipboardMode::Copy);
                }
                if ui.add(clear_s_button).clicked() {
                    pending_clear_selected = Some(());
                }

                let (mut p_text, mut cp_text) =
                    (RichText::new("paste"), RichText::new("clear clipboard"));

                if self.clipboard.entries.is_empty() {
                    p_text = p_text.color(Color32::WHITE.gamma_multiply(0.5));
                    cp_text = cp_text.color(Color32::WHITE.gamma_multiply(0.5));
                }

                let (mut paste_button, mut clearcp_button) = (
                    Button::new(p_text)
                        .stroke(Stroke::NONE)
                        .shortcut_text(ctx.format_shortcut(&keybinds.paste_from_clipboard)),
                    Button::new(cp_text)
                        .stroke(Stroke::NONE)
                        .shortcut_text(ctx.format_shortcut(&keybinds.clear_clipboard)),
                );

                if self.clipboard.entries.is_empty() {
                    paste_button = paste_button.sense(Sense::empty());
                    clearcp_button = clearcp_button.sense(Sense::empty());
                }

                if ui.add(paste_button).clicked() {
                    self.new_modal(ModalKind::Paste);
                }
                if ui.add(clearcp_button).clicked() {
                    pending_clear_cb = Some(());
                }
            });

            if let Some(kind) = pending_new_modal {
                self.new_modal(kind);
            }

            if pending_clear_cb.is_some() {
                self.clear_clipboard();
            }

            if pending_clear_selected.is_some() {
                self.clear_selected();
            }

            if let Some(mode) = pending_add_to_cb {
                self.add_to_clipboard(mode);
            }
        });

        // modals

        if let Some(modal) = &self.modals.rename {
            let modal_widget = Modal::new(Id::new("rename_modal"));
            let mut content = modal.content.clone();
            let error = &modal.error.clone();

            modal_widget.show(&ctx, |ui| {
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

            modal_widget.show(&ctx, |ui| {
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

            modal_widget.show(&ctx, |ui| {
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

            modal_widget.show(&ctx, |ui| {
                ui.heading(format!(
                    "you are {} these:",
                    match self.clipboard.mode.as_ref().unwrap() {
                        ClipboardMode::Copy => "copying",
                        ClipboardMode::Cut => "cutting",
                    }
                ));
                let frame = Frame::NONE.fill(Color32::BLACK);
                frame.show(ui, |f| {
                    self.clipboard.entries.iter().for_each(|item| {
                        f.label(format!("{}", item.display()));
                    });
                });

                ui.separator();

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

            modal_widget.show(&ctx, |w| {
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
            pending_close_delete = true;
        }
        if pending_close_delete {
            self.close_modal(ModalKind::Delete);
        }

        let mut pending_close_metadata = false;

        if let Some(_modal) = &self.modals.metadata {
            let modal_widget = Modal::new(Id::new("metadata_modal"));
            let entry = &self.entries.entry(&self.current_index.unwrap()).unwrap();

            modal_widget.show(&ctx, |m| {
                m.label(format!("showing metadata for {}", entry.name));
                m.separator();
                m.label(format!("full path: {}", entry.path.display()));
                m.label(format!("type: {}", entry.file_type));
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

        let toasts_opt = self.toasts.clone();

        if let Ok(toast_list) = toasts_opt.try_lock()
            && toast_list.len() > 0
        {
            let toast_overlay = Window::new("toast")
                .title_bar(false)
                .frame(Frame::NONE)
                .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-8.0, -8.0))
                .resizable(false);

            toast_overlay.show(ui, |overlay| {
                for toast in toast_list.iter() {
                    let mut frame = Frame::new()
                        .corner_radius(4.0)
                        .stroke(Stroke::new(1.0, Color32::TRANSPARENT))
                        .fill(Color32::BLACK)
                        .inner_margin(4.0)
                        .outer_margin(4.0);

                    match toast.kind {
                        ToastKind::Info => frame.stroke.color = Color32::LIGHT_BLUE,
                        ToastKind::Danger => frame.stroke.color = Color32::LIGHT_RED,
                        ToastKind::Success => frame.stroke.color = Color32::LIGHT_GREEN,
                    }

                    frame.show(overlay, |fr| {
                        fr.vertical(|hor| {
                            let instant = Instant::now();
                            let delta = instant.duration_since(toast.start_time);

                            hor.label(toast.title.clone());
                            hor.label(toast.content.clone());
                            hor.add(
                                ProgressBar::new(delta.div_duration_f32(toast.duration))
                                    .corner_radius(2.0)
                                    .desired_height(6.0)
                                    .fill(Color32::WHITE.gamma_multiply(0.4)),
                            );
                        });
                    });
                }
            });
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
