use std::{
    collections::HashSet,
    env::home_dir,
    ops::Sub,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use chrono::{DateTime, Datelike, Utc};
use rayon::prelude::*;

use iced::{
    Background, Border, Color, Element, Event, Length, Padding, Subscription, Task, Theme,
    alignment,
    border::Radius,
    event::{self, Status},
    keyboard::{
        self, Modifiers,
        key::{self, Code, Physical},
    },
    widget::{
        button, column, container, float, mouse_area, opaque, operation, row, scrollable, selector,
        stack, text, text_input,
    },
};

use crate::types::{
    Clipboard, ClipboardMode, CreateModal, Direction, Entries, Entry, ModalMessage, ModalType,
    PasteType, RenameModal,
};
use iced::widget::{
    operation::AbsoluteOffset, scrollable::Viewport, selector::Target, text::Wrapping,
};

use crate::config::{self, Displaying, SortingBy};
use crate::path;

struct States {
    modifiers: ModifiersState,
    modals: ModalsState,
    explorer: ExplorerState,
    is_visual_mode: bool,
}

impl Default for States {
    fn default() -> Self {
        States {
            modifiers: ModifiersState::default(),
            modals: ModalsState::default(),
            explorer: ExplorerState::default(),
            is_visual_mode: false,
        }
    }
}

struct ModalsState {
    opened: bool,
    operation: bool,
    delete: bool,
    create_file: Option<CreateModal>,
    create_folder: Option<CreateModal>,
    rename: Option<RenameModal>,

    choices: Vec<Message>,
    current_choice: usize,
}

impl Default for ModalsState {
    fn default() -> Self {
        ModalsState {
            opened: false,
            operation: false,
            delete: false,
            create_file: None,
            create_folder: None,
            rename: None,
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
    UpdateEntries(Option<PathBuf>),
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
    KeyPressed(Physical, Modifiers),
    KeyModifiers(bool, bool, bool),

    // modals, modal choices navigation
    Modal(ModalType, ModalMessage),
    FocusModal,
    CloseModals,
    ClearChoices,
    SelectChoice,
    ChoiceIndex(bool),
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
    pub fn new(input: &str, config: config::Config) -> (Self, Task<Message>) {
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
                config,
                theme: Theme::CatppuccinLatte,

                current_path: path,
                current_index: None,

                entries: Entries::new(),
                clipboard: Clipboard::default(),
                selected: HashSet::new(),

                states: States::default(),
            },
            Task::done(Message::UpdateEntries(None)),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Open(index) => {
                if index.is_none() {
                    return Task::none();
                }

                let mut path = &PathBuf::new();

                self.entries
                    .children
                    .iter()
                    .enumerate()
                    .for_each(|(i, entry)| {
                        if i == index.unwrap() {
                            path = &entry.path;
                        }
                    });

                if path.is_file() {
                    let cmd = Command::new("xdg-open")
                        .arg(path)
                        .stderr(Stdio::null())
                        .spawn();

                    if let Err(e) = cmd {
                        println!("{}", e);
                    }

                    Task::none()
                } else {
                    self.current_path = path.to_owned();
                    self.current_index = None;
                    Task::done(Message::UpdateEntries(None))
                }
            }
            Message::NavigateBack => {
                if self.states.modals.opened {
                    return Task::none();
                }

                let path = Some(self.current_path.clone());
                self.current_path.pop();

                Task::done(Message::UpdateEntries(path)).chain(
                    selector::find(selector::id("scrollable"))
                        .then(|output| Task::done(Message::ExplorerScroll(output))),
                )
            }

            Message::KeyPressed(physical_key, modifiers) => {
                let keybinds = &self.config.keybinds;

                if physical_key == keybinds.navigate_backward.key
                    && modifiers == keybinds.navigate_backward.modifiers
                {
                    // for navigating up the directory tree AND changing modal selection index
                    return Task::done(Message::ChoiceIndex(false))
                        .chain(Task::done(Message::NavigateBack));
                } else if physical_key == keybinds.navigate_forward.key
                    && modifiers == keybinds.navigate_forward.modifiers
                {
                    // for navigating down the directory tree AND changing modal selection index
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
                }

                Task::none()
            }

            Message::UpdateEntries(prev_path) => {
                self.entries.children.clear();
                self.current_index = None;
                self.selected.clear();

                let try_get_paths = path::read_dir(&self.current_path);

                if let Err(error) = try_get_paths {
                    self.states.explorer.error = Some(error);

                    return Task::none();
                }

                self.states.explorer.error = None;

                let cur_paths = try_get_paths.unwrap();

                let mut i: usize = 0;

                let entries = &mut self.entries.children;
                let mut non_hidden_entries: Vec<Entry> = Vec::with_capacity(30);

                for path in cur_paths {
                    let is_hidden = path::is_hidden(&path);

                    if !self.config.view_hidden && is_hidden {
                        continue;
                    }

                    let entry = Entry {
                        id: i,
                        name: path.file_name().unwrap().to_str().unwrap().to_string(),

                        created: path::file_created(&path),
                        accessed: path::file_accessed(&path),

                        filetype: path::file_type(&path),
                        filesize: path::file_size(&path),

                        hovered: false,
                        hidden: is_hidden,

                        path: path,
                    };

                    if !is_hidden {
                        non_hidden_entries.push(entry);
                    } else {
                        // pushing hidden entries in first so they display separately
                        entries.push(entry);
                    }

                    i += 1;
                }

                // sorting logic
                let sorting_config = &self.config.sorting;

                sort(&sorting_config.sorting_by, entries);
                sort(&sorting_config.sorting_by, &mut non_hidden_entries);

                if sorting_config.reversed {
                    entries.reverse();
                    non_hidden_entries.reverse();
                }

                self.entries.children.extend(non_hidden_entries);

                // highlight from lower directory if provided
                if let Some(path) = prev_path {
                    self.entries.children.iter().for_each(|entry| {
                        if entry.path == path {
                            self.current_index = Some(self.entries.index(&entry.id));
                        }
                    });
                }

                Task::none()
            }
            Message::HoverEntry(id, state) => {
                self.entries.children.par_iter_mut().for_each(|entry| {
                    if entry.id == id {
                        entry.hovered = state
                    }
                });

                Task::none()
            }

            Message::ToggleHiddenView => {
                self.config.view_hidden = !self.config.view_hidden;
                Task::done(Message::UpdateEntries(None))
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
                let try_current_index = self.current_index;

                if try_current_index.is_none() {
                    return Task::none();
                }

                let current_index: f32 = try_current_index.unwrap() as f32 + 1.0;
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
                    let try_getentry = &self.entries.children.get(*index);

                    if let Some(entry) = try_getentry {
                        path::delete(&entry.path);
                    }
                }

                Task::done(Message::Modal(ModalType::Delete, ModalMessage::Close))
                    .chain(Task::done(Message::UpdateEntries(None)))
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
                // shitty logic that forces the index to be 0 if nothing is selected yet.

                match direction {
                    Direction::Down => {
                        if current_index < self.entries.children.len() - 1 {
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
                        .insert(self.entries.children.get(*i).unwrap().path.clone());
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
                    .chain(Task::done(Message::UpdateEntries(None)))
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
                    .chain(Task::done(Message::UpdateEntries(None)))
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
                        .chain(Task::done(Message::UpdateEntries(None)));
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
                        .chain(Task::done(Message::UpdateEntries(None)));
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
                                    let selected = self.entries.children.get(index).unwrap();

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

                                modals_state.operation = true;
                                *modals_opened = true;

                                self.states.modals.choices.extend(vec![
                                    Message::PasteClipboard(PasteType::Replace),
                                    Message::PasteClipboard(PasteType::Duplicate),
                                ]);
                            }
                            ModalMessage::Close => {
                                modals_state.operation = false;
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
                }
            }
            Message::FocusModal => {
                let mut task = Task::none();

                task = task.chain(operation::is_focused("rename").then(|focused| {
                    if !focused {
                        return Task::done(Message::Modal(ModalType::Rename, ModalMessage::Close));
                    } else {
                        return Task::none();
                    }
                }));

                task = task.chain(operation::is_focused("create").then(|focused| {
                    if !focused {
                        return Task::batch(vec![
                            Task::done(Message::Modal(ModalType::CreateFile, ModalMessage::Close)),
                            Task::done(Message::Modal(
                                ModalType::CreateFolder,
                                ModalMessage::Close,
                            )),
                        ]);
                    } else {
                        return Task::none();
                    }
                }));

                task
            }
            Message::CloseModals => {
                let modals_state = &mut self.states.modals;

                if !modals_state.opened {
                    return Task::none();
                }

                modals_state.opened = false;
                modals_state.delete = false;
                modals_state.operation = false;

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
        let entries = &self.entries.children;

        let palette = self.theme.palette();
        let text_color = palette.text;
        let darken_text_color = text_color.scale_alpha(0.69);

        let explorer_column = column(
            entries
                .iter()
                .map(|entry| {
                    let mut row = row![].spacing(10);

                    for child in &self.config.view {
                        match child {
                            Displaying::Name => {
                                row = row.push(
                                    container(
                                        text(&entry.name)
                                            .wrapping(Wrapping::None)
                                            .align_x(alignment::Horizontal::Left)
                                            .color(if entry.hidden {
                                                darken_text_color
                                            } else {
                                                text_color
                                            }),
                                    )
                                    .width(300)
                                    .clip(true),
                                )
                            }
                            Displaying::FileSize => {
                                row = row.push(
                                    container(
                                        text(path::bytes_to_string(&entry.filesize))
                                            .align_x(alignment::Horizontal::Left)
                                            .wrapping(Wrapping::None)
                                            .color(if entry.hidden {
                                                darken_text_color
                                            } else {
                                                text_color
                                            }),
                                    )
                                    .width(100)
                                    .clip(true),
                                );
                            }
                            Displaying::FileType => {
                                row = row.push(
                                    container(
                                        text(&entry.filetype)
                                            .align_x(alignment::Horizontal::Left)
                                            .wrapping(Wrapping::None)
                                            .color(if entry.hidden {
                                                darken_text_color
                                            } else {
                                                text_color
                                            }),
                                    )
                                    .width(150)
                                    .clip(true),
                                );
                            }
                            Displaying::Created => {
                                row = row.push(
                                    container(
                                        text(format_date(entry.created))
                                            .align_x(alignment::Horizontal::Left)
                                            .wrapping(Wrapping::None)
                                            .color(if entry.hidden {
                                                darken_text_color
                                            } else {
                                                text_color
                                            }),
                                    )
                                    .width(200)
                                    .clip(true),
                                );
                            }
                            Displaying::LastAccessed => {
                                row = row.push(
                                    container(
                                        text(format_date(entry.accessed))
                                            .align_x(alignment::Horizontal::Left)
                                            .wrapping(Wrapping::None)
                                            .color(if entry.hidden {
                                                darken_text_color
                                            } else {
                                                text_color
                                            }),
                                    )
                                    .width(200)
                                    .clip(true),
                                );
                            }
                        }
                    }

                    let index = self.entries.index(&entry.id);

                    container(
                        mouse_area(row)
                            .on_double_click(Message::Open(Some(index)))
                            .on_press(Message::Select(index))
                            .on_enter(Message::HoverEntry(entry.id, true))
                            .on_exit(Message::HoverEntry(entry.id, false)),
                    )
                    .center_y(30)
                    .padding(Padding::from([0, 5]))
                    .style(|_| {
                        let mut style = container::Style::default();
                        let index = self.entries.index(&entry.id);

                        if let Some(cur_index) = self.current_index
                            && cur_index == index
                        {
                            style.border = Border {
                                color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                                width: 2.0,
                                radius: Radius::new(4.0),
                            };
                        }

                        if entry.hovered {
                            style.background =
                                Some(Background::Color(Color::from_rgba(0.4, 0.4, 0.4, 0.1)));
                        }

                        if self.selected.contains(&index) {
                            style.background =
                                Some(Background::Color(Color::from_rgba(0.4, 0.4, 0.4, 0.3)));
                        }
                        style
                    })
                    .into()
                })
                .collect::<Vec<_>>(),
        )
        .spacing(10)
        .width(Length::Fill);

        let explorer_scroll = scrollable(explorer_column)
            .id("scrollable")
            .width(Length::Fill)
            .height(Length::Fill)
            .on_scroll(Message::ExplorerOffset);

        let mut column_names = row![].spacing(10).padding(5);

        for child in &self.config.view {
            match child {
                Displaying::Name => {
                    column_names = column_names.push(
                        container(
                            text("file name")
                                .wrapping(Wrapping::None)
                                .align_x(alignment::Horizontal::Left),
                        )
                        .width(300)
                        .clip(true),
                    );
                }
                Displaying::FileSize => {
                    column_names = column_names.push(
                        container(
                            text("size")
                                .wrapping(Wrapping::None)
                                .align_x(alignment::Horizontal::Left),
                        )
                        .clip(true)
                        .width(100),
                    );
                }
                Displaying::FileType => {
                    column_names = column_names.push(
                        container(
                            text("type")
                                .wrapping(Wrapping::None)
                                .align_x(alignment::Horizontal::Left),
                        )
                        .clip(true)
                        .width(150),
                    );
                }
                Displaying::Created => {
                    column_names = column_names.push(
                        container(
                            text("creation date")
                                .wrapping(Wrapping::None)
                                .align_x(alignment::Horizontal::Left),
                        )
                        .clip(true)
                        .width(200),
                    );
                }
                Displaying::LastAccessed => {
                    column_names = column_names.push(
                        container(
                            text("accessed date")
                                .wrapping(Wrapping::None)
                                .align_x(alignment::Horizontal::Left),
                        )
                        .width(200)
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
                    .color(palette.danger.scale_alpha(0.5)),
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
            column![text(clipboard_mode_display).color(palette.success)]
                .extend(
                    clipboard_entries
                        .iter()
                        .map(|e| text(e.display().to_string()).into()),
                )
                .spacing(10)
                .width(Length::Fill)
                .height(Length::Fill)
                .into();

        let left_col = column![
            row![
                button(text("....")).on_press(Message::NavigateBack),
                container(text(format!("{}", self.current_path.display())))
                    .style(move |_| { palette.text.scale_alpha(0.1).into() })
                    .center_y(30)
                    .center_x(Length::Fill)
                    .padding(5),
            ],
            explorer_select
        ]
        .spacing(10)
        .height(Length::Fill)
        .width(Length::Fill);

        let mut explorer_info = column![
            container(text("explorer info"))
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
        ]
        .spacing(10)
        .height(Length::Fill);

        if self.states.is_visual_mode {
            explorer_info = explorer_info.push(text("VISUAL MODE").height(20).width(Length::Fill));
        }

        if self.config.view_hidden {
            explorer_info = explorer_info.push(
                text(format!("showing hidden files",))
                    .height(20)
                    .width(Length::Fill),
            );
        }

        explorer_info = explorer_info.push(clipboard);

        let mut file_info = column![
            container(text("file metadata"))
                .height(30)
                .center_y(30)
                .center_x(Length::Fill)
                .padding(5),
        ]
        .spacing(10);

        if let Some(index) = self.current_index {
            let entry = self.entries.children.get(index).unwrap();

            file_info = file_info.extend(vec![
                text(format!("name: {}", entry.name)).into(),
                text(format!("type: {}", entry.filetype)).into(),
                text(format!("size: {}", path::bytes_to_string(&entry.filesize))).into(),
                text(format!(
                    "last accessed: {}",
                    DateTime::from_timestamp_secs(entry.accessed)
                        .unwrap()
                        .format(&self.config.misc.format_date)
                ))
                .into(),
                text(format!(
                    "creation date: {}",
                    DateTime::from_timestamp_secs(entry.created)
                        .unwrap()
                        .format(&self.config.misc.format_date)
                ))
                .into(),
            ]);
        }

        let right_col = column![explorer_info, file_info].width(300).spacing(20);

        let content = row![left_col, right_col]
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(30)
            .spacing(10);

        let mut stack = stack![content].width(Length::Fill).height(Length::Fill);

        if let Some(modal) = &self.states.modals.rename {
            let input = text_input("input the new name here :3", &modal.content)
                .on_input(|inp| Message::Modal(ModalType::Rename, ModalMessage::Content(inp)))
                .on_submit(Message::Rename)
                .padding(7)
                .id("rename");

            let col = column![
                text("press Esc to exit, Enter to confirm :D")
                    .color(palette.primary.scale_alpha(0.4)),
                text(format!("you are renaming, {}", modal.path.display())),
                input,
                text(modal.error).color(palette.warning).size(13)
            ]
            .width(500)
            .spacing(10);

            let overlay = opaque(float(
                container(col)
                    .style(move |_| palette.background.scale_alpha(0.8).into())
                    .center(Length::Fill),
            ));

            stack = stack.push(overlay);
        }

        if self.states.modals.operation {
            let row = row![
                button(text("Replace \nreplace file if name is matched"))
                    .on_press(Message::PasteClipboard(PasteType::Replace))
                    .padding(7)
                    .style(move |_, _| {
                        let mut style = button::Style::default();

                        if self.states.modals.current_choice == 0 {
                            style.border = Border {
                                color: palette.warning,
                                width: 2.0,
                                radius: Radius::new(8.0),
                            }
                        }
                        style
                    }),
                button(text(
                    "Duplicate \nadd (n) to the end of file name if name is matched"
                ))
                .on_press(Message::PasteClipboard(PasteType::Duplicate))
                .padding(7)
                .style(move |_, _| {
                    let mut style = button::Style::default();

                    if self.states.modals.current_choice == 1 {
                        style.border = Border {
                            color: palette.warning,
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
                        text("press Esc to exit")
                            .size(13)
                            .color(palette.primary.scale_alpha(0.4)),
                        text("choose an operation type"),
                        row
                    ]
                    .spacing(10),
                )
                .style(move |_| palette.background.scale_alpha(0.8).into())
                .center(Length::Fill),
            ));

            stack = stack.push(overlay);
        }

        if self.states.modals.delete {
            let overlay = opaque(float(
                container(
                    column![
                        text("press Esc to exit")
                            .size(13)
                            .color(palette.primary.scale_alpha(0.4)),
                        text("you gonna delete the selections?"),
                        button(text("yeah :3"))
                            .padding(7)
                            .style(move |_, _| {
                                let mut style = button::Style::default();

                                if self.states.modals.current_choice == 0 {
                                    style.border = Border {
                                        color: palette.warning,
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
                .style(move |_| palette.background.scale_alpha(0.8).into())
                .center(Length::Fill),
            ));

            stack = stack.push(overlay);
        }

        if let Some(modal) = &self.states.modals.create_file {
            let input = text_input("input the file path here! :3", &modal.content)
                .on_input(|inp| Message::Modal(ModalType::CreateFile, ModalMessage::Content(inp)))
                .on_submit(Message::Create(true))
                .padding(7)
                .id("create");

            let col = column![
                text(format!(
                    "creating a new file in {}",
                    self.current_path.display()
                )),
                input,
                text("press Esc to exit, Enter to confirm :D")
                    .size(13)
                    .color(palette.primary.scale_alpha(0.4)),
                text(modal.error).color(palette.warning)
            ]
            .width(497)
            .spacing(7);

            let overlay = opaque(float(
                container(col)
                    .style(move |_| palette.background.scale_alpha(0.8).into())
                    .center(Length::Fill),
            ));

            stack = stack.push(overlay);
        }

        if let Some(modal) = &self.states.modals.create_folder {
            let input = text_input("input the folder path here! :3", &modal.content)
                .on_input(|inp| Message::Modal(ModalType::CreateFolder, ModalMessage::Content(inp)))
                .on_submit(Message::Create(false))
                .padding(7)
                .id("create");

            let col = column![
                text(format!(
                    "creating new folder(s) in {}",
                    self.current_path.display()
                )),
                input,
                text("press Esc to exit, Enter to confirm :D")
                    .size(13)
                    .color(palette.primary.scale_alpha(0.4)),
                text(modal.error).color(palette.warning)
            ]
            .width(497)
            .spacing(7);

            let overlay = opaque(float(
                container(col)
                    .style(move |_| palette.background.scale_alpha(0.8).into())
                    .center(Length::Fill),
            ));

            stack = stack.push(overlay);
        }

        stack.into()
    }

    pub fn theme(&self) -> Theme {
        self.theme.clone()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        event::listen_with(move |event, status, _| {
            if status == Status::Captured {
                return Some(Message::FocusModal);
            }

            match event {
                Event::Keyboard(keyboard::Event::ModifiersChanged(state)) => Some(
                    Message::KeyModifiers(state.control(), state.shift(), state.alt()),
                ),

                Event::Keyboard(keyboard::Event::KeyPressed {
                    physical_key,
                    modifiers,
                    ..
                }) => match (physical_key, modifiers) {
                    (key::Physical::Code(Code::Escape), _) => Some(Message::CloseModals),
                    (key::Physical::Code(Code::Enter), _) => Some(Message::SelectChoice),

                    _ => Some(Message::KeyPressed(physical_key, modifiers)),
                    // weird stuff going on with my lsp, why is it lagging so much here?
                },
                _ => None,
            }
        })
    }
}

fn sort(sorting_by: &SortingBy, entries: &mut Vec<Entry>) {
    match sorting_by {
        SortingBy::Name => entries.par_sort_by(|a, b| {
            let (x, y) = (&a.name, &b.name);
            x.cmp(y)
        }),
        SortingBy::Size => entries.par_sort_by(|a, b| {
            let (x, y) = (&a.filesize, &b.filesize);
            x.cmp(y)
        }),
        SortingBy::Type => entries.par_sort_by(|a, b| {
            let (x, y) = (&a.filetype, &b.filetype);
            x.cmp(y)
        }),
        SortingBy::Created => entries.par_sort_by(|a, b| {
            let (x, y) = (&a.created, &b.created);
            x.cmp(y)
        }),
        SortingBy::Accessed => entries.par_sort_by(|a, b| {
            let (x, y) = (&a.accessed, &b.accessed);
            x.cmp(y)
        }),
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
