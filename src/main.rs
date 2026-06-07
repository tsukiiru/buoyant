use std::{
    collections::HashSet,
    env::{args, home_dir},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use chrono::DateTime;

use iced::{
    Background, Border, Color, Element, Event, Length, Padding, Subscription, Task, Theme,
    alignment,
    border::Radius,
    event::{self, Status},
    keyboard::{
        self, Modifiers,
        key::{self, Code, Physical},
    },
    theme,
    widget::{
        button, column, container, float, mouse_area, opaque,
        operation::{self, AbsoluteOffset},
        row,
        scrollable::Viewport,
        selector::Target,
        stack, text, text_input,
    },
};

use iced::widget::text::Wrapping;
use iced::widget::{scrollable, selector};

use config::{Displaying, SortingBy};

mod config;
mod path;
use path as file;
use rayon::slice::ParallelSliceMut;

struct Program {
    path: PathBuf,
}

impl Program {
    fn init(starting_path: PathBuf) -> Self {
        Program {
            path: starting_path,
        }
    }

    fn open(&mut self, new_path: &Path) {
        if new_path.is_dir() {
            self.path = new_path.to_path_buf();
        } else {
            let cmd = Command::new("xdg-open")
                .arg(new_path)
                .stderr(Stdio::null())
                .spawn();

            if let Err(e) = cmd {
                println!("{}", e);
            }
        }
    }

    fn navigate_back(&mut self) {
        self.path.pop();
    }
}

#[derive(Clone, Debug)]
enum ClipboardMode {
    Copy,
    Cut,
}

#[derive(Clone, Debug)]
enum Direction {
    Up,
    Down,
}

#[derive(Clone, Debug)]
enum Message {
    None,
    Open(PathBuf),
    // referencing path here is also possible?
    Return,

    KeyPressed(Physical, Modifiers),

    UpdateEntries(Option<PathBuf>),
    HoverEntry(usize, bool),

    ToggleHiddenView,
    ToggleVisualMode,
    UpdateModifiersState(bool, bool, bool),

    UpdateExplorerScroll(Option<Target>),
    UpdateExplorerOffset(Viewport),

    Select(usize),
    ResetSelection,
    DeleteSelection,
    NavigateSelection(Direction),
    OpenSelection,

    AddClipboard(ClipboardMode),
    PasteClipboard(file::OperationChoice),

    Rename,
    Create(bool),

    UpdateModal(ModalType, ModalMessage),
    CheckModals,
    CloseModals,
    ClearChoices,
    SelectChoice,
    UpdateChoiceIndex(bool),
}

struct Entry {
    id: usize,
    name: String,
    path: PathBuf,

    accessed: i64,
    created: i64,
    filetype: &'static str,
    filesize: u64,

    hidden: bool,
    hovered: bool,
}

struct RenameModal {
    path: PathBuf,
    content: String,
    error: Option<&'static str>,
}

struct CreateModal {
    content: String,
    error: &'static str,
}

struct Clipboard {
    entries: HashSet<PathBuf>,
    mode: ClipboardMode,
}

struct ModifiersState {
    ctrl: bool,
    shift: bool,
    alt: bool,
}

struct ModalState {
    opened: bool,
    operation: bool,
    delete: bool,
    create_file: Option<CreateModal>,
    create_folder: Option<CreateModal>,
    rename: Option<RenameModal>,

    choices: Vec<Message>,
    current_choice: usize,
}

#[derive(Clone, Debug)]
enum ModalType {
    Rename,
    Operation,
    Delete,
    CreateFile,
    CreateFolder,
}

#[derive(Clone, Debug)]
enum ModalMessage {
    Open,
    Close,
    Content(String),
}

struct Entries {
    children: Vec<Entry>,
}

impl Entries {
    fn new() -> Self {
        Entries {
            children: Vec::with_capacity(30),
        }
    }

    fn getv_index(&self, index: &usize) -> Option<&Entry> {
        self.children.get(*index)
    }

    fn get_mut(&mut self, id: &usize) -> Option<&mut Entry> {
        for entry in &mut self.children {
            if entry.id == *id {
                return Some(entry);
            }
        }
        None
    }

    fn get_index(&self, id: &usize) -> usize {
        let mut res: usize = 0;

        self.children.iter().enumerate().for_each(|(index, entry)| {
            if entry.id == *id {
                res = index;
            }
        });

        res
    }
}

struct Application {
    config: config::Config,
    program: Program,

    current_index: Option<usize>,
    entries: Entries,
    selected: HashSet<usize>,

    explorer_offset: f32,

    modifiers_state: ModifiersState,
    clipboard: Clipboard,
    modals_state: ModalState,

    visual_mode: bool,
}

impl Application {
    fn new(input: &str, config: config::Config) -> (Self, Task<Message>) {
        let path_conversion = PathBuf::from(input);
        let path: PathBuf;

        if !path_conversion.exists() {
            path = home_dir().unwrap();
        } else {
            path = path_conversion;
        }

        (
            Application {
                config,

                program: Program::init(path),

                current_index: None,
                entries: Entries::new(),
                selected: HashSet::new(),

                explorer_offset: 0.0,

                modifiers_state: ModifiersState {
                    ctrl: false,
                    shift: false,
                    alt: false,
                },
                clipboard: Clipboard {
                    entries: HashSet::new(),
                    mode: ClipboardMode::Copy,
                },
                modals_state: ModalState {
                    opened: false,
                    operation: false,
                    delete: false,
                    rename: None,
                    create_file: None,
                    create_folder: None,

                    choices: Vec::with_capacity(2),
                    current_choice: 0,
                },

                visual_mode: false,
            },
            Task::done(Message::UpdateEntries(None)),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::None => Task::none(),
            Message::Open(path) => {
                self.program.open(&path);

                if path.is_file() {
                    return Task::none();
                }

                self.current_index = None;
                Task::done(Message::UpdateEntries(None))
            }
            Message::Return => {
                if self.modals_state.opened {
                    return Task::none();
                }

                let path = Some(self.program.path.clone());
                self.program.navigate_back();

                Task::done(Message::UpdateEntries(path)).chain(
                    selector::find(selector::id("scrollable"))
                        .then(|output| Task::done(Message::UpdateExplorerScroll(output))),
                )
            }

            Message::KeyPressed(physical_key, modifiers) => {
                let keybinds = &self.config.keybinds;

                if physical_key == keybinds.navigate_backward.key
                    && modifiers == keybinds.navigate_backward.modifiers
                {
                    // for navigating up the directory tree AND going changing modal selection index
                    // - 1
                    return Task::done(Message::UpdateChoiceIndex(false))
                        .chain(Task::done(Message::Return));
                } else if physical_key == keybinds.navigate_forward.key
                    && modifiers == keybinds.navigate_forward.modifiers
                {
                    return Task::done(Message::UpdateChoiceIndex(true))
                        .chain(Task::done(Message::OpenSelection));
                } else if physical_key == keybinds.navigate_down.key
                    && modifiers == keybinds.navigate_down.modifiers
                {
                    return Task::done(Message::NavigateSelection(Direction::Down));
                } else if physical_key == keybinds.navigate_up.key
                    && modifiers == keybinds.navigate_up.modifiers
                {
                    return Task::done(Message::NavigateSelection(Direction::Up));
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
                    return Task::done(Message::UpdateModal(
                        ModalType::Operation,
                        ModalMessage::Open,
                    ));
                } else if physical_key == keybinds.delete_selections.key
                    && modifiers == keybinds.delete_selections.modifiers
                {
                    return Task::done(Message::UpdateModal(ModalType::Delete, ModalMessage::Open));
                } else if physical_key == keybinds.rename_file.key
                    && modifiers == keybinds.rename_file.modifiers
                {
                    return Task::done(Message::UpdateModal(ModalType::Rename, ModalMessage::Open));
                } else if physical_key == keybinds.toggle_hidden_view.key
                    && modifiers == keybinds.toggle_hidden_view.modifiers
                {
                    return Task::done(Message::ToggleHiddenView);
                } else if physical_key == keybinds.create_file_path.key
                    && modifiers == keybinds.create_file_path.modifiers
                {
                    return Task::done(Message::UpdateModal(
                        ModalType::CreateFile,
                        ModalMessage::Open,
                    ));
                } else if physical_key == keybinds.create_folder_path.key
                    && modifiers == keybinds.create_folder_path.modifiers
                {
                    return Task::done(Message::UpdateModal(
                        ModalType::CreateFolder,
                        ModalMessage::Open,
                    ));
                }
                if physical_key == keybinds.toggle_visual_mode.key
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

                let cur_paths = file::read_dir(&self.program.path);
                let mut i: usize = 0;

                let entries = &mut self.entries.children;
                let mut non_hidden_entries: Vec<Entry> = Vec::with_capacity(30);

                for path in cur_paths {
                    let is_hidden = file::is_hidden(&path);

                    if !self.config.view_hidden && is_hidden {
                        continue;
                    }

                    let entry = Entry {
                        name: path.file_name().unwrap().to_str().unwrap().to_string(),
                        path: path.clone(),

                        created: file::get_filecreated(&path),
                        accessed: file::get_fileaccessed(&path),

                        id: i,

                        filetype: file::get_filetype(&path),
                        filesize: file::get_filesize(&path),

                        hovered: false,
                        hidden: is_hidden,
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
                            self.current_index = Some(self.entries.get_index(&entry.id));
                        }
                    });
                }

                Task::none()
            }
            Message::HoverEntry(id, state) => {
                let entry = self.entries.get_mut(&id);

                if let Some(e) = entry {
                    e.hovered = state;
                }

                Task::none()
            }

            Message::ToggleHiddenView => {
                self.config.view_hidden = !self.config.view_hidden;
                Task::done(Message::UpdateEntries(None))
            }
            Message::ToggleVisualMode => {
                self.visual_mode = !self.visual_mode;
                Task::none()
            }

            Message::UpdateModifiersState(ctrl_state, shift_state, alt_state) => {
                let modifiers_state = &mut self.modifiers_state;

                modifiers_state.ctrl = ctrl_state;
                modifiers_state.shift = shift_state;
                modifiers_state.alt = alt_state;

                Task::none()
            }

            Message::UpdateExplorerScroll(target) => {
                // consutrction
                let widget = target.unwrap();
                let height = widget.visible_bounds().unwrap().height;
                //
                let try_current_index = self.current_index;

                if try_current_index.is_none() {
                    return Task::none();
                }

                let current_index: f32 = try_current_index.unwrap() as f32 + 1.0;
                let offset: f32 = 40.0 * (current_index - 1.0);

                let widget_range = (self.explorer_offset, self.explorer_offset + height);

                //println!("range is {:#?}", widget_range);
                //               println!("while the offset is {}", offset);

                if offset <= widget_range.0 {
                    return operation::scroll_to(
                        "scrollable",
                        AbsoluteOffset { x: 0.0, y: offset },
                    );
                }

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
            Message::UpdateExplorerOffset(viewport) => {
                self.explorer_offset = viewport.absolute_offset().y;
                //          println!("current offset is {}", self.explorer_offset);
                Task::none()
            }

            Message::Select(index) => {
                if !self.modifiers_state.shift && !self.visual_mode && !self.modifiers_state.ctrl {
                    self.selected.clear();
                }

                let end_index = if let Some(i) = self.current_index
                    && (self.modifiers_state.shift || self.visual_mode)
                {
                    i
                } else {
                    index
                };

                for i in index.min(end_index)..=end_index.max(index) {
                    self.selected.insert(i);
                } // selecting everything between the two indicies

                if self.modifiers_state.ctrl {
                    if self.selected.contains(&index) {
                        self.selected.remove(&index);
                    } else {
                        self.selected.insert(index);
                    }
                }

                self.current_index = Some(index);

                selector::find(selector::id("scrollable"))
                    .then(|output| Task::done(Message::UpdateExplorerScroll(output)))
            }
            Message::ResetSelection => {
                self.selected.clear();
                Task::none()
            }
            Message::DeleteSelection => {
                for index in &self.selected {
                    let try_getentry = &self.entries.getv_index(index);

                    if let Some(entry) = try_getentry {
                        file::delete(&entry.path);
                    } else {
                        println!(
                            "encountered some error while trying to get entry from index {}",
                            index
                        );
                    }
                }

                Task::done(Message::UpdateModal(ModalType::Delete, ModalMessage::Close))
                    .chain(Task::done(Message::UpdateEntries(None)))
            }
            Message::NavigateSelection(direction) => {
                if self.modals_state.opened {
                    return Task::none();
                }

                let index_opt = self.current_index.as_mut();
                let mut current_index: usize = 0;

                if index_opt.is_none() {
                    return Task::done(Message::Select(0));
                } else if let Some(thing) = index_opt {
                    current_index = *thing;
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
            Message::OpenSelection => {
                if self.modals_state.opened {
                    return Task::none();
                }

                let current_index = self.current_index;
                let temp_index: usize;

                if let Some(index) = &current_index {
                    temp_index = *index;
                } else {
                    return Task::none();
                }

                if let Some(entry) = self.entries.getv_index(&temp_index) {
                    Task::done(Message::Open(entry.path.clone()))
                } else {
                    Task::none()
                }
            }

            Message::AddClipboard(mode) => {
                if self.modals_state.opened || self.selected.is_empty() {
                    return Task::none();
                }

                let clipboard = &mut self.clipboard;

                clipboard.entries.clear();

                self.selected.iter().for_each(|i| {
                    let _ = clipboard
                        .entries
                        .insert(self.entries.getv_index(i).unwrap().path.clone());
                });

                clipboard.mode = mode;

                Task::none()
            }
            Message::PasteClipboard(opp) => {
                let clipboard = &mut self.clipboard;

                if clipboard.entries.is_empty() {
                    return Task::none();
                }

                match clipboard.mode {
                    ClipboardMode::Copy => {
                        file::copy_dir(&clipboard.entries, &self.program.path, &opp);
                    }
                    ClipboardMode::Cut => {
                        file::move_dir(&clipboard.entries, &self.program.path, &opp);
                        clipboard.entries.clear();
                    }
                }

                Task::done(Message::UpdateModal(
                    ModalType::Operation,
                    ModalMessage::Close,
                ))
                .chain(Task::done(Message::UpdateEntries(None)))
            }

            Message::Rename => {
                let overlay = self.modals_state.rename.as_mut().unwrap();
                let name = &overlay.content;

                if name.is_empty() {
                    return Task::none();
                }

                // checking if the new name is valid?
                for char in file::NONO_CHARACTERS {
                    if name.contains(char) {
                        overlay.error = Some("name cannot contain invalid characters!");
                        return Task::none();
                    }
                }

                let mut test_path = overlay.path.clone();
                test_path.set_file_name(name);

                // check if already exists in destination
                if test_path.exists() {
                    overlay.error = Some("ERROR: file with the same name already exists");
                    return Task::none();
                }

                file::rename(&mut overlay.path, name);
                Task::done(Message::UpdateModal(ModalType::Rename, ModalMessage::Close))
                    .chain(Task::done(Message::UpdateEntries(None)))
            }
            Message::Create(mode) => {
                // true if creating file, else creating folder
                if mode {
                    let overlay = self.modals_state.create_file.as_mut().unwrap();

                    let err =
                        file::create(&self.program.path, Path::new(overlay.content.trim()), true);

                    if let Some(e) = err {
                        overlay.error = e;
                    } else {
                        return Task::batch(vec![
                            Task::done(Message::UpdateModal(
                                ModalType::CreateFile,
                                ModalMessage::Close,
                            )),
                            Task::done(Message::UpdateEntries(None)),
                        ]);
                    }
                } else {
                    let overlay = self.modals_state.create_folder.as_mut().unwrap();

                    let err =
                        file::create(&self.program.path, Path::new(overlay.content.trim()), false);
                    if let Some(e) = err {
                        overlay.error = &e;
                    } else {
                        return Task::batch(vec![
                            Task::done(Message::UpdateEntries(None)),
                            Task::done(Message::UpdateModal(
                                ModalType::CreateFolder,
                                ModalMessage::Close,
                            )),
                        ]);
                    }
                }

                Task::none()
            }

            Message::UpdateModal(modal_type, msg) => {
                let modals_state = &mut self.modals_state;
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
                                    let selected = self.entries.getv_index(&index).unwrap();

                                    modals_state.rename = Some(RenameModal {
                                        path: selected.path.clone(),
                                        content: selected.name.clone(),
                                        error: None,
                                    }) // TODO: reference path here is possible?
                                }
                                *modals_opened = true;

                                return Task::batch(vec![operation::focus("rename")]);
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
                                modals_state.delete = true;
                                *modals_opened = true;

                                self.modals_state.choices.push(Message::DeleteSelection);
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
                    ModalType::Operation => {
                        match msg {
                            ModalMessage::Open => {
                                if self.clipboard.entries.is_empty() {
                                    return Task::none();
                                }

                                modals_state.operation = true;
                                *modals_opened = true;

                                self.modals_state.choices.extend(vec![
                                    Message::PasteClipboard(path::OperationChoice::Merge),
                                    Message::PasteClipboard(path::OperationChoice::Duplicate),
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

                                return Task::batch(vec![operation::focus("create")]);
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

                                return Task::batch(vec![operation::focus("create")]);
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
            Message::CheckModals => {
                let mut task = Task::none();

                task = task.chain(operation::is_focused("rename").then(|focused| {
                    if !focused {
                        return Task::done(Message::UpdateModal(
                            ModalType::Rename,
                            ModalMessage::Close,
                        ));
                    } else {
                        return Task::none();
                    }
                }));

                task = task.chain(operation::is_focused("create").then(|focused| {
                    if !focused {
                        return Task::batch(vec![
                            Task::done(Message::UpdateModal(
                                ModalType::CreateFile,
                                ModalMessage::Close,
                            )),
                            Task::done(Message::UpdateModal(
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
                if !self.modals_state.opened {
                    return Task::none();
                }

                Task::batch(vec![
                    Task::done(Message::UpdateModal(
                        ModalType::Operation,
                        ModalMessage::Close,
                    )),
                    Task::done(Message::UpdateModal(ModalType::Delete, ModalMessage::Close)),
                    Task::done(Message::UpdateModal(ModalType::Rename, ModalMessage::Close)),
                    Task::done(Message::UpdateModal(
                        ModalType::CreateFile,
                        ModalMessage::Close,
                    )),
                    Task::done(Message::UpdateModal(
                        ModalType::CreateFolder,
                        ModalMessage::Close,
                    )),
                    Task::done(Message::ClearChoices),
                ])
            }
            Message::ClearChoices => {
                self.modals_state.choices.clear();
                self.modals_state.current_choice = 0;

                Task::none()
            }
            Message::SelectChoice => {
                let choice = self
                    .modals_state
                    .choices
                    .get(self.modals_state.current_choice);

                if let Some(decision) = choice
                    && self.modals_state.opened
                {
                    // clone is fine here since its a enum (i think (i hope :pray:))
                    return Task::done(decision.clone());
                }
                Task::none()
            }
            Message::UpdateChoiceIndex(right) => {
                // fuck i dont like the look of this at all

                if self.modals_state.choices.len() == 0 {
                    return Task::none();
                }

                let cur_choice = self.modals_state.current_choice as i8;
                let dir = if right { 1 } else { -1 };
                let new_index =
                    (cur_choice + dir).clamp(0, (self.modals_state.choices.len() - 1) as i8);

                self.modals_state.current_choice = new_index as usize;

                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let mut buttons = column![];
        let entries = &self.entries.children;

        let palette = theme::Theme::KanagawaLotus.palette();
        let text_color = palette.text;
        let darken_text_color = text_color.scale_alpha(0.69);

        buttons = buttons
            .extend(
                entries
                    .iter()
                    .map(|e| {
                        let mut row = row![].spacing(10);

                        for child in &self.config.view {
                            match child {
                                Displaying::Name => {
                                    row = row.push(
                                        container(
                                            text(&e.name)
                                                .wrapping(Wrapping::None)
                                                .align_x(alignment::Horizontal::Left)
                                                .color(if e.hidden {
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
                                            text(file::convert_bytes_to_string(&e.filesize))
                                                .align_x(alignment::Horizontal::Left)
                                                .wrapping(Wrapping::None)
                                                .color(if e.hidden {
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
                                            text(e.filetype)
                                                .align_x(alignment::Horizontal::Left)
                                                .wrapping(Wrapping::None)
                                                .color(if e.hidden {
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
                                    row =
                                        row.push(
                                            container(
                                                text(
                                                    DateTime::from_timestamp_secs(e.created)
                                                        .unwrap()
                                                        .to_string(),
                                                )
                                                .align_x(alignment::Horizontal::Left)
                                                .wrapping(Wrapping::None)
                                                .color(if e.hidden {
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
                                    row =
                                        row.push(
                                            container(
                                                text(
                                                    DateTime::from_timestamp_secs(e.accessed)
                                                        .unwrap()
                                                        .to_string(),
                                                )
                                                .align_x(alignment::Horizontal::Left)
                                                .wrapping(Wrapping::None)
                                                .color(if e.hidden {
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

                        let index = self.entries.get_index(&e.id);

                        container(
                            mouse_area(row)
                                .on_double_click(Message::Open(e.path.clone()))
                                .on_press(Message::Select(index))
                                .on_enter(Message::HoverEntry(e.id, true))
                                .on_exit(Message::HoverEntry(e.id, false)),
                        )
                        .center_y(30)
                        .padding(Padding::from([0, 5]))
                        .style(|_| {
                            let mut style = container::Style::default();
                            let index = self.entries.get_index(&e.id);

                            if let Some(cur_index) = self.current_index
                                && cur_index == index
                            {
                                style.border = Border {
                                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                                    width: 2.0,
                                    radius: Radius::new(4.0),
                                };
                            }

                            if e.hovered {
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

        let explorer_scroll = scrollable(buttons)
            .id("scrollable")
            .width(Length::Fill)
            .height(Length::Fill)
            .on_scroll(Message::UpdateExplorerOffset);

        let mut row = row![].spacing(10).padding(5);

        for child in &self.config.view {
            match child {
                Displaying::Name => {
                    row = row.push(
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
                    row = row.push(
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
                    row = row.push(
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
                    row = row.push(
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
                    row = row.push(
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

        let explorer_select = container(
            column![
                row,
                mouse_area(explorer_scroll).on_press(
                    if !self.modifiers_state.ctrl || !self.visual_mode {
                        Message::ResetSelection
                    } else {
                        Message::None
                    },
                )
            ]
            .spacing(10),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(20);

        let clipboard_mode = match self.clipboard.mode {
            ClipboardMode::Copy => "Copy",
            ClipboardMode::Cut => "Cut",
        };

        let clipboard_entries = &self.clipboard.entries;
        let clipboard: Element<Message> = column![text(format!("type: {}", clipboard_mode))]
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
                button(text("....")).on_press(Message::Return),
                container(text(format!("{}", self.program.path.display())))
                    .style(|_| {
                        container::Style::default()
                            .background(Background::Color(Color::from_rgba(0.8, 0.8, 0.8, 0.8)))
                    })
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

        if self.visual_mode {
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
            let entry = self.entries.getv_index(&index).unwrap();

            file_info = file_info.extend(vec![
                text(format!("name: {}", entry.name)).into(),
                text(format!("type: {}", entry.filetype)).into(),
                text(format!(
                    "size: {}",
                    file::convert_bytes_to_string(&entry.filesize)
                ))
                .into(),
                text(format!(
                    "last accessed: {}",
                    DateTime::from_timestamp_secs(entry.accessed)
                        .unwrap()
                        .to_string()
                ))
                .into(),
                text(format!(
                    "creation date: {}",
                    DateTime::from_timestamp_secs(entry.created)
                        .unwrap()
                        .to_string()
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

        if let Some(thing) = &self.modals_state.rename {
            let input = text_input("input the new name here :3", &thing.content)
                .on_input(|inp| Message::UpdateModal(ModalType::Rename, ModalMessage::Content(inp)))
                .on_submit(Message::Rename)
                .padding(7)
                .id("rename");

            let mut col = column![
                text(format!("you are renaming, {}", thing.path.display())),
                input,
            ]
            .width(497)
            .spacing(7);

            if let Some(th) = thing.error {
                col = col.push(
                    text(th)
                        .color(Color::from_rgba(1.0, 105.0 / 255.0, 97.0 / 255.0, 1.0))
                        .size(13),
                );
            }

            let overlay = opaque(float(
                container(col).style(move |_| style()).center(Length::Fill),
            ));

            stack = stack.push(overlay);
        }

        if self.modals_state.operation {
            let row = row![
                button(text("Replace \nreplace file if name is matched"))
                    .on_press(Message::PasteClipboard(file::OperationChoice::Merge))
                    .padding(7)
                    .style(|theme: &Theme, _| {
                        let mut style = button::Style::default();
                        let palette = theme.palette();

                        if self.modals_state.current_choice == 0 {
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
                .on_press(Message::PasteClipboard(file::OperationChoice::Duplicate))
                .padding(7)
                .style(|theme: &Theme, _| {
                    let mut style = button::Style::default();
                    let palette = theme.palette();

                    if self.modals_state.current_choice == 1 {
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
                container(column![text("choose an operation type"), row].spacing(10))
                    .style(move |_| style())
                    .center(Length::Fill),
            ));

            stack = stack.push(overlay);
        }

        if self.modals_state.delete {
            let overlay = opaque(float(
                container(
                    column![
                        text("you gonna delete the selections?"),
                        button(text("yeah :3"))
                            .padding(7)
                            .style(|theme: &Theme, _| {
                                let mut style = button::Style::default();
                                let palette = theme.palette();

                                if self.modals_state.current_choice == 0 {
                                    style.border = Border {
                                        color: palette.warning,
                                        width: 2.0,
                                        radius: Radius::new(8.0),
                                    }
                                }
                                style
                            })
                            .on_press(Message::DeleteSelection)
                    ]
                    .spacing(10),
                )
                .style(move |_| style())
                .center(Length::Fill),
            ));

            stack = stack.push(overlay);
        }

        if let Some(thing) = &self.modals_state.create_file {
            let input = text_input("input the file path here! :3", &thing.content)
                .on_input(|inp| {
                    Message::UpdateModal(ModalType::CreateFile, ModalMessage::Content(inp))
                })
                .on_submit(Message::Create(true))
                .padding(7)
                .id("create");

            let col = column![
                text(format!(
                    "creating a new file in {}",
                    self.program.path.display()
                )),
                input,
                text(thing.error).color(Color::from_rgba(1.0, 105.0 / 255.0, 97.0 / 255.0, 1.0))
            ]
            .width(497)
            .spacing(7);

            let overlay = opaque(float(
                container(col).style(move |_| style()).center(Length::Fill),
            ));

            stack = stack.push(overlay);
        }

        if let Some(thing) = &self.modals_state.create_folder {
            let input = text_input("input the folder path here! :3", &thing.content)
                .on_input(|inp| {
                    Message::UpdateModal(ModalType::CreateFolder, ModalMessage::Content(inp))
                })
                .on_submit(Message::Create(false))
                .padding(7)
                .id("create");

            let col = column![
                text(format!(
                    "creating new folder(s) in {}",
                    self.program.path.display()
                )),
                input,
                text(thing.error).color(Color::from_rgba(1.0, 105.0 / 255.0, 97.0 / 255.0, 1.0))
            ]
            .width(497)
            .spacing(7);

            let overlay = opaque(float(
                container(col).style(move |_| style()).center(Length::Fill),
            ));

            stack = stack.push(overlay);
        }

        stack.into()
    }

    fn subscription(&self) -> Subscription<Message> {
        event::listen_with(move |event, status, _| {
            if status == Status::Captured {
                return Some(Message::CheckModals);
            }

            match event {
                Event::Keyboard(keyboard::Event::ModifiersChanged(state)) => Some(
                    Message::UpdateModifiersState(state.control(), state.shift(), state.alt()),
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
            let (x, y) = (a.name.as_str(), b.name.as_str());
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

fn style() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.95))),
        ..Default::default()
    }
}

fn main() -> iced::Result {
    let input = args().nth(1).unwrap_or_default();

    iced::application(
        move || Application::new(&input, config::get_keybinds()),
        Application::update,
        Application::view,
    )
    .subscription(Application::subscription)
    .title("buoyant")
    .theme(Theme::KanagawaLotus)
    .run()
}
