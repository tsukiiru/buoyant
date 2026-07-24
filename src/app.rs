use chrono::{DateTime, Datelike, Utc};
use eframe::egui::{
    Align, Align2, Button, CentralPanel, Color32, Context, Event, Frame, Grid, Id, Image, Key,
    Label, Margin, Modal, ProgressBar, RichText, ScrollArea, Sense, Stroke, TextEdit, Theme, Ui,
    Vec2, Window, mutex::RwLock,
};
use egui_extras;
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
    sync::Arc,
    time::{Duration, Instant},
};
use tokio;

use crate::{
    config::{self, *},
    file_system::{self, BANNED_CHARACTERS},
    icons,
    types::*,
};

type Toasts = Arc<RwLock<Vec<Toast>>>;
pub struct App {
    scroll_signal: bool, // temporary solution for auto scrolling
    ctx: Context,
    current_path: PathBuf,
    entries: Entries,
    current_index: usize,
    selected: HashSet<usize>,
    clipboard: Clipboard,
    overlay: Overlay,
    field: Field,
    config: Config,
    actions: Actions,
    toasts: Toasts, // mhm toasts
}

impl Default for App {
    fn default() -> Self {
        App {
            scroll_signal: false,
            ctx: Context::default(),
            current_path: env::home_dir().unwrap(),
            overlay: Overlay::default(),
            field: Field::default(),
            entries: Entries::default(),
            clipboard: Clipboard::default(),
            selected: HashSet::with_capacity(20),
            current_index: 0,
            config: Config::default(),
            actions: Actions::default(),
            toasts: Arc::new(RwLock::new(Vec::with_capacity(5))),
        }
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = cc.egui_ctx.clone();
        egui_extras::install_image_loaders(&ctx);
        let mut app = App::default();
        app.ctx = ctx;
        app.fetch_config();
        app.bind_keybinds();

        app
    }

    fn toggle_view_hidden(&mut self) {
        self.config.view.view_hidden_files = !self.config.view.view_hidden_files;

        let selected_paths: Vec<PathBuf> = self
            .selected
            .par_iter()
            .map(|i| self.entries.children[*i].path.clone())
            .collect();

        self.filter_entries(Some(
            self.entries
                .entry(&self.current_index)
                .unwrap()
                .path
                .clone(),
        ));
        self.scroll_signal = true;
        for p in selected_paths {
            for (id, e) in self.entries.children.iter().enumerate() {
                if p == e.path
                    && let Some(i) = self.entries.displaying.iter().find(|i| **i == id)
                {
                    self.selected.insert(*i);
                }
            }
        }
    }

    fn fetch_config(&mut self) {
        config::fetch(&mut self.config);
        self.ctx.set_theme(if self.config.view.dark_mode {
            Theme::Dark
        } else {
            Theme::Light
        });
        self.fetch_entries(None);
    }

    fn bind_keybinds(&mut self) {
        let actions = &mut self.actions;

        for (action, sc) in &mut self.config.keybinds_list {
            if sc.modifiers.matches_logically(CTRL) && sc.logical_key == Key::C {
                actions.copy = action.clone();
            }
            if sc.modifiers.matches_logically(CTRL) && sc.logical_key == Key::X {
                actions.cut = action.clone();
            }
            if sc.modifiers.matches_logically(CTRL) && sc.logical_key == Key::V {
                actions.paste = action.clone();
            }
        }
    }

