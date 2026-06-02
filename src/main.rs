use std::{
    collections::HashSet,
    env::{args, home_dir},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use chrono::DateTime;

use iced::{
    Background, Border, Color, Element, Event, Length, Subscription, Task, alignment,
    border::Radius,
    event::{self, Status},
    keyboard::{
        self, Modifiers,
        key::{self, Code, Physical},
    },
    widget::{
        button, column, container, float, mouse_area, opaque, operation, row, scrollable, stack,
        text, text_input,
    },
};

use config::SortingBy;

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
    Return,

    KeyPressed(Physical, Modifiers),

    UpdateEntries(Option<PathBuf>),
    HoverEntry(usize, bool),

    ToggleHiddenView,
    ToggleVisualMode,
    UpdateModifiersState(bool, bool, bool),

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
}

struct Entry {
    id: usize,
    name: String,
    path: PathBuf,

    accessed: i64,
    created: i64,
    filetype: &'static str,
    filesize: u64,

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

struct ModalsState {
    opened: bool,
    rename: Option<RenameModal>,
    operation: bool,
    delete: bool,
    create_file: Option<CreateModal>,
    create_folder: Option<CreateModal>,
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

struct Displaying {
    hidden: bool,
    last_accessed: bool,
    created: bool,
    filetype: bool,
    filesize: bool,
}

impl Default for Displaying {
    fn default() -> Self {
        Displaying {
            hidden: false,
            last_accessed: false,
            created: false,
            filetype: true,
            filesize: true,
        }
    }
}

struct Entries {
    children: Vec<Entry>,
}

impl Entries {
    fn new() -> Self {
        Entries {
            children: Vec::new(),
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

    modifiers_state: ModifiersState,
    clipboard: Clipboard,
    modals_state: ModalsState,

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

                modifiers_state: ModifiersState {
                    ctrl: false,
                    shift: false,
                    alt: false,
                },
                clipboard: Clipboard {
                    entries: HashSet::new(),
                    mode: ClipboardMode::Copy,
                },
                modals_state: ModalsState {
                    opened: false,
                    rename: None,
                    operation: false,
                    delete: false,
                    create_file: None,
                    create_folder: None,
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

                Task::done(Message::UpdateEntries(path))
            }

            Message::KeyPressed(physical_key, modifiers) => {
                let keybinds = &self.config.keybinds;

                if physical_key == keybinds.navigate_backward.key
                    && modifiers == keybinds.navigate_backward.modifiers
                {
                    return Task::done(Message::Return);
                } else if physical_key == keybinds.navigate_forward.key
                    && modifiers == keybinds.navigate_forward.modifiers
                {
                    return Task::done(Message::OpenSelection);
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

                let cur_paths = file::read_dir(&self.program.path);
                let mut i: usize = 0;

                for path in cur_paths {
                    if !self.config.view.hidden && file::is_hidden(&path) {
                        continue;
                    }

                    self.entries.children.push(Entry {
                        name: path.file_name().unwrap().to_str().unwrap().to_string(),
                        path: path.clone(),
                        created: file::get_filecreated(&path),
                        accessed: file::get_fileaccessed(&path),
                        id: i,
                        hovered: false,
                        filetype: file::get_filetype(&path),
                        filesize: file::get_filesize(&path),
                    });

                    i += 1;
                }

                // sorting logic
                let sorting_config = &self.config.sorting;
                let entries = &mut self.entries.children;

                match sorting_config.sorting_by {
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

                if sorting_config.reversed {
                    self.entries.children.reverse();
                }

                if let Some(path) = prev_path {
                    self.entries.children.iter().for_each(|entry| {
                        if entry.path == path {
                            self.current_index = Some(self.entries.get_index(&entry.id));
                        }
                    });
                } else {
                    self.selected.clear();
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
                self.config.view.hidden = !self.config.view.hidden;
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

                Task::none()
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

                // TODO: update position of view following the selection index, bro this is hard
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
                                    })
                                }
                                *modals_opened = true;

                                return Task::batch(vec![operation::focus("rename")]);
                            }
                            ModalMessage::Close => {
                                modals_state.rename = None;
                                *modals_opened = false;
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
                            }
                            ModalMessage::Close => {
                                modals_state.delete = false;
                                *modals_opened = false;
                            }
                            _ => {}
                        }
                        Task::none()
                    }
                    ModalType::Operation => {
                        match msg {
                            ModalMessage::Open => {
                                modals_state.operation = true;
                                *modals_opened = true;
                            }
                            ModalMessage::Close => {
                                modals_state.operation = false;
                                *modals_opened = false;
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
                            }
                        }
                        Task::none()
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

                                return Task::batch(vec![operation::focus("create")]);
                            }
                        }
                        Task::none()
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
                ])
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let mut buttons = column![];
        let display_conf = &self.config.view;

        let mut row = row![
            text("file name")
                .width(300)
                .align_x(alignment::Horizontal::Left)
        ]
        .spacing(5)
        .padding(5);

        if display_conf.filesize {
            row = row.push(text("size").width(100).align_x(alignment::Horizontal::Left));
        }

        if display_conf.filetype {
            row = row.push(text("type").width(150).align_x(alignment::Horizontal::Left));
        }

        if display_conf.created {
            row = row.push(
                text("creation date")
                    .width(200)
                    .align_x(alignment::Horizontal::Left),
            );
        }

        if display_conf.last_accessed {
            row = row.push(text("accessed date").align_x(alignment::Horizontal::Left));
        }

        buttons = buttons.push(row);

        let entries = &self.entries.children;

        buttons = buttons
            .extend(
                entries
                    .iter()
                    .map(|e| {
                        let mut row = row![
                            text(&e.name)
                                .width(300)
                                .align_x(alignment::Horizontal::Left),
                        ]
                        .spacing(5)
                        .padding(5);

                        if display_conf.filesize {
                            row = row.push(
                                text(file::convert_bytes_to_string(&e.filesize))
                                    .width(100)
                                    .align_x(alignment::Horizontal::Left),
                            );
                        }

                        if display_conf.filetype {
                            row = row.push(
                                text(e.filetype)
                                    .width(150)
                                    .align_x(alignment::Horizontal::Left),
                            );
                        }

                        if display_conf.created {
                            row = row.push(
                                text(
                                    DateTime::from_timestamp_secs(e.created)
                                        .unwrap()
                                        .to_string(),
                                )
                                .width(200)
                                .align_x(alignment::Horizontal::Left),
                            );
                        }

                        if display_conf.last_accessed {
                            row = row.push(
                                text(
                                    DateTime::from_timestamp_secs(e.accessed)
                                        .unwrap()
                                        .to_string(),
                                )
                                .align_x(alignment::Horizontal::Left),
                            );
                        }

                        let index = self.entries.get_index(&e.id);

                        container(
                            mouse_area(row)
                                .on_double_click(Message::Open(e.path.clone()))
                                .on_press(Message::Select(index))
                                .on_enter(Message::HoverEntry(e.id, true))
                                .on_exit(Message::HoverEntry(e.id, false)),
                        )
                        .style(|_theme| {
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
            .padding(20)
            .width(Length::Fill);

        let explorer_scroll = scrollable(buttons)
            .id("scrollable")
            .width(Length::Fill)
            .height(Length::Fill);

        let explorer_select = container(mouse_area(explorer_scroll).on_press(
            if !self.modifiers_state.ctrl || !self.visual_mode {
                Message::ResetSelection
            } else {
                Message::None
            },
        ))
        .width(Length::Fill)
        .height(Length::Fill);

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
                    .style(|_theme| {
                        container::Style::default()
                            .background(Background::Color(Color::from_rgba(0.8, 0.8, 0.8, 0.8)))
                    })
                    .center_y(30)
                    .center_x(Length::Fill)
                    .height(30)
                    .padding(5),
            ],
            explorer_select
        ]
        .spacing(10)
        .height(Length::Fill)
        .width(Length::Fill);

        let mut right_col = column![
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
        .width(300)
        .spacing(10);

        if self.visual_mode {
            right_col = right_col.push(text("VISUAL MODE").height(20).width(Length::Fill));
        }

        if self.config.view.hidden {
            right_col = right_col.push(
                text(format!("showing hidden files",))
                    .height(20)
                    .width(Length::Fill),
            );
        }

        right_col = right_col.push(clipboard);

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
                    .padding(7),
                button(text(
                    "Duplicate \nadd (n) to the end of file name if name is matched"
                ))
                .on_press(Message::PasteClipboard(file::OperationChoice::Duplicate))
                .padding(7)
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
                    _ => Some(Message::KeyPressed(physical_key, modifiers)),
                },
                _ => None,
            }
        })
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
    .run()
}
