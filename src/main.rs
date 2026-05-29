use std::{
    env::{args, home_dir},
    path::PathBuf,
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

mod config;
mod path;
use path as file;

struct Program {
    path: PathBuf,
}

impl Program {
    fn init(starting_path: PathBuf) -> Self {
        Program {
            path: starting_path,
        }
    }

    fn open(&mut self, new_path: &PathBuf) {
        if new_path.is_dir() {
            self.path = new_path.clone();
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
    UpdateModifiersState(bool, bool),

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
    name: String,
    path: PathBuf,
    accessed: i64,
    created: i64,
    index: usize,
    hovered: bool,
    filetype: String,
}

struct RenameModal {
    path: PathBuf,
    content: String,
    error: Option<String>,
}

struct CreateModal {
    content: String,
    error: String,
}

struct Clipboard {
    entries: Vec<PathBuf>,
    mode: ClipboardMode,
}

struct ModifiersState {
    ctrl: bool,
    shift: bool,
}

struct ModalsState {
    opened: bool,
    rename: Option<RenameModal>,
    operation: Option<bool>,
    delete: Option<bool>,
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

struct Application {
    view_hidden: bool,
    config: config::Config,

    program: Program,

    current_index: Option<usize>,
    entries: Vec<Entry>,
    selected: Vec<usize>,

    modifiers_state: ModifiersState,
    clipboard: Clipboard,
    modals_state: ModalsState,
}

impl Application {
    fn new(input: String, config: config::Config) -> (Self, Task<Message>) {
        let path_conversion = PathBuf::from(input);
        let path: PathBuf;

        if !path_conversion.exists() {
            path = home_dir().unwrap();
        } else {
            path = path_conversion;
        }

        (
            Application {
                view_hidden: false,
                config,

                program: Program::init(path),

                current_index: None,
                entries: vec![],
                selected: vec![],

                modifiers_state: ModifiersState {
                    ctrl: false,
                    shift: false,
                },
                clipboard: Clipboard {
                    entries: vec![],
                    mode: ClipboardMode::Copy,
                },
                modals_state: ModalsState {
                    opened: false,
                    rename: None,
                    operation: None,
                    delete: None,
                    create_file: None,
                    create_folder: None,
                },
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

            Message::KeyPressed(physical_key, _modifiers) => {
                let keybinds_config = &self.config.keybinds;

                if physical_key == keybinds_config.navigate_backward {
                    return Task::done(Message::Return);
                } else if physical_key == keybinds_config.navigate_forward {
                    return Task::done(Message::OpenSelection);
                } else if physical_key == keybinds_config.navigate_down {
                    return Task::done(Message::NavigateSelection(Direction::Down));
                } else if physical_key == keybinds_config.navigate_up {
                    return Task::done(Message::NavigateSelection(Direction::Up));
                }

                Task::none()
            }

            Message::UpdateEntries(prev_path) => {
                self.entries.clear();

                let cur_paths = file::read_dir(&self.program.path);
                let mut i: usize = 0;
                for path in cur_paths {
                    if !self.view_hidden && file::is_hidden(&path) {
                        continue;
                    }

                    self.entries.push(Entry {
                        name: path.file_name().unwrap().to_str().unwrap().to_string(),
                        path: path.clone(),
                        created: file::get_filecreated(&path),
                        accessed: file::get_fileaccessed(&path),
                        index: i,
                        hovered: false,
                        filetype: file::get_filetype(&path),
                    });
                    i += 1;
                }

                if let Some(path) = prev_path {
                    self.entries.iter().for_each(|entry| {
                        if entry.path == path {
                            self.current_index = Some(entry.index);
                        }
                    });
                } else {
                    self.selected.clear();
                }

                Task::none()
            }
            Message::HoverEntry(id, state) => {
                let entry = self.entries.get_mut(id);

                if let Some(e) = entry {
                    e.hovered = state;
                }

                Task::none()
            }

            Message::ToggleHiddenView => {
                self.view_hidden = !self.view_hidden;
                Task::done(Message::UpdateEntries(None))
            }
            Message::UpdateModifiersState(ctrl_state, shift_state) => {
                let modifiers_state = &mut self.modifiers_state;

                modifiers_state.ctrl = ctrl_state;
                modifiers_state.shift = shift_state;
                Task::none()
            }

            Message::Select(index) => {
                if !self.modifiers_state.shift {
                    self.selected.clear();
                }

                let end_index = if let Some(i) = self.current_index
                    && self.modifiers_state.shift
                {
                    i
                } else {
                    index
                };

                for i in index.min(end_index)..=end_index.max(index) {
                    self.selected.push(i);
                } // selecting everything between the two indicies

                self.current_index = Some(index);

                Task::none()
            }
            Message::ResetSelection => {
                self.selected.clear();
                Task::none()
            }
            Message::DeleteSelection => {
                for index in &self.selected {
                    file::delete(&self.entries[*index].path);
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
                        if current_index < self.entries.len() - 1 {
                            current_index += 1;
                        }
                    }
                    Direction::Up => {
                        if !(current_index == 0) {
                            current_index -= 1;
                        }
                    }
                }

                // println!("{}", current_index);

                // TODO: update position of view following the selection index
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

                if let Some(entry) = self.entries.get(temp_index) {
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

                self.selected
                    .iter()
                    .for_each(|i| clipboard.entries.push(self.entries[*i].path.clone()));

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
                        file::copy_dir(clipboard.entries.clone(), self.program.path.clone(), &opp);
                    }
                    ClipboardMode::Cut => {
                        file::move_dir(clipboard.entries.clone(), self.program.path.clone(), &opp);
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
                        overlay.error =
                            Some(String::from(format!("ERROR: name cannot contain {}", char)));
                        return Task::none();
                    }
                }

                let mut test_path = overlay.path.clone();
                test_path.set_file_name(name);

                // check if already exists in destination
                if test_path.exists() {
                    overlay.error = Some(String::from(
                        "ERROR: file with the same name already exists",
                    ));
                    return Task::none();
                }

                file::rename(&mut overlay.path, name.clone());
                Task::done(Message::UpdateModal(ModalType::Rename, ModalMessage::Close))
                    .chain(Task::done(Message::UpdateEntries(None)))
            }
            Message::Create(mode) => {
                // true if creating file, else creating folder
                if mode {
                    let overlay = self.modals_state.create_file.as_mut().unwrap();

                    let err = file::create(
                        &self.program.path,
                        PathBuf::from(overlay.content.clone().trim()),
                        true,
                    );
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

                    let err = file::create(
                        &self.program.path,
                        PathBuf::from(overlay.content.clone().trim()),
                        false,
                    );
                    if let Some(e) = err {
                        overlay.error = e;
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
                                    let selected = self.entries.get(index).unwrap();

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
                                modals_state.delete = Some(true);
                                *modals_opened = true;
                            }
                            ModalMessage::Close => {
                                modals_state.delete = None;
                                *modals_opened = false;
                            }
                            _ => {}
                        }
                        Task::none()
                    }
                    ModalType::Operation => {
                        match msg {
                            ModalMessage::Open => {
                                modals_state.operation = Some(true);
                                *modals_opened = true;
                            }
                            ModalMessage::Close => {
                                modals_state.operation = None;
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
                                    error: String::new(),
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
                                    error: String::new(),
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
        let entries = &self.entries;
        let buttons: Element<Message> = column!()
            .extend(
                entries
                    .iter()
                    .map(|e| {
                        container(
                            mouse_area(
                                row![
                                    text(e.name.clone())
                                        .width(300)
                                        .align_x(alignment::Horizontal::Left),
                                    text(e.filetype.clone())
                                        .width(150)
                                        .align_x(alignment::Horizontal::Left),
                                    text(
                                        DateTime::from_timestamp_secs(e.created)
                                            .unwrap()
                                            .to_string()
                                    )
                                    .width(200)
                                    .align_x(alignment::Horizontal::Left),
                                    text(
                                        DateTime::from_timestamp_secs(e.accessed)
                                            .unwrap()
                                            .to_string()
                                    )
                                    .width(Length::FillPortion(3))
                                    .align_x(alignment::Horizontal::Left)
                                ]
                                .spacing(5)
                                .padding(5),
                            )
                            .on_double_click(Message::Open(e.path.clone()))
                            .on_press(Message::Select(e.index.clone()))
                            .on_enter(Message::HoverEntry(e.index.clone(), true))
                            .on_exit(Message::HoverEntry(e.index.clone(), false)),
                        )
                        .style(|_theme| {
                            let mut style = container::Style::default();

                            if let Some(cur_index) = self.current_index
                                && cur_index == e.index
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

                            if self.selected.contains(&e.index) {
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
            .width(Length::Fill)
            .into();

        let explorer_scroll = scrollable(buttons)
            .id("scrollable")
            .width(Length::Fill)
            .height(Length::Fill);

        let explorer_select = container(mouse_area(explorer_scroll).on_press(
            if !self.modifiers_state.ctrl {
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

        let right_col = column![
            container(text("explorer info"))
                .height(30)
                .center_y(30)
                .center_x(Length::Fill)
                .padding(5),
            text(format!(
                "showing hidden files: {}",
                if self.view_hidden { "yes" } else { "no" }
            ))
            .height(20)
            .width(Length::Fill),
            clipboard
        ]
        .width(300)
        .spacing(10);

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

            if let Some(th) = &thing.error {
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

        if let Some(_) = self.modals_state.operation {
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

        if let Some(_) = self.modals_state.delete {
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
                text(&thing.error).color(Color::from_rgba(1.0, 105.0 / 255.0, 97.0 / 255.0, 1.0))
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
                text(&thing.error).color(Color::from_rgba(1.0, 105.0 / 255.0, 97.0 / 255.0, 1.0))
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
                Event::Keyboard(keyboard::Event::ModifiersChanged(m)) => {
                    Some(Message::UpdateModifiersState(m.control(), m.shift()))
                }

                Event::Keyboard(keyboard::Event::KeyPressed {
                    physical_key,
                    modifiers,
                    ..
                }) => {
                    match (physical_key, modifiers) {
                        (key::Physical::Code(Code::KeyC), keyboard::Modifiers::CTRL) => {
                            Some(Message::AddClipboard(ClipboardMode::Copy))
                        }
                        (key::Physical::Code(Code::KeyX), keyboard::Modifiers::CTRL) => {
                            Some(Message::AddClipboard(ClipboardMode::Cut))
                        }
                        (key::Physical::Code(Code::KeyV), keyboard::Modifiers::CTRL) => Some(
                            Message::UpdateModal(ModalType::Operation, ModalMessage::Open),
                        ),
                        (key::Physical::Code(Code::Delete), _) => {
                            Some(Message::UpdateModal(ModalType::Delete, ModalMessage::Open))
                        }
                        (key::Physical::Code(Code::F2), _) => {
                            Some(Message::UpdateModal(ModalType::Rename, ModalMessage::Open))
                        }
                        (key::Physical::Code(Code::Escape), _) => Some(Message::CloseModals),
                        (key::Physical::Code(Code::KeyH), keyboard::Modifiers::CTRL) => {
                            Some(Message::ToggleHiddenView)
                        }
                        (key::Physical::Code(Code::KeyN), keyboard::Modifiers::CTRL) => Some(
                            Message::UpdateModal(ModalType::CreateFile, ModalMessage::Open),
                        ), // create file
                        (key::Physical::Code(Code::KeyN), keyboard::Modifiers::ALT) => Some(
                            Message::UpdateModal(ModalType::CreateFolder, ModalMessage::Open),
                        ), // create folder
                        _ => Some(Message::KeyPressed(physical_key, modifiers)),
                    }
                }
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
        move || Application::new(input.clone(), config::get_keybinds()),
        Application::update,
        Application::view,
    )
    .subscription(Application::subscription)
    .title("buoyant")
    .run()
}