    fn handle_actions(&mut self, action: KeybindAction, is_ctrled: bool, is_shifted: bool) {
        if self.overlay.kind.is_some() {
            if let KeybindAction::Choice(choice) = action {
                self.finalize_overlay_choice(choice);
            }
            return;
        }

        if self.field.focused {
            return;
            // block input
        }

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

                self.open_overlay(OverlayKind::Paste);
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

                self.open_overlay(OverlayKind::Delete);
            }
            KeybindAction::Rename => self.open_overlay(OverlayKind::Rename),
            KeybindAction::ClearClipboard => {
                self.clear_clipboard();
                self.new_toast(
                    String::from("Success!"),
                    String::from("successfully cleared clipboard!"),
                    ToastKind::Success,
                    Duration::from_secs(3),
                );
            }
            KeybindAction::ToggleHidden => self.toggle_view_hidden(),
            KeybindAction::CreateFile => self.open_overlay(OverlayKind::CreateFile),
            KeybindAction::CreateFolder => self.open_overlay(OverlayKind::CreateFolder),
            KeybindAction::Info => self.open_overlay(OverlayKind::Metadata),
            KeybindAction::Search => self.new_field(FieldKind::Search),
            KeybindAction::Refresh => self.fetch_config(),
            _ => {}
        }
    }

    fn should_fetch(&self, property: Property) -> bool {
        let config = &self.config;
        config.view.explorer.contains(&property) || config.sorting.sorting_by == property
    }

    fn fetch_entries(&mut self, prev_path: Option<PathBuf>) {
        self.close_overlay();
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
            let (fetch_accessed, fetch_created) = (
                self.should_fetch(Property::Accessed),
                self.should_fetch(Property::Created),
            );
            let fetch_size = self.should_fetch(Property::Size);
            let (accessed, created) =
                file_system::accessed_and_created(&path, &fetch_accessed, &fetch_created);
            let (file_type, file_icon) = file_system::file_type(&path);

            self.push_entry(
                &TempEntry {
                    name: path.file_name().unwrap().to_str().unwrap(),
                    file_type: file_type,
                    file_icon: file_icon,
                    is_hidden: file_system::is_hidden(&path),
                    path: &path,
                    accessed,
                    created,
                    folder_size: file_system::folder_size(&path, &fetch_size),
                    file_size: file_system::file_size(&path, &fetch_size),
                },
                index,
            );
            index += 1;
        }

        self.entries.children.truncate(index);

        // freed some mem from the greedy alloc
        #[cfg(target_env = "gnu")]
        unsafe {
            unsafe extern "C" {
                fn malloc_trim(pad: usize) -> i32;
            }
            malloc_trim(0);
        }

        self.filter_entries(prev_path);
    }

    fn push_entry(&mut self, temp_entry: &TempEntry, index: usize) {
        if let Some(entry) = self.entries.children.get_mut(index) {
            entry.is_hidden = temp_entry.is_hidden;
            entry.using = true;
            entry.file_size = temp_entry.file_size;
            entry.accessed = temp_entry.accessed;
            entry.created = temp_entry.created;
            entry.folder_size = temp_entry.folder_size;
            entry.file_type = temp_entry.file_type;
            entry.file_icon = temp_entry.file_icon.clone();

            entry.name.push_str(temp_entry.name);
            entry.path.push(temp_entry.path);
        } else {
            let mut entry = Entry {
                is_hidden: temp_entry.is_hidden,
                folder_size: temp_entry.folder_size,
                file_size: temp_entry.file_size,
                accessed: temp_entry.accessed,
                created: temp_entry.created,
                file_type: temp_entry.file_type,
                file_icon: temp_entry.file_icon.clone(),
                using: true,
                ..Default::default()
            };
            entry.name.push_str(temp_entry.name);
            entry.path.push(temp_entry.path);

            self.entries.children.push(entry);
        }
    }

    fn filter_entries(&mut self, prev_path: Option<PathBuf>) {
        self.entries.displaying.clear();
        self.current_index = 0;
        self.selected.clear();

        let mut filter = "";
        let view_hidden = self.config.view.view_hidden_files;

        if let Some(kind) = &self.field.kind
            && kind == &FieldKind::Search
        {
            filter = self.field.buffer.trim();
        }

        for (i, entry) in self.entries.children.iter().enumerate() {
            if !entry.using || (!view_hidden && entry.is_hidden) || !entry.name.contains(&filter) {
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
                        self.current_index = index.clone();
                    }
                });
        }
    }

    fn sort(&mut self, index: usize, is_from_start: bool) {
        let reference = &self.entries.children;
        let displaying = if is_from_start {
            &mut self.entries.displaying[..index]
        } else {
            &mut self.entries.displaying[index..]
        };

        match &self.config.sorting.sorting_by {
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
            _ => {}
        }

        if self.config.sorting.reversed {
            displaying.reverse();
        }
    }

    fn nav_forward(&mut self) {
        let to = &self.entries.entry(&self.current_index).unwrap().path;

        if !to.is_file() && !to.is_dir() {
            return;
        }

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

        self.current_path = to.to_path_buf();
        self.fetch_entries(None);
    }

    fn nav_back(&mut self) {
        let old_path = self.current_path.clone();
        self.current_path.pop();
        self.fetch_entries(Some(old_path));
    }

    fn delete(&mut self) {
        if let Err(e) = file_system::delete(
            self.selected
                .par_iter()
                .map(|entry_index| {
                    self.entries
                        .children
                        .get(*entry_index)
                        .unwrap()
                        .path
                        .as_ref()
                })
                .collect::<Vec<&Path>>(),
        ) {
            self.new_toast(
                String::from("Delete"),
                e,
                ToastKind::Danger,
                Duration::from_secs(3),
            );
        }
        self.fetch_entries(None);
    }

    fn create(&mut self, mode: CreateType) {
        // true: file
        // false: folder

        let overlay = &mut self.overlay;
        let mut content = overlay.buffer.trim();
        if content.starts_with("/") {
            content = &content[1..];
        }

        let try_create = file_system::create(&self.current_path, Path::new(content), &mode);

        if let Err(error) = try_create {
            overlay.error.clear();
            overlay.error.push_str(&error);
            return;
        }

        self.close_overlay();
        self.fetch_entries(try_create.ok());
    }

    fn rename(&mut self) {
        let overlay = &mut self.overlay;
        if overlay.kind != Some(OverlayKind::Rename) {
            return;
        }

        for char in BANNED_CHARACTERS {
            if overlay.buffer.trim().contains(char) {
                overlay.error.clear();
                overlay.error.push_str("Containing invalid characters!");
                return;
            }
        }

        let rename_res = file_system::rename(&overlay.path.as_ref().unwrap(), &overlay.buffer);

        if let Err(err) = &rename_res {
            self.new_toast(
                String::from("Rename"),
                err.to_owned(),
                ToastKind::Danger,
                Duration::from_secs(3),
            );
        }

        self.fetch_entries(rename_res.ok());
        self.close_overlay();
    }

    fn new_field(&mut self, kind: FieldKind) {
        let field = &mut self.field;
        field.focused = true;

        if field.kind.is_some() {
            field.buffer = String::new();
            self.filter_entries(None);
            return;
        }

        field.kind = Some(kind.clone());
    }

    fn close_field(&mut self) {
        self.field.reset();
        self.filter_entries(None);
    }

    fn update_field_buffer(&mut self, buffer: String) {
        self.field.buffer = buffer;
    }

    fn logic_field(&mut self, kind: &FieldKind) {
        match kind {
            FieldKind::Search => self.filter_entries(None),
        }
    }

    fn open_overlay(&mut self, kind: OverlayKind) {
        let overlay = &mut self.overlay;
        overlay.kind = Some(kind);

        match kind {
            OverlayKind::Rename => {
                let path = &self.entries.entry(&self.current_index).unwrap().path;

                overlay.path = Some(path.to_path_buf());
                overlay.buffer = path.file_name().unwrap().to_str().unwrap().to_string();
            }
            OverlayKind::Metadata => {
                let metadata_conf = &self.config.view.metadata;
                let selected_entry = self.entries.entry(&self.current_index).unwrap();
                let mut new_entry = Entry::default();

                new_entry.path = selected_entry.path.clone();
                new_entry.name = selected_entry.name.clone();
                new_entry.file_type = selected_entry.file_type;

                if let Some(t) = selected_entry.accessed
                    && let Some(p) = selected_entry.created
                {
                    new_entry.accessed = Some(t);
                    new_entry.created = Some(p);
                } else {
                    let (acc, cr) = file_system::accessed_and_created(
                        &selected_entry.path,
                        &metadata_conf.contains(&Property::Accessed),
                        &metadata_conf.contains(&Property::Created),
                    );
                    new_entry.accessed = acc;
                    new_entry.created = cr;
                }

                if let Some(t) = selected_entry.file_size {
                    new_entry.file_size = Some(t);
                    new_entry.folder_size = selected_entry.folder_size;
                } else {
                    let fetch_size = &metadata_conf.contains(&Property::Size);
                    new_entry.file_size = file_system::file_size(&selected_entry.path, fetch_size);
                    new_entry.folder_size =
                        file_system::folder_size(&selected_entry.path, fetch_size);
                }

                overlay.entry = Some(new_entry);
            }
            _ => {}
        }
    }

    fn finalize_overlay_choice(&mut self, choice: usize) {
        match self.overlay.kind.unwrap() {
            OverlayKind::Paste => {
                if choice == 0 {
                    self.paste(PasteKind::Replace);
                } else if choice == 1 {
                    self.paste(PasteKind::Duplicate);
                }
            }
            OverlayKind::Delete => {
                if choice == 0 {
                    self.delete();
                } else if choice == 1 {
                    self.close_overlay();
                }
            }
            _ => {}
        }
    }

    fn update_overlay_buffer(&mut self, buffer: String) {
        self.overlay.buffer = buffer;
    }

    fn close_overlay(&mut self) {
        self.overlay.kind = None;
        self.overlay.error.clear();
        self.overlay.buffer = String::new();
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

        let amount = clipboard.entries.len();
        let returned_path = match clipboard_mode.unwrap() {
            ClipboardMode::Copy => {
                file_system::copy_dir(&clipboard.entries, &self.current_path, &paste_type)
            }
            ClipboardMode::Cut => {
                let paths =
                    file_system::move_dir(&clipboard.entries, &self.current_path, &paste_type);

                clipboard.entries.clear();
                clipboard.mode = None;

                paths
            }
        };

        self.close_overlay();
        self.new_toast(
            String::from("Clipboard"),
            format!("successfully pasted {} items!", amount),
            ToastKind::Success,
            Duration::from_secs(3),
        );
        self.fetch_entries(Some(returned_path[0].to_owned()));

        for p in returned_path {
            for (id, e) in self.entries.children.iter().enumerate() {
                if p == e.path
                    && let Some(i) = self.entries.displaying.iter().find(|i| **i == id)
                {
                    self.selected.insert(*i);
                }
            }
        }
    }

    fn navigate_index(&mut self, direction: &NavigateDirection, is_ctrled: bool, is_shifted: bool) {
        let mut current_index: usize = self.current_index;

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
        self.scroll_signal = true;
    }

    fn swap_selected(&mut self, index: &usize) {
        // - 1 selected: swapping
        // - >= 2 selected: add to the selected
        let selected = &mut self.selected;

        if let Some(entry_index) = self.entries.displaying.get(*index)
            && !selected.contains(entry_index)
        {
            if selected.len() == 1 {
                selected.clear();
            }
            selected.insert(*entry_index);
        }

        self.current_index = *index;
    }

    fn modify_selected(&mut self, index: usize, is_ctrled: bool, is_shifted: bool) {
        if !is_shifted && !is_ctrled {
            self.selected.clear();
        }

        let end_index = if is_shifted {
            self.current_index
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
                self.current_index = index;
                return;
            } else {
                self.selected.insert(*entry_index);
            }
        }

        self.current_index = index;
        self.selected.insert(*entry_index);
    }

    fn clear_selected(&mut self) {
        self.selected.clear();
    }

    fn new_toast(&self, title: String, content: String, kind: ToastKind, duration: Duration) {
        let toasts = Arc::clone(&self.toasts);

        tokio::spawn(async move {
            let id = {
                let mut list = toasts.write();
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

            let mut list = toasts.write();
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
            self.handle_actions(action, is_ctrled, is_shifted);
        }
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let mut i = 0;
        let ctx = self.ctx.clone();
        let visuals = ctx.theme().default_visuals();

        let (
            mut req_set_clipboard,
            mut req_reset_clipboard,
            mut req_swap_selected,
            mut req_modify_selected,
            mut req_reset_selected,
        ) = (None, false, None, None, false);

        let mut req_navigate_forward = false;
        let mut req_create = None;
        let mut req_rename = false;

        let (
            mut req_open_overlay,
            mut req_overlay_choice,
            mut req_close_overlay,
            mut req_update_overlay_buffer,
        ) = (None, None, false, None);

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

            {
                let mut content = self.field.buffer.clone();
                let bol = if let Some(kind) = self.field.kind
                    && kind == FieldKind::Search
                {
                    true
                } else {
                    false
                };

                let input = ui.add(
                    TextEdit::singleline(&mut content)
                        .background_color(Color32::TRANSPARENT)
                        .hint_text(format!(
                            "({}) input search entry :3",
                            ctx.format_shortcut(&self.config.keybinds.search)
                        ))
                        .frame(Frame::NONE)
                        .desired_width(f32::INFINITY),
                );

                if input.gained_focus() && !bol {
                    self.new_field(FieldKind::Search);
                }
                if input.changed() {
                    self.update_field_buffer(content);
                    self.logic_field(&FieldKind::Search);
                }
                if input.lost_focus() {
                    self.field.focused = false;
                }
                if self.field.kind.is_some() && ui.input(|i| i.key_pressed(Key::Escape)) {
                    self.close_field();
                }
                if self.field.focused {
                    input.request_focus();
                }
            }

            ui.separator();
            ui.horizontal(|ui| {
                let view = &self.config.view.explorer;
                ui.allocate_space(Vec2::new(2.0 + 16.0, 0.0));

                let mut grid = Grid::new(i);
                i += 1;
                grid = grid.min_col_width(200.0);

                grid.show(ui, |ui| {
                    view.iter().for_each(|p| {
                        ui.add(Label::new(p.to_string()).halign(Align::Min));
                    });
                });
            });

            let bg_response = ScrollArea::vertical().show(ui, |ui| {
                let bg_response = ui.interact(
                    ui.available_rect_before_wrap(),
                    Id::new(format!("explorer-area{}", i)),
                    Sense::click(),
                );
                i += 1;

                let keybinds = &self.config.keybinds;
                let current_index = &self.current_index;
                let len = self.entries.displaying.len();

                for (index, entry_index) in {
                    let start_index =
                        (*current_index as i16 - 30).clamp(0, len as i16 - 1) as usize;
                    let end_index = (*current_index as i16 + 30).clamp(0, len as i16 - 1) as usize;

                    (start_index..=end_index).into_iter().zip(
                        self.entries.displaying[start_index..=end_index]
                            .iter()
                            .clone(),
                    )
                } {
                    let is_current_index = index == *current_index;
                    let entry_opt = self.entries.children.get(*entry_index);
                    if entry_opt.is_none() {
                        continue;
                    }

                    let entry = entry_opt.unwrap();

                    ui.horizontal(|ui| {
                        let view = &self.config.view.explorer;
                        let mut frame = Frame::NONE
                            .stroke(Stroke::new(1.0, Color32::TRANSPARENT))
                            .corner_radius(4.0);
                        if self.selected.contains(&entry_index) {
                            frame.fill = Color32::LIGHT_GREEN.gamma_multiply(0.3);
                        }
                        if is_current_index {
                            frame.stroke.color = visuals.text_color().linear_multiply(0.3);
                        }

                        let fr = frame.show(ui, |f| {
                            let mut color = visuals.text_color();
                            if entry.is_hidden {
                                color = visuals.text_color().gamma_multiply(0.3);
                            }

                            let another_frame = Frame::NONE.inner_margin(Margin::symmetric(2, 8));
                            another_frame.show(f, |a| {
                                a.add(
                                    Image::new(icons::match_icon(&entry.file_icon))
                                        .fit_to_exact_size(Vec2::new(14.0, 14.0)),
                                );
                            });

                            let mut grid = Grid::new(Id::new(i));
                            i += 1;

                            grid = grid.min_col_width(200.0);
                            grid.show(f, |g| {
                                view.iter().for_each(|p| match p {
                                    Property::Name => {
                                        g.add(Label::new(RichText::new(&entry.name).color(color)));
                                    }
                                    Property::Accessed => {
                                        g.add(Label::new(
                                            RichText::new(format_date(entry.accessed)).color(color),
                                        ));
                                    }
                                    Property::Created => {
                                        g.add(Label::new(
                                            RichText::new(format_date(entry.created)).color(color),
                                        ));
                                    }
                                    Property::Size => {
                                        g.add(Label::new(
                                            RichText::new(if let Some(size) = &entry.folder_size {
                                                format!("{} items", size)
                                            } else {
                                                file_system::bytes_to_string(
                                                    entry.file_size.unwrap_or_default(),
                                                )
                                            })
                                            .color(color),
                                        ));
                                    }
                                    Property::Type => {
                                        g.add(Label::new(
                                            RichText::new(entry.file_type).color(color),
                                        ));
                                    }
                                    Property::Path => {
                                        g.add(Label::new(
                                            RichText::new(format!("{}", entry.path.display()))
                                                .color(color),
                                        ));
                                    }
                                });
                            });
                        });

                        let btn_response =
                            ui.interact(fr.response.rect, Id::new(i), Sense::click());
                        i += 1;

                        if is_current_index && self.scroll_signal {
                            btn_response.scroll_to_me(None);
                            self.scroll_signal = false;
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
                                req_open_overlay = Some(OverlayKind::Rename);
                            }
                            if ui
                                .add(Button::new("delete").shortcut_text(
                                    ctx.format_shortcut(&keybinds.delete_selections),
                                ))
                                .clicked()
                            {
                                req_open_overlay = Some(OverlayKind::Delete);
                            }
                            if ui
                                .add(
                                    Button::new("cut").shortcut_text(
                                        ctx.format_shortcut(&keybinds.cut_to_clipboard),
                                    ),
                                )
                                .clicked()
                            {
                                req_set_clipboard = Some(ClipboardMode::Cut);
                            }
                            if ui
                                .add(Button::new("copy").shortcut_text(
                                    ctx.format_shortcut(&keybinds.copy_to_clipboard),
                                ))
                                .clicked()
                            {
                                req_set_clipboard = Some(ClipboardMode::Copy);
                            }
                            if ui
                                .add(
                                    Button::new("info")
                                        .shortcut_text(ctx.format_shortcut(&keybinds.view_info)),
                                )
                                .clicked()
                            {
                                req_open_overlay = Some(OverlayKind::Metadata);
                            }
                        });

                        if btn_response.clicked() {
                            req_modify_selected = Some(index);
                        }

                        if btn_response.double_clicked() {
                            req_navigate_forward = true;
                        }

                        if btn_response.secondary_clicked() {
                            req_swap_selected = Some(index);
                        }
                    });
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
                let keybinds = &self.config.keybinds;
                ui.label("create");

                if ui
                    .add(
                        Button::new("create file")
                            .shortcut_text(ctx.format_shortcut(&keybinds.create_file_path)),
                    )
                    .clicked()
                {
                    req_open_overlay = Some(OverlayKind::CreateFile);
                }
                if ui
                    .add(
                        Button::new("create folder")
                            .shortcut_text(ctx.format_shortcut(&keybinds.create_folder_path)),
                    )
                    .clicked()
                {
                    req_open_overlay = Some(OverlayKind::CreateFolder);
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
                    del_label = del_label.color(visuals.text_color().gamma_multiply(0.5));
                    cut_label = cut_label.color(visuals.text_color().gamma_multiply(0.5));
                    copy_label = copy_label.color(visuals.text_color().gamma_multiply(0.5));
                    clear_s_label = clear_s_label.color(visuals.text_color().gamma_multiply(0.5));
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
                    req_open_overlay = Some(OverlayKind::Delete);
                }
                if ui.add(cut_button).clicked() {
                    req_set_clipboard = Some(ClipboardMode::Cut);
                }
                if ui.add(copy_button).clicked() {
                    req_set_clipboard = Some(ClipboardMode::Copy);
                }
                if ui.add(clear_s_button).clicked() {
                    req_reset_selected = true;
                }

                let (mut p_text, mut cp_text) =
                    (RichText::new("paste"), RichText::new("clear clipboard"));

                if self.clipboard.entries.is_empty() {
                    p_text = p_text.color(visuals.text_color().gamma_multiply(0.5));
                    cp_text = cp_text.color(visuals.text_color().gamma_multiply(0.5));
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
                    self.open_overlay(OverlayKind::Paste);
                }
                if ui.add(clearcp_button).clicked() {
                    req_reset_clipboard = true;
                }
            });
        });

        // modals

        let overlay = &self.overlay;
        let overlay_kind = overlay.kind;
        if let Some(kind) = overlay_kind
            && kind == OverlayKind::Rename
        {
            let modal_widget = Modal::new(Id::new("rename_modal"));
            let mut content = overlay.buffer.clone();
            let error = &overlay.error.clone();

            modal_widget.show(&ctx, |ui| {
                ui.heading("renaming");
                let input = ui.add(TextEdit::singleline(&mut content));
                ui.add(Label::new(RichText::new(error).color(Color32::LIGHT_RED)));

                if input.changed() {
                    req_update_overlay_buffer = Some(content);
                }

                if input.lost_focus() {
                    if ui.input(|i| i.key_pressed(Key::Enter)) {
                        req_rename = true;
                    } else {
                        req_close_overlay = true;
                    }
                }

                input.request_focus();
            });
        }

        if let Some(kind) = overlay_kind
            && kind == OverlayKind::CreateFile
        {
            let modal_widget = Modal::new(Id::new("create_file_modal"));
            let mut content = overlay.buffer.clone();
            let error = &overlay.error.clone();

            modal_widget.show(&ctx, |ui| {
                ui.label(format!("creating file at {}", self.current_path.display()));
                let input = ui.add(TextEdit::singleline(&mut content));
                ui.add(Label::new(RichText::new(error).color(Color32::LIGHT_RED)));

                if input.changed() {
                    req_update_overlay_buffer = Some(content);
                }

                if input.lost_focus() {
                    if ui.input(|i| i.key_pressed(Key::Enter)) {
                        req_create = Some(CreateType::File);
                    } else {
                        req_close_overlay = true;
                    }
                }

                input.request_focus();
            });
        }

        if let Some(kind) = overlay_kind
            && kind == OverlayKind::CreateFolder
        {
            let modal_widget = Modal::new(Id::new("create_folder_modal"));
            let mut content = overlay.buffer.clone();
            let error = &overlay.error.clone();

            modal_widget.show(&ctx, |ui| {
                ui.label(format!(
                    "creating folder at {}",
                    self.current_path.display()
                ));
                let input = ui.add(TextEdit::singleline(&mut content));
                ui.add(Label::new(RichText::new(error).color(Color32::LIGHT_RED)));

                if input.changed() {
                    req_update_overlay_buffer = Some(content);
                }

                if input.lost_focus() {
                    if ui.input(|i| i.key_pressed(Key::Enter)) {
                        req_create = Some(CreateType::Folder);
                    } else {
                        req_close_overlay = true;
                    }
                }

                input.request_focus();
            });
        }

        if let Some(kind) = overlay_kind
            && kind == OverlayKind::Paste
        {
            let modal_widget = Modal::new(Id::new("paste_modal"));

            modal_widget.show(&ctx, |ui| {
                let keybinds = &self.config.keybinds;
                ui.heading(format!(
                    "you are {} these:",
                    match self.clipboard.mode.as_ref().unwrap() {
                        ClipboardMode::Copy => "copying",
                        ClipboardMode::Cut => "cutting",
                    }
                ));
                let frame = Frame::NONE.fill(visuals.text_edit_bg_color());
                frame.show(ui, |f| {
                    self.clipboard.entries.iter().for_each(|item| {
                        f.label(format!("{}", item.display()));
                    });
                });
                ui.separator();
                ui.heading("choose pasting type");
                ui.vertical(|ui| {
                    if ui
                        .add(
                            Button::new("replace")
                                .shortcut_text(ctx.format_shortcut(&keybinds.choice_0)),
                        )
                        .clicked()
                    {
                        req_overlay_choice = Some(0);
                    }
                    ui.label("replace if file(s) with the same name already existed");
                });
                ui.vertical(|ui| {
                    if ui
                        .add(
                            Button::new("duplicate")
                                .shortcut_text(ctx.format_shortcut(&keybinds.choice_1)),
                        )
                        .clicked()
                    {
                        req_overlay_choice = Some(1);
                    }
                    ui.label("make a duplicate if file(s) with the same name already existed");
                });

                if ui.input(|i| i.key_pressed(Key::Escape)) {
                    self.close_overlay();
                }
            });
        };

        if let Some(kind) = overlay_kind
            && kind == OverlayKind::Delete
        {
            let keybinds = &self.config.keybinds;
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
                    .fill(visuals.text_edit_bg_color().gamma_multiply(0.7))
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
                    if u.add(
                        Button::new("yeah").shortcut_text(ctx.format_shortcut(&keybinds.choice_0)),
                    )
                    .clicked()
                    {
                        req_overlay_choice = Some(0);
                    }
                    if u.add(
                        Button::new("no").shortcut_text(ctx.format_shortcut(&keybinds.choice_1)),
                    )
                    .clicked()
                        || u.input(|i| i.key_pressed(Key::Escape))
                    {
                        req_overlay_choice = Some(1);
                    }
                })
            });
        }

        if let Some(kind) = overlay_kind
            && kind == OverlayKind::Metadata
        {
            let modal_widget = Modal::new(Id::new("metadata_modal"));
            let entry = self.overlay.entry.as_ref().unwrap();

            modal_widget.show(&ctx, |m| {
                m.label(format!("showing metadata for {}", entry.name));
                m.separator();

                self.config.view.metadata.iter().for_each(|p| match p {
                    Property::Path => {
                        m.label(format!("full path: {}", entry.path.display()));
                    }
                    Property::Type => {
                        m.label(format!("type: {}", entry.file_type));
                    }
                    Property::Accessed => {
                        m.label(format!(
                            "last accessed date: {}",
                            DateTime::from_timestamp_secs(entry.accessed.unwrap_or_default())
                                .unwrap()
                                .format(&self.config.view.format_date)
                        ));
                    }
                    Property::Created => {
                        m.label(format!(
                            "created date: {}",
                            DateTime::from_timestamp_secs(entry.created.unwrap_or_default())
                                .unwrap()
                                .format(&self.config.view.format_date)
                        ));
                    }
                    Property::Size => {
                        m.label(if let Some(size) = &entry.folder_size {
                            format!("folder size: {} items", size)
                        } else {
                            format!(
                                "file size: {}",
                                file_system::bytes_to_string(entry.file_size.unwrap_or_default())
                            )
                        });
                    }
                    _ => {}
                });
                if m.input(|i| i.key_pressed(Key::Escape)) {
                    req_close_overlay = true;
                }
            });
        }

        if req_close_overlay {
            self.close_overlay();
        }

        if let Some(kind) = req_open_overlay {
            self.open_overlay(kind);
        }

        if req_reset_clipboard {
            self.clear_clipboard();
        }

        if req_reset_selected {
            self.clear_selected();
        }

        if let Some(mode) = req_set_clipboard {
            self.add_to_clipboard(mode);
        }

        if let Some(choice) = req_overlay_choice {
            self.finalize_overlay_choice(choice);
        }

        if let Some(index) = req_swap_selected {
            self.swap_selected(&index);
        }

        if let Some(content) = req_update_overlay_buffer {
            self.update_overlay_buffer(content);
        }

        if let Some(kind) = req_create {
            self.create(kind);
        }

        if let Some(index) = req_modify_selected {
            let ctrl_pressed =
                ui.input(|i| i.key_down(Key::ControlLeft) || i.key_down(Key::ControlRight));
            let shift_pressed =
                ui.input(|i| i.key_down(Key::ShiftLeft) || i.key_down(Key::ShiftRight));

            self.modify_selected(index, ctrl_pressed, shift_pressed);
        }

        if req_rename {
            self.rename();
        }

        if req_navigate_forward {
            self.nav_forward();
        }

        let toast_list = self.toasts.read();
        if toast_list.len() > 0 {
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
                        .fill(visuals.text_edit_bg_color())
                        .inner_margin(4.0)
                        .outer_margin(4.0);

                    match toast.kind {
                        ToastKind::Info => frame.stroke.color = Color32::LIGHT_BLUE,
                        ToastKind::Danger => frame.stroke.color = Color32::LIGHT_RED,
                        ToastKind::Success => frame.stroke.color = Color32::LIGHT_GREEN,
                    }

                    frame.show(overlay, |fr| {
                        fr.vertical(|hor| {
                            hor.label(toast.title.clone());
                            hor.label(toast.content.clone());
                            hor.add(
                                ProgressBar::new(
                                    Instant::now()
                                        .duration_since(toast.start_time)
                                        .div_duration_f32(toast.duration),
                                )
                                .corner_radius(2.0)
                                .desired_height(6.0)
                                .fill(visuals.text_color().gamma_multiply(0.4)),
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
    let delta_day = current_date.sub(given_date).num_hours() / 24;

    // today
    if delta_day < 1 && current_date.day() == given_date.day() {
        format!("Today, {}", given_date.format("%I:%M %p"))
    }
    // yesterday
    else if delta_day < 2 {
        format!("Yesterday, {}", given_date.format("%I:%M %p"))
    }
    // this week
    else if delta_day <= 7 {
        format!("{} days ago", delta_day)
    }
    // last week
    else if delta_day <= 14 {
        String::from("Last week")
    }
    // this month
    else if delta_day <= 31 {
        format!("{} weeks ago", delta_day / 7)
    }
    // last month
    else if delta_day <= 62 {
        String::from("Last month")
    }
    // this year
    else if delta_day <= 365 {
        format!("{} months ago", delta_day / 31)
    }
    // last year
    else if delta_day <= 730 {
        String::from("Last year")
    }
    // blah blah blah
    else {
        format!("{} years ago", delta_day / 365)
    }
}
