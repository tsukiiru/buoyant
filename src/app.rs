use std::{
    collections::{HashMap, HashSet},
    env::home_dir,
    ops::{Deref, Sub},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use chrono::{DateTime, Datelike, Utc};
use rayon::prelude::*;

use iced::widget::{
    operation::AbsoluteOffset, scrollable::Viewport, selector::Target, text::Wrapping,
};
use iced::{
    Background, Border, Color, Element, Event, Length, Padding, Subscription, Task, alignment,
    border::Radius,
    event::{self, Status},
    keyboard::{
        self, Modifiers,
        key::{self, Code, Physical},
    },
    widget::{
        button, column, container, float, mouse_area, opaque, operation, row, scrollable, selector,
        stack, svg, text, text_input,
    },
};

use crate::path;
use crate::theme::Theme;
use crate::{
    config::{self, Displaying, SortingBy},
    types::SearchModal,
};
use crate::{
    theme,
    types::{
        Clipboard, ClipboardMode, CreateModal, Direction, Entries, Item, ModalMessage, ModalType,
        PasteType, RenameModal, TempItem,
    },
};

struct States {
    modifiers: ModifiersState,
    modals: ModalsState,
    explorer: ExplorerState,
    is_visual_mode: bool,
    is_loading: bool,
}

impl Default for States {
    fn default() -> Self {
        States {
            modifiers: ModifiersState::default(),
            modals: ModalsState::default(),
            explorer: ExplorerState::default(),
            is_visual_mode: false,
            is_loading: false,
        }
    }
}

struct ModalsState {
    opened: bool,
    paste: bool,
    delete: bool,
    create_file: Option<CreateModal>,
    create_folder: Option<CreateModal>,
    rename: Option<RenameModal>,
    search: Option<SearchModal>,

    choices: Vec<Message>,
    current_choice: usize,
}

impl Default for ModalsState {
    fn default() -> Self {
        ModalsState {
            opened: false,
            paste: false,
            delete: false,
            create_file: None,
            create_folder: None,
            rename: None,
            search: None,
            choices: Vec::with_capacity(2),
            current_choice: 0,
        }
    }
}

struct ExplorerState {
    offset: f32,
    error: Option<String>,
}

impl Default for ExplorerState {
    fn default() -> Self {
        ExplorerState {
            offset: 0.0,
            error: None,
        }
    }
}

struct ModifiersState {
    ctrl: bool,
    shift: bool,
    alt: bool,
}

impl Default for ModifiersState {
    fn default() -> Self {
        ModifiersState {
            ctrl: false,
            shift: false,
            alt: false,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    // navigation
    Open(Option<usize>),
    FetchEntries(Option<PathBuf>),
    FilterEntries(Option<PathBuf>),
    NavigateBack,
    NavigateTo(Direction),
    ExplorerScroll(Option<Target>),
    ExplorerOffset(Viewport),

    // selection
    Select(usize),
    ResetSelection,
    HoverEntry(usize, bool),

    // display
    ToggleHiddenView,
    ToggleVisualMode,

    // clipboard
    AddClipboard(ClipboardMode),
    PasteClipboard(PasteType),

    // file operations
    Rename,
    Delete,
    Create(bool),

    // input
    HandleEvent(Physical, Modifiers, Status),
    KeyModifiers(bool, bool, bool),

    // modals, modal choices navigation
    Modal(ModalType, ModalMessage),
    FocusModal,
    CloseModals,
    ClearChoices,
    SelectChoice,
    ChoiceIndex(bool),

    // app
    FetchConfig,
}

pub struct Buoyant {
    config: config::Config,
    theme: Theme,

    current_path: PathBuf,
    current_index: Option<usize>,

    entries: Entries,
    selected: HashSet<usize>,
    clipboard: Clipboard,

    states: States,
}

impl Buoyant {
    pub fn new(input: &str) -> (Self, Task<Message>) {
        let path_conversion = PathBuf::from(input);
        let path: PathBuf;

        if !path_conversion.exists() {
            let home_directory = home_dir();

            if let Some(dir) = home_directory {
                path = dir;
            } else {
                path = PathBuf::from("/");
            }
        } else {
            path = path_conversion;
        }

        (
            Buoyant {
                config: config::Config::default(),
                theme: theme::Theme::default(),

                current_path: path,
                current_index: None,

                entries: Entries::new(),
                clipboard: Clipboard::default(),
                selected: HashSet::with_capacity(5),

                states: States::default(),
            },
            Task::done(Message::FetchConfig).chain(Task::done(Message::FetchEntries(None))),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FetchConfig => {
                self.states.is_loading = true;
                config::fetch(&mut self.config);
                self.theme = theme::fetch(self.config.misc.theme_path.as_deref());
                self.states.is_loading = false;
                Task::none()
            }

            Message::Open(index) => {
                if index.is_none() {
                    return Task::none();
                }

                let path = &self.entries.item(&index.unwrap()).unwrap().path;

                if path.is_dir() {
                    self.current_path = path.to_owned();
                    self.current_index = None;
                    Task::done(Message::FetchEntries(None))
                } else {
                    let cmd = Command::new("xdg-open")
                        .arg(path)
                        .stderr(Stdio::null())
                        .spawn();

                    if let Err(e) = cmd {
                        println!("{}", e);
                    }

                    Task::none()
                }
            }
            Message::NavigateBack => {
                if self.states.modals.opened {
                    return Task::none();
                }

                let path = Some(self.current_path.clone());
                self.current_path.pop();

                Task::done(Message::FetchEntries(path)).chain(
                    selector::find(selector::id("scrollable"))
                        .then(|output| Task::done(Message::ExplorerScroll(output))),
                )
            }

            Message::HandleEvent(physical_key, modifiers, status) => {
                if status == Status::Captured && self.states.modals.opened {
                    return Task::done(Message::FocusModal);
                }

                if physical_key == Physical::Code(Code::Escape)
                    && (self.states.modals.search.is_some() || self.states.modals.opened)
                {
                    return Task::done(Message::CloseModals);
                }

                if let Some(modal) = &self.states.modals.search
                    && modal.focused
                {
                    return Task::none();
                }

                let keybinds = &self.config.keybinds;

                if physical_key == keybinds.navigate_backward.key
                    && modifiers == keybinds.navigate_backward.modifiers
                {
                    return Task::done(Message::ChoiceIndex(false))
                        .chain(Task::done(Message::NavigateBack));
                } else if physical_key == keybinds.navigate_forward.key
                    && modifiers == keybinds.navigate_forward.modifiers
                {
                    return Task::done(Message::ChoiceIndex(true))
                        .chain(Task::done(Message::Open(self.current_index)));
                } else if physical_key == keybinds.navigate_down.key
                    && modifiers == keybinds.navigate_down.modifiers
                {
                    return Task::done(Message::NavigateTo(Direction::Down));
                } else if physical_key == keybinds.navigate_up.key
                    && modifiers == keybinds.navigate_up.modifiers
                {
                    return Task::done(Message::NavigateTo(Direction::Up));
                } else if physical_key == keybinds.copy_to_clipboard.key
                    && modifiers == keybinds.copy_to_clipboard.modifiers
                {
                    return Task::done(Message::AddClipboard(ClipboardMode::Copy));
                } else if physical_key == keybinds.cut_to_clipboard.key
                    && modifiers == keybinds.cut_to_clipboard.modifiers
                {
                    return Task::done(Message::AddClipboard(ClipboardMode::Cut));
                } else if physical_key == keybinds.paste_from_clipboard.key
                    && modifiers == keybinds.paste_from_clipboard.modifiers
                {
                    return Task::done(Message::Modal(ModalType::Paste, ModalMessage::Open));
                } else if physical_key == keybinds.clear_clipboard.key
                    && modifiers == keybinds.clear_clipboard.modifiers
                {
                    self.clipboard.entries.clear();
                    self.clipboard.mode = None;
                    return Task::none();
                } else if physical_key == keybinds.delete_selections.key
                    && modifiers == keybinds.delete_selections.modifiers
                {
                    return Task::done(Message::Modal(ModalType::Delete, ModalMessage::Open));
                } else if physical_key == keybinds.rename_file.key
                    && modifiers == keybinds.rename_file.modifiers
                {
                    return Task::done(Message::Modal(ModalType::Rename, ModalMessage::Open));
                } else if physical_key == keybinds.toggle_hidden_view.key
                    && modifiers == keybinds.toggle_hidden_view.modifiers
                {
                    return Task::done(Message::ToggleHiddenView);
                } else if physical_key == keybinds.create_file_path.key
                    && modifiers == keybinds.create_file_path.modifiers
                {
                    return Task::done(Message::Modal(ModalType::CreateFile, ModalMessage::Open));
                } else if physical_key == keybinds.create_folder_path.key
                    && modifiers == keybinds.create_folder_path.modifiers
                {
                    return Task::done(Message::Modal(ModalType::CreateFolder, ModalMessage::Open));
                } else if physical_key == keybinds.toggle_visual_mode.key
                    && modifiers == keybinds.toggle_visual_mode.modifiers
                {
                    return Task::done(Message::ToggleVisualMode);
                } else if physical_key == keybinds.refresh.key
                    && modifiers == keybinds.refresh.modifiers
                {
                    return Task::done(Message::FetchConfig)
                        .chain(Task::done(Message::FetchEntries(None)));
                } else if physical_key == keybinds.search.key
                    && modifiers == keybinds.search.modifiers
                {
                    return Task::done(Message::Modal(ModalType::Search, ModalMessage::Open));
                }

                Task::none()
            }

            Message::FetchEntries(prev_path) => {
                self.states.modals.search = None;
                self.states.explorer.error = None;
                // clear all entries, without reallocating
                self.entries.children.par_iter_mut().for_each(|item| {
                    item.using = false;
                    item.name.clear();
                    item.path.clear();
                    item.accessed = 0;
                    item.created = 0;
                    item.foldersize = None;
                    item.filetype.clear();
                });

                let cur_paths_opt = path::read_dir(&self.current_path);

                if let Err(error) = cur_paths_opt {
                    self.states.explorer.error = Some(error);
                    return Task::none();
                }

                let current_paths = cur_paths_opt.unwrap();
                let mut index: usize = 0;

                for path in current_paths {
                    let (file_type, icon) = &path::file_type(&path);

                    self.push_entry(
                        &TempItem {
                            filetype: &file_type,
                            icon: &icon,
                            accessed: path::file_accessed(&path),
                            created: path::file_created(&path),
                            filesize: path::file_size(&path),
                            foldersize: path::folder_size(&path),
                            hidden: path::is_hidden(&path),
                            name: path.file_name().unwrap().to_str().unwrap(),

                            path: &path,
                        },
                        index,
                    );

                    index += 1;
                }

                Task::done(Message::FilterEntries(prev_path))
            }
            Message::FilterEntries(prev_path) => {
                self.entries.displaying.clear();
                self.current_index = None;
                self.selected.clear();

                for (i, entry) in self.entries.children.iter().enumerate() {
                    if !entry.using || (!self.config.view_hidden && entry.hidden) {
                        continue;
                    }
                    if let Some(modal) = &self.states.modals.search
                        && !entry.name.contains(&modal.content.trim())
                    {
                        continue;
                    }

                    self.entries.displaying.push(i);
                }

                self.entries.displaying.par_sort_by(|a, b| {
                    let (x, y) = (
                        &self.entries.children[*a].hidden,
                        &self.entries.children[*b].hidden,
                    );
                    y.cmp(x)
                });

                let mut last_hidden_index: usize = 0;

                for (index, entry_index) in self.entries.displaying.iter().enumerate() {
                    if !self.entries.children[*entry_index].hidden {
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

                Task::none()
            }

            Message::HoverEntry(index, state) => {
                let item = self.entries.children.get_mut(index).unwrap();
                item.hovered = state;

                Task::none()
            }

            Message::ToggleHiddenView => {
                self.config.view_hidden = !self.config.view_hidden;
                Task::done(Message::FilterEntries(None))
            }
            Message::ToggleVisualMode => {
                self.states.is_visual_mode = !self.states.is_visual_mode;
                Task::none()
            }

            Message::KeyModifiers(ctrl_state, shift_state, alt_state) => {
                let modifiers_state = &mut self.states.modifiers;

                modifiers_state.ctrl = ctrl_state;
                modifiers_state.shift = shift_state;
                modifiers_state.alt = alt_state;

                Task::none()
            }

            Message::ExplorerScroll(target) => {
                let cur_index_opt = self.current_index;

                if cur_index_opt.is_none() {
                    return Task::none();
                }

                let current_index: f32 = cur_index_opt.unwrap() as f32 + 1.0;
                let offset: f32 = 40.0 * (current_index - 1.0);

                let height = target.unwrap().visible_bounds().unwrap().height;
                let widget_range = (
                    self.states.explorer.offset,
                    self.states.explorer.offset + height - 10.0,
                );

                if offset <= widget_range.0 {
                    return operation::scroll_to(
                        "scrollable",
                        AbsoluteOffset { x: 0.0, y: offset },
                    );
                }

                // 40 is the height of the button

                if widget_range.1 <= offset {
                    return operation::scroll_to(
                        "scrollable",
                        AbsoluteOffset {
                            x: 0.0,
                            y: offset - height + 40.0,
                        },
                    );
                }

                Task::none()
            }
            Message::ExplorerOffset(viewport) => {
                self.states.explorer.offset = viewport.absolute_offset().y;
                Task::none()
            }

            Message::Select(index) => {
                let states = &self.states;

                if !states.modifiers.shift && !states.is_visual_mode && !states.modifiers.ctrl {
                    self.selected.clear();
                }

                let end_index = if let Some(current_index) = self.current_index
                    && (states.modifiers.shift || states.is_visual_mode)
                {
                    current_index
                } else {
                    index
                };

                for i in index.min(end_index)..=end_index.max(index) {
                    self.selected.insert(i);
                } // selecting everything between the two indicies

                if states.modifiers.ctrl {
                    if self.selected.contains(&index) {
                        self.selected.remove(&index);
                    } else {
                        self.selected.insert(index);
                    }
                }

                self.current_index = Some(index);

                selector::find(selector::id("scrollable"))
                    .then(|output| Task::done(Message::ExplorerScroll(output)))
            }
            Message::ResetSelection => {
                let states = &self.states;

                if !states.modifiers.ctrl || !states.is_visual_mode {
                    self.selected.clear();
                }
                Task::none()
            }
            Message::Delete => {
                for index in &self.selected {
                    let item_opt = self.entries.item(index);

                    if let Some(item) = item_opt {
                        path::delete(&item.path);
                    }
                }

                Task::done(Message::Modal(ModalType::Delete, ModalMessage::Close))
                    .chain(Task::done(Message::FetchEntries(None)))
            }
            Message::NavigateTo(direction) => {
                if self.states.modals.opened {
                    return Task::none();
                }

                let index_opt = self.current_index.as_mut();
                let mut current_index: usize = 0;

                if index_opt.is_none() {
                    return Task::done(Message::Select(0));
                } else if let Some(index) = index_opt {
                    current_index = *index;
                }

                match direction {
                    Direction::Down => {
                        if current_index < self.entries.displaying.len() - 1 {
                            current_index += 1;
                        }
                    }
                    Direction::Up => {
                        if !(current_index == 0) {
                            current_index -= 1;
                        }
                    }
                }

                Task::done(Message::Select(current_index))
            }

            Message::AddClipboard(mode) => {
                if self.states.modals.opened || self.selected.is_empty() {
                    return Task::none();
                }

                let clipboard = &mut self.clipboard;

                clipboard.entries.clear();

                self.selected.iter().for_each(|i| {
                    let _ = clipboard
                        .entries
                        .insert(self.entries.item(&i).unwrap().path.clone());
                });

                clipboard.mode = Some(mode);

                Task::none()
            }
            Message::PasteClipboard(opp) => {
                let clipboard = &mut self.clipboard;
                let clipboard_mode = clipboard.mode.as_ref();

                if clipboard.entries.is_empty() || clipboard_mode.is_none() {
                    return Task::none();
                }

                let mode = clipboard_mode.unwrap();

                match mode {
                    ClipboardMode::Copy => {
                        path::copy_dir(&clipboard.entries, &self.current_path, &opp);
                    }
                    ClipboardMode::Cut => {
                        path::move_dir(&clipboard.entries, &self.current_path, &opp);

                        clipboard.entries.clear();
                        clipboard.mode = None;
                    }
                }

                Task::done(Message::Modal(ModalType::Paste, ModalMessage::Close))
                    .chain(Task::done(Message::FetchEntries(None)))
            }

            Message::Rename => {
                let overlay = self.states.modals.rename.as_mut().unwrap();
                let name = &overlay.content;

                if name.is_empty() {
                    return Task::none();
                }

                // checking if the new name is valid?
                for char in path::NONO_CHARACTERS {
                    if name.contains(char) {
                        overlay.error = "name cannot contain invalid characters!";
                        return Task::none();
                    }
                }

                let mut test_path = overlay.path.clone();
                test_path.set_file_name(name);

                // check if already exists in destination
                if test_path.exists() {
                    overlay.error = "ERROR: file with the same name already exists";
                    return Task::none();
                }

                path::rename(&mut overlay.path, name);
                Task::done(Message::Modal(ModalType::Rename, ModalMessage::Close))
                    .chain(Task::done(Message::FetchEntries(None)))
            }
            Message::Create(mode) => {
                // true if creating file, else creating folder
                if mode {
                    let overlay = self.states.modals.create_file.as_mut().unwrap();

                    let try_create =
                        path::create(&self.current_path, Path::new(overlay.content.trim()), true);

                    if let Some(error) = try_create {
                        overlay.error = error;
                    } else {
                        return Task::done(Message::Modal(
                            ModalType::CreateFile,
                            ModalMessage::Close,
                        ))
                        .chain(Task::done(Message::FetchEntries(None)));
                    }
                } else {
                    let overlay = self.states.modals.create_folder.as_mut().unwrap();

                    let try_create =
                        path::create(&self.current_path, Path::new(overlay.content.trim()), false);
                    if let Some(error) = try_create {
                        overlay.error = &error;
                    } else {
                        return Task::done(Message::Modal(
                            ModalType::CreateFolder,
                            ModalMessage::Close,
                        ))
                        .chain(Task::done(Message::FetchEntries(None)));
                    }
                }

                Task::none()
            }

            Message::Modal(modal_type, msg) => {
                let modals_state = &mut self.states.modals;
                let modals_opened = &mut modals_state.opened;

                match modal_type {
                    ModalType::Rename => {
                        match msg {
                            ModalMessage::Open => {
                                if *modals_opened {
                                    return Task::none();
                                }

                                let current_index = self.current_index;

                                if let Some(index) = current_index {
                                    let selected = self.entries.item(&index).unwrap();

                                    modals_state.rename = Some(RenameModal {
                                        path: selected.path.clone(),
                                        content: selected.name.clone(),
                                        error: "",
                                    })
                                }
                                *modals_opened = true;

                                return operation::focus("rename");
                            }
                            ModalMessage::Close => {
                                modals_state.rename = None;
                                *modals_opened = false;

                                return Task::done(Message::ClearChoices);
                            }
                            ModalMessage::Content(content) => {
                                let overlay = modals_state.rename.as_mut().unwrap();
                                overlay.content = content;
                            }
                        }
                        Task::none()
                    }
                    ModalType::Delete => {
                        match msg {
                            ModalMessage::Open => {
                                if self.selected.is_empty() {
                                    return Task::none();
                                }

                                modals_state.delete = true;
                                *modals_opened = true;

                                self.states.modals.choices.push(Message::Delete);
                            }
                            ModalMessage::Close => {
                                modals_state.delete = false;
                                *modals_opened = false;

                                return Task::done(Message::ClearChoices);
                            }
                            _ => {}
                        }
                        Task::none()
                    }
                    ModalType::Paste => {
                        match msg {
                            ModalMessage::Open => {
                                if self.clipboard.entries.is_empty() {
                                    return Task::none();
                                }

                                modals_state.paste = true;
                                *modals_opened = true;

                                self.states.modals.choices.extend(vec![
                                    Message::PasteClipboard(PasteType::Replace),
                                    Message::PasteClipboard(PasteType::Duplicate),
                                ]);
                            }
                            ModalMessage::Close => {
                                modals_state.paste = false;
                                *modals_opened = false;

                                return Task::done(Message::ClearChoices);
                            }
                            _ => {}
                        }
                        Task::none()
                    }
                    ModalType::CreateFile => {
                        match msg {
                            ModalMessage::Open => {
                                modals_state.create_file = Some(CreateModal {
                                    content: String::new(),
                                    error: "",
                                });
                                *modals_opened = true;

                                return operation::focus("create");
                            }
                            ModalMessage::Close => {
                                modals_state.create_file = None;
                                *modals_opened = false;
                            }
                            ModalMessage::Content(content) => {
                                let overlay = modals_state.create_file.as_mut().unwrap();
                                overlay.content = content;

                                return Task::none();
                            }
                        }
                        Task::done(Message::ClearChoices)
                    }
                    ModalType::CreateFolder => {
                        match msg {
                            ModalMessage::Open => {
                                modals_state.create_folder = Some(CreateModal {
                                    content: String::new(),
                                    error: "",
                                });
                                *modals_opened = true;

                                return operation::focus("create");
                            }
                            ModalMessage::Close => {
                                modals_state.create_folder = None;
                                *modals_opened = false;
                            }
                            ModalMessage::Content(content) => {
                                let overlay = modals_state.create_folder.as_mut().unwrap();
                                overlay.content = content;

                                return Task::none();
                            }
                        }
                        Task::done(Message::ClearChoices)
                    }
                    ModalType::Search => match msg {
                        ModalMessage::Content(content) => {
                            let modal = modals_state.search.as_mut().unwrap();
                            modal.content = content;
                            return Task::done(Message::FilterEntries(None));
                        }
                        ModalMessage::Open => {
                            modals_state.search = Some(SearchModal::default());
                            modals_state.search.as_mut().unwrap().focused = true;
                            return operation::focus("search_box")
                                .chain(Task::done(Message::FilterEntries(None)));
                        }
                        ModalMessage::Close => {
                            modals_state.search.as_mut().unwrap().focused = false;
                            return Task::none();
                        }
                    },
                }
            }
            Message::FocusModal => {
                let mut task = Task::none();

                if self.states.modals.rename.is_some() {
                    task = task.chain(operation::is_focused("rename").then(|focused| {
                        if !focused {
                            return Task::done(Message::Modal(
                                ModalType::Rename,
                                ModalMessage::Close,
                            ));
                        } else {
                            return Task::none();
                        }
                    }));
                }

                if self.states.modals.create_file.is_some()
                    || self.states.modals.create_folder.is_some()
                {
                    task = task.chain(operation::is_focused("create").then(|focused| {
                        if !focused {
                            return Task::batch(vec![
                                Task::done(Message::Modal(
                                    ModalType::CreateFile,
                                    ModalMessage::Close,
                                )),
                                Task::done(Message::Modal(
                                    ModalType::CreateFolder,
                                    ModalMessage::Close,
                                )),
                            ]);
                        } else {
                            return Task::none();
                        }
                    }));
                }

                task
            }
            Message::CloseModals => {
                let modals_state = &mut self.states.modals;

                if modals_state.search.is_some() {
                    modals_state.search = None;
                    return Task::done(Message::FilterEntries(None));
                }

                if !modals_state.opened {
                    return Task::none();
                }

                modals_state.opened = false;
                modals_state.delete = false;
                modals_state.paste = false;

                modals_state.create_file = None;
                modals_state.create_folder = None;
                modals_state.rename = None;
                // sloppy code
                // i mean there has to be some state-resetting somewhere right?

                Task::done(Message::ClearChoices)
            }
            Message::ClearChoices => {
                self.states.modals.choices.clear();
                self.states.modals.current_choice = 0;

                Task::none()
            }
            Message::SelectChoice => {
                let choice = self
                    .states
                    .modals
                    .choices
                    .get(self.states.modals.current_choice);

                if let Some(decision) = choice
                    && self.states.modals.opened
                {
                    // clone is fine here since its a enum (i think (i hope :pray:))
                    return Task::done(decision.clone());
                }
                Task::none()
            }
            Message::ChoiceIndex(right) => {
                if self.states.modals.choices.len() == 0 {
                    return Task::none();
                }

                let cur_choice = self.states.modals.current_choice as i8;
                // conv to i8 because usize cant go under 0
                let dir = if right { 1 } else { -1 };
                let new_index =
                    (cur_choice + dir).clamp(0, (self.states.modals.choices.len() - 1) as i8);

                self.states.modals.current_choice = new_index as usize;

                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        // TODO: add toasts

        let palette = &self.theme.palette;
        let text_color = palette.text;
        let text_muted_color = palette.text_muted;
        let info_color = palette.blue;

        let button_style = button::Style {
            background: Some(Background::Color(palette.accent_dark)),
            ..Default::default()
        };

        let bg_style = container::Style {
            background: Some(Background::Color(palette.background)),
            ..Default::default()
        };

        let panel_style = container::Style {
            background: Some(Background::Color(palette.overlay)),
            ..Default::default()
        };

        let overlay_style = container::Style {
            background: Some(Background::Color(palette.scrim)),
            ..Default::default()
        };

        let text_input_style = text_input::Style {
            background: Background::Color(palette.overlay),
            border: Border::default(),
            placeholder: palette.text_muted,
            icon: palette.text,
            value: palette.text,
            selection: palette.accent,
        };

        let search_input_style = text_input::Style {
            background: Background::Color(Color::from_rgba8(0, 0, 0, 0.0)),
            border: Border::default(),
            placeholder: palette.text_muted,
            icon: palette.text_muted,
            value: palette.text,
            selection: palette.accent,
        };

        let unfocused_search_style = text::Style {
            color: Some(palette.text),
        };

        if self.states.is_loading {
            return container(text("loading...").color(text_color).size(17))
                .style(move |_| overlay_style)
                .center(Length::Fill)
                .into();
        }

        let mut explorer_column = column![].spacing(10).width(Length::Fill);

        for (index, &entry_index) in self.entries.displaying.iter().enumerate() {
            let mut row = row![].spacing(10);

            let item_opt = &self.entries.children.get(entry_index);
            let item;

            if let Some(thing) = item_opt {
                item = thing;
            } else {
                continue;
            }

            row = row
                .push(container(svg(item.icon.deref().clone()).width(16).height(16)).center_y(30));

            for child in &self.config.view.explorer {
                match child {
                    Displaying::Name => {
                        row = row.push(
                            container(
                                text(&item.name)
                                    .wrapping(Wrapping::None)
                                    .align_x(alignment::Horizontal::Left)
                                    .color(if item.hidden {
                                        text_muted_color.scale_alpha(0.7)
                                    } else {
                                        text_color
                                    }),
                            )
                            .center(30)
                            .align_left(300)
                            .clip(true),
                        )
                    }
                    Displaying::FileSize => {
                        let txt = if let Some(s) = item.foldersize {
                            format!("{} items", s)
                        } else {
                            path::bytes_to_string(item.filesize)
                        };

                        row = row.push(
                            container(
                                text(txt)
                                    .align_x(alignment::Horizontal::Left)
                                    .wrapping(Wrapping::None)
                                    .color(if item.hidden {
                                        text_muted_color.scale_alpha(0.7)
                                    } else {
                                        text_color
                                    }),
                            )
                            .center(30)
                            .align_left(100)
                            .clip(true),
                        );
                    }
                    Displaying::FileType => {
                        row = row.push(
                            container(
                                text(&item.filetype)
                                    .align_x(alignment::Horizontal::Left)
                                    .wrapping(Wrapping::None)
                                    .color(if item.hidden {
                                        text_muted_color.scale_alpha(0.7)
                                    } else {
                                        text_color
                                    }),
                            )
                            .align_left(150)
                            .center(30)
                            .clip(true),
                        );
                    }
                    Displaying::Created => {
                        row = row.push(
                            container(
                                text(format_date(item.created))
                                    .align_x(alignment::Horizontal::Left)
                                    .wrapping(Wrapping::None)
                                    .color(if item.hidden {
                                        text_muted_color.scale_alpha(0.7)
                                    } else {
                                        text_color
                                    }),
                            )
                            .center(30)
                            .align_left(200)
                            .clip(true),
                        );
                    }
                    Displaying::LastAccessed => {
                        row = row.push(
                            container(
                                text(format_date(item.accessed))
                                    .align_x(alignment::Horizontal::Left)
                                    .wrapping(Wrapping::None)
                                    .color(if item.hidden {
                                        text_muted_color.scale_alpha(0.7)
                                    } else {
                                        text_color
                                    }),
                            )
                            .center(30)
                            .align_left(200)
                            .clip(true),
                        );
                    }
                }
            }

            let hovered = item.hovered;
            let selected = self.selected.contains(&index);
            let current_index = self.current_index;

            explorer_column = explorer_column.push(
                container(
                    mouse_area(row)
                        .on_double_click(Message::Open(Some(index)))
                        .on_press(Message::Select(index))
                        .on_enter(Message::HoverEntry(index, true))
                        .on_exit(Message::HoverEntry(index, false)),
                )
                .padding(Padding::from([0, 5]))
                .style(move |_| {
                    let mut style = container::Style::default();

                    if let Some(cur_index) = current_index
                        && cur_index == index
                    {
                        style.border = Border {
                            color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                            width: 2.0,
                            radius: Radius::new(4.0),
                        };
                    }

                    if hovered {
                        style.background =
                            Some(Background::Color(Color::from_rgba(0.4, 0.4, 0.4, 0.1)));
                    }

                    if selected {
                        style.background =
                            Some(Background::Color(Color::from_rgba(0.4, 0.4, 0.4, 0.3)));
                    }
                    style
                }),
            )
        }

        let explorer_scroll = scrollable(explorer_column)
            .id("scrollable")
            .width(Length::Fill)
            .height(Length::Fill)
            .on_scroll(Message::ExplorerOffset);

        let mut column_names = row![].spacing(10).padding(5);
        column_names = column_names.push(container(text("")).width(16).height(16));

        for child in &self.config.view.explorer {
            match child {
                Displaying::Name => {
                    column_names = column_names.push(
                        container(
                            text("file name")
                                .color(text_color)
                                .wrapping(Wrapping::None)
                                .align_x(alignment::Horizontal::Left),
                        )
                        .align_left(300)
                        .center_y(30)
                        .clip(true),
                    );
                }
                Displaying::FileSize => {
                    column_names = column_names.push(
                        container(
                            text("size")
                                .color(text_color)
                                .wrapping(Wrapping::None)
                                .align_x(alignment::Horizontal::Left),
                        )
                        .clip(true)
                        .center_y(30)
                        .align_left(100),
                    );
                }
                Displaying::FileType => {
                    column_names = column_names.push(
                        container(
                            text("type")
                                .color(text_color)
                                .wrapping(Wrapping::None)
                                .align_x(alignment::Horizontal::Left),
                        )
                        .clip(true)
                        .center_y(30)
                        .align_left(150),
                    );
                }
                Displaying::Created => {
                    column_names = column_names.push(
                        container(
                            text("creation date")
                                .color(text_color)
                                .wrapping(Wrapping::None)
                                .align_x(alignment::Horizontal::Left),
                        )
                        .clip(true)
                        .align_left(200)
                        .center_y(30),
                    );
                }
                Displaying::LastAccessed => {
                    column_names = column_names.push(
                        container(
                            text("accessed date")
                                .color(text_color)
                                .wrapping(Wrapping::None)
                                .align_x(alignment::Horizontal::Left),
                        )
                        .align_left(200)
                        .center_y(30)
                        .clip(true),
                    );
                }
            }
        }

        let mut explorer_select_col = column![column_names,].spacing(10);

        if let Some(error) = &self.states.explorer.error {
            explorer_select_col = explorer_select_col.push(
                text(error)
                    .center()
                    .width(Length::Fill)
                    .color(palette.yellow.scale_alpha(0.5)),
            );
        }

        explorer_select_col =
            explorer_select_col.push(mouse_area(explorer_scroll).on_press(Message::ResetSelection));

        let explorer_select = container(explorer_select_col)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20);

        let clipboard_mode = &self.clipboard.mode;
        let mut clipboard_mode_display = "";

        if let Some(mode) = clipboard_mode {
            clipboard_mode_display = match mode {
                ClipboardMode::Copy => "Clipboard Mode: Copy",
                ClipboardMode::Cut => "Clipboard Mode: Cut",
            };
        }

        let clipboard_entries = &self.clipboard.entries;
        let clipboard: Element<Message> =
            column![text(clipboard_mode_display).color(palette.green)]
                .extend(
                    clipboard_entries
                        .iter()
                        .map(|e| text(e.display().to_string()).color(text_color).into()),
                )
                .spacing(10)
                .width(Length::Fill)
                .height(Length::Fill)
                .into();

        let mut explorer_info = column![
            container(text("explorer info").color(text_color))
                .height(30)
                .center_y(30)
                .center_x(Length::Fill)
                .padding(5),
            text(format!(
                "sorting by: {} ({})",
                match self.config.sorting.sorting_by {
                    SortingBy::Name => "name",
                    SortingBy::Type => "file type",
                    SortingBy::Size => "file size",
                    SortingBy::Created => "creation date",
                    SortingBy::Accessed => "last accessed date",
                },
                if self.config.sorting.reversed {
                    "↑"
                } else {
                    "↓"
                }
            ),)
            .color(text_color)
        ]
        .spacing(10)
        .height(Length::Fill);

        if self.states.is_visual_mode {
            explorer_info = explorer_info.push(
                text("VISUAL MODE")
                    .color(text_color)
                    .height(20)
                    .width(Length::Fill),
            );
        }

        if self.config.view_hidden {
            explorer_info = explorer_info.push(
                text(format!("showing hidden files",))
                    .height(20)
                    .width(Length::Fill)
                    .color(text_color),
            );
        }

        explorer_info = explorer_info.push(clipboard);

        let mut file_info =
            row![container(text("file metadata").color(text_color)).width(Length::Fill)]
                .spacing(20);

        if let Some(index) = self.current_index
            && let Some(item) = self.entries.item(&index)
        {
            for v in &self.config.view.metadata {
                match v {
                    Displaying::Name => {
                        file_info =
                            file_info.push(text(format!("name: {}", item.name)).color(text_color));
                    }
                    Displaying::FileType => {
                        file_info = file_info
                            .push(text(format!("type: {}", item.filetype)).color(text_color));
                    }
                    Displaying::FileSize => {
                        file_info = file_info.push(
                            text(format!(
                                "size: {}",
                                path::bytes_to_string(if self.config.misc.accurate_filesize {
                                    path::accurate_filesize(&item.path)
                                } else {
                                    item.filesize
                                })
                            ))
                            .color(text_color),
                        );
                    }
                    Displaying::LastAccessed => {
                        file_info = file_info.push(
                            text(format!(
                                "last accessed: {}",
                                DateTime::from_timestamp_secs(item.accessed)
                                    .unwrap()
                                    .format(&self.config.misc.format_date)
                            ))
                            .color(text_color),
                        );
                    }
                    Displaying::Created => {
                        file_info = file_info.push(
                            text(format!(
                                "creation date: {}",
                                DateTime::from_timestamp_secs(item.created)
                                    .unwrap()
                                    .format(&self.config.misc.format_date)
                            ))
                            .color(text_color),
                        );
                    }
                };
            }
        };

        let mut left_col = column![
            row![
                button(text("..back").color(text_color))
                    .style(move |_, _| button_style.into())
                    .on_press(Message::NavigateBack),
                container(text(format!("{}", self.current_path.display())).color(text_color))
                    .style(move |_| { palette.text.scale_alpha(0.1).into() })
                    .center_y(30)
                    .center_x(Length::Fill)
                    .padding(5),
            ],
            explorer_select,
            container(file_info.wrap().vertical_spacing(20)).padding(10)
        ]
        .spacing(10)
        .height(Length::Fill)
        .width(Length::Fill);

        if let Some(modal) = &self.states.modals.search {
            if modal.focused {
                left_col = left_col.push(
                    text_input("searching...", &modal.content)
                        .style(move |_, _| search_input_style)
                        .padding(Padding::from([5, 10]))
                        .on_input(|inp| {
                            Message::Modal(ModalType::Search, ModalMessage::Content(inp))
                        })
                        .on_submit(Message::Modal(ModalType::Search, ModalMessage::Close))
                        .id("search_box"),
                );
            } else {
                left_col = left_col.push(
                    container(text(&modal.content).style(move |_| unfocused_search_style))
                        .padding(Padding::from([5, 10])),
                );
            }
        }

        let right_col = column![explorer_info].width(250).spacing(20);

        let content = container(
            row![
                container(left_col)
                    .padding(5)
                    .clip(true)
                    .style(move |_| panel_style.into()),
                container(right_col)
                    .padding(5)
                    .clip(true)
                    .style(move |_| panel_style.into()),
            ]
            .spacing(20),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(20)
        .style(move |_| bg_style.into());

        let mut stack = stack![content].width(Length::Fill).height(Length::Fill);

        if let Some(modal) = &self.states.modals.rename {
            let input = text_input("input the new name here :3", &modal.content)
                .on_input(|inp| Message::Modal(ModalType::Rename, ModalMessage::Content(inp)))
                .on_submit(Message::Rename)
                .style(move |_, _| text_input_style)
                .padding(7)
                .id("rename");

            let col = column![
                text("press Esc to exit, Enter to confirm :D").color(info_color),
                text(format!("you are renaming, {}", modal.path.display())).color(text_color),
                input,
                text(modal.error).color(palette.yellow).size(13)
            ]
            .width(500)
            .spacing(10);

            let overlay = opaque(float(
                container(col)
                    .style(move |_| overlay_style)
                    .center(Length::Fill),
            ));

            stack = stack.push(overlay);
        }

        if self.states.modals.paste {
            let row = row![
                button(text("Replace \nreplace file if name is matched").color(text_color))
                    .on_press(Message::PasteClipboard(PasteType::Replace))
                    .padding(7)
                    .style(move |_, _| {
                        let mut style = button_style;

                        if self.states.modals.current_choice == 0 {
                            style.border = Border {
                                color: palette.yellow,
                                width: 2.0,
                                radius: Radius::new(8.0),
                            }
                        }
                        style
                    }),
                button(
                    text("Duplicate \nadd (n) to the end of file name if name is matched")
                        .color(text_color)
                )
                .on_press(Message::PasteClipboard(PasteType::Duplicate))
                .padding(7)
                .style(move |_, _| {
                    let mut style = button_style;

                    if self.states.modals.current_choice == 1 {
                        style.border = Border {
                            color: palette.yellow,
                            width: 2.0,
                            radius: Radius::new(8.0),
                        }
                    }
                    style
                }),
            ]
            .spacing(10);

            let overlay = opaque(float(
                container(
                    column![
                        text("press Esc to exit").size(13).color(info_color),
                        text("choose a response when overlapping files").color(text_color),
                        row
                    ]
                    .spacing(10),
                )
                .style(move |_| overlay_style)
                .center(Length::Fill),
            ));

            stack = stack.push(overlay);
        }

        if self.states.modals.delete {
            let overlay = opaque(float(
                container(
                    column![
                        text("press Esc to exit").size(13).color(info_color),
                        text("you gonna delete the selections?").color(text_color),
                        button(text("yeah :3").color(text_color))
                            .padding(7)
                            .style(move |_, _| {
                                let mut style = button_style;

                                if self.states.modals.current_choice == 0 {
                                    style.border = Border {
                                        color: palette.yellow,
                                        width: 2.0,
                                        radius: Radius::new(8.0),
                                    }
                                }
                                style
                            })
                            .on_press(Message::Delete)
                    ]
                    .spacing(10),
                )
                .style(move |_| overlay_style)
                .center(Length::Fill),
            ));

            stack = stack.push(overlay);
        }

        if let Some(modal) = &self.states.modals.create_file {
            let input = text_input("input the file path here! :3", &modal.content)
                .on_input(|inp| Message::Modal(ModalType::CreateFile, ModalMessage::Content(inp)))
                .on_submit(Message::Create(true))
                .style(move |_, _| text_input_style)
                .padding(7)
                .id("create");

            let col = column![
                text(format!(
                    "creating a new file in {}",
                    self.current_path.display()
                ))
                .color(text_color),
                input,
                text("press Esc to exit, Enter to confirm :D")
                    .size(13)
                    .color(info_color),
                text(modal.error).color(palette.yellow)
            ]
            .width(497)
            .spacing(7);

            let overlay = opaque(float(
                container(col)
                    .style(move |_| overlay_style)
                    .center(Length::Fill),
            ));

            stack = stack.push(overlay);
        }

        if let Some(modal) = &self.states.modals.create_folder {
            let input = text_input("input the folder path here! :3", &modal.content)
                .on_input(|inp| Message::Modal(ModalType::CreateFolder, ModalMessage::Content(inp)))
                .on_submit(Message::Create(false))
                .style(move |_, _| text_input_style)
                .padding(7)
                .id("create");

            let col = column![
                text(format!(
                    "creating new folder(s) in {}",
                    self.current_path.display()
                ))
                .color(text_color),
                input,
                text("press Esc to exit, Enter to confirm :D")
                    .size(13)
                    .color(info_color),
                text(modal.error).color(palette.yellow)
            ]
            .width(497)
            .spacing(7);

            let overlay = opaque(float(
                container(col)
                    .style(move |_| overlay_style)
                    .center(Length::Fill),
            ));

            stack = stack.push(overlay);
        }

        stack.into()
    }

    fn sort(&mut self, index: usize, from_start: bool) {
        let sorting_by = &self.config.sorting.sorting_by;
        let reference = &self.entries.children;
        let displaying = if from_start {
            &mut self.entries.displaying[..index]
        } else {
            &mut self.entries.displaying[index..]
        };

        match sorting_by {
            SortingBy::Name => {
                let lowercased: HashMap<usize, String> = displaying
                    .iter()
                    .map(|entry_index| {
                        let v = self.entries.children[*entry_index].name.to_lowercase();
                        return (entry_index.clone(), v);
                    })
                    .collect();

                displaying.par_sort_by(|a, b| {
                    let (x, y) = (&lowercased[a], &lowercased[b]);
                    x.cmp(y)
                })
            }
            SortingBy::Size => displaying.par_sort_by(|a, b| {
                let (x, y) = (&reference[*a].filesize, &reference[*b].filesize);
                x.cmp(y)
            }),
            SortingBy::Type => displaying.par_sort_by(|a, b| {
                let (x, y) = (&reference[*a].filetype, &reference[*b].filetype);
                x.cmp(y)
            }),
            SortingBy::Created => displaying.par_sort_by(|a, b| {
                let (x, y) = (&reference[*a].created, &reference[*b].created);
                x.cmp(y)
            }),
            SortingBy::Accessed => displaying.par_sort_by(|a, b| {
                let (x, y) = (&reference[*a].accessed, &reference[*b].accessed);
                x.cmp(y)
            }),
        }

        if self.config.sorting.reversed {
            displaying.reverse();
        }
    }

    pub fn push_entry(&mut self, entry: &TempItem, index: usize) {
        let filesize = entry.filesize;
        let hidden = entry.hidden;
        let accessed = entry.accessed;
        let created = entry.created;
        let name = entry.name;
        let path = entry.path;
        let filetype = entry.filetype;
        let foldersize = entry.foldersize;
        let icon = entry.icon;

        let item_opt = self.entries.children.get_mut(index);

        if let Some(item) = item_opt {
            item.filesize = filesize;
            item.hidden = hidden;
            item.accessed = accessed;
            item.created = created;
            item.using = true;
            item.foldersize = foldersize;
            item.icon = icon;

            item.name.push_str(name);
            item.path.push(path);
            item.filetype.push_str(filetype);
        } else {
            let mut entry = Item {
                filesize,
                hidden,
                accessed,
                created,
                foldersize,
                icon,
                using: true,
                ..Default::default()
            };

            entry.name.push_str(name);
            entry.path.push(path);
            entry.filetype.push_str(filetype);

            self.entries.children.push(entry);
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        event::listen_with(move |event, status, _| match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(state)) => Some(
                Message::KeyModifiers(state.control(), state.shift(), state.alt()),
            ),

            Event::Keyboard(keyboard::Event::KeyPressed {
                physical_key,
                modifiers,
                ..
            }) => match (physical_key, modifiers) {
                (key::Physical::Code(Code::Enter), _) => Some(Message::SelectChoice),

                _ => Some(Message::HandleEvent(physical_key, modifiers, status)),
            },
            _ => None,
        })
    }
}

fn format_date(date: i64) -> String {
    let current_date = Utc::now();
    let given_date = DateTime::from_timestamp_secs(date).unwrap_or_default();

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
