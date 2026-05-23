use std::{
    env::home_dir,
    path::PathBuf,
    process::{Command, Stdio},
};

use chrono::DateTime;

use iced::{
    Background, Color, Element, Event, Length, Subscription, Task, alignment, event,
    event::Status,
    keyboard::{
        self,
        key::{self, Code},
    },
    widget::{
        Id, button, column, container, float, mouse_area, opaque, operation, row, scrollable,
        stack, text, text_input,
    },
};

mod path;
use file::PathControl;
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
            // if its a folder
        } else {
            let cmd = Command::new("xdg-open")
                .arg(new_path)
                .stderr(Stdio::null())
                .spawn();

            if let Err(e) = cmd {
                println!("{}", e);
            }
            // try to open with default program
            // if not, errors
        }
    }

    fn relative_nav(&mut self, dir: PathControl) {
        match dir {
            PathControl::Backward => {
                self.path.pop();
            }
            PathControl::Forward => {
                // probaly not yet, cuz theres no history for now
                // isnt this just going to the previous path??
            }
        }

        // println!("new path is at: {}", self.path.display());
    }
}

#[derive(Clone, Debug)]
enum ClipboardType {
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
    Navigate(PathControl),

    UpdateEntries(Option<PathBuf>),
    ToggleHiddenView,
    Select(usize),
    HoverEntry(usize, bool),

    ResetSelection,
    DeleteSelection,
    NavigateSelection(Direction),
    OpenSelection,

    UpdateControlKey(bool),

    Rename,

    AddClipboard(ClipboardType),
    PasteClipboard(file::OperationChoice),

    CheckModals,
    UpdateRenameModal(String),
    CloseRenameModal,
    OpenRenameModal,
    OpenOperationModal,
    CloseOperationModal,
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

struct Application {
    program: Program,
    entries: Vec<Entry>,
    view_hidden: bool,
    selected: Vec<usize>,
    holding_ctrl: bool,
    clipboard: Vec<PathBuf>,
    clipboard_type: ClipboardType,
    rename_modal: Option<RenameModal>,
    operation_modal: Option<bool>,
    modal_opened: bool,
}

const NONO_CHARACTERS: [&str; 10] = ["\0", "\\", "\"", "/", ":", "*", "?", "<", ">", "|"];

impl Application {
    fn new() -> (Self, Task<Message>) {
        (
            Application {
                program: Program::init(home_dir().unwrap()),
                entries: vec![],
                view_hidden: false,
                selected: vec![],
                holding_ctrl: false,
                clipboard: vec![],
                clipboard_type: ClipboardType::Copy,
                rename_modal: None,
                operation_modal: None,
                modal_opened: false,
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

                Task::done(Message::UpdateEntries(None))
            }
            Message::Navigate(to) => {
                if self.modal_opened {
                    return Task::none();
                }

                let mut path: Option<PathBuf> = None;

                match to {
                    PathControl::Backward => {
                        path = Some(self.program.path.clone());
                        self.program.relative_nav(PathControl::Backward);
                    }
                    PathControl::Forward => {
                        self.program.relative_nav(PathControl::Forward);
                    }
                }

                Task::done(Message::UpdateEntries(path))
            }
            Message::UpdateEntries(prev_path) => {
                self.entries.clear();

                let cur_paths = file::read_dir(&self.program.path).unwrap();
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

                if let Some(index) = prev_path {
                    for entry in self.entries.iter() {
                        if entry.path == index {
                            self.selected = vec![entry.index];
                        }
                    }
                } else {
                    self.selected.clear();
                }

                Task::none()
            }
            Message::HoverEntry(id, state) => {
                let entry = self.entries.get_mut(id).unwrap();
                entry.hovered = state;

                Task::none()
            }
            Message::UpdateControlKey(state) => {
                self.holding_ctrl = state;
                Task::none()
            }
            Message::Select(index) => {
                if !self.holding_ctrl {
                    self.selected.clear();
                }

                self.selected.push(index);

                Task::none()
            }
            Message::ResetSelection => {
                self.selected.clear();
                Task::none()
            }
            Message::DeleteSelection => Task::none(),
            Message::AddClipboard(t) => {
                if self.modal_opened {
                    return Task::none();
                }

                if !self.selected.is_empty() {
                    let mut new_vec: Vec<PathBuf> = vec![];

                    self.selected
                        .iter()
                        .for_each(|i| new_vec.push(self.entries[*i].path.clone()));

                    self.clipboard = new_vec;
                    self.clipboard_type = t;
                }
                Task::none()
            }
            Message::PasteClipboard(opp) => {
                if self.modal_opened {
                    return Task::none();
                }

                if self.clipboard.is_empty() {
                    return Task::none();
                }

                match self.clipboard_type {
                    ClipboardType::Copy => {
                        file::copy_dir(self.clipboard.clone(), self.program.path.clone(), &opp);
                    }
                    ClipboardType::Cut => {
                        file::move_dir(self.clipboard.clone(), self.program.path.clone(), &opp);
                        self.clipboard.clear();
                    }
                }
                Task::done(Message::CloseOperationModal)
                    .chain(Task::done(Message::UpdateEntries(None)))
            }
            Message::NavigateSelection(direction) => {
                if self.modal_opened {
                    return Task::none();
                }

                let index_opt = self.selected.get(self.selected.len() - 1);
                let mut current_index: usize = 0;
                let mut exists = true;

                if let Some(thing) = index_opt {
                    current_index = thing.clone();
                } else {
                    exists = false;
                    // im sorry.
                }

                let new_index: usize;
                // get the last selected index OR the first index if no selection
                match direction {
                    Direction::Down => {
                        new_index = if current_index >= self.entries.len() - 1 {
                            current_index
                        } else if !exists {
                            0
                        } else {
                            current_index + 1
                        };
                    }
                    Direction::Up => {
                        new_index = if current_index == 0 {
                            current_index
                        } else {
                            current_index - 1
                        };
                    }
                }

                if self.holding_ctrl {
                    self.selected.push(new_index);
                } else {
                    self.selected = vec![new_index];
                }

                // TODO: update position of view following the selection index
                Task::none()
            }
            Message::OpenSelection => {
                if self.modal_opened {
                    return Task::none();
                }

                let index_opt = self.selected.get(self.selected.len() - 1);
                let cur_index: usize;

                if let Some(thing) = index_opt {
                    cur_index = *thing;
                } else {
                    return Task::none();
                }

                if let Some(path) = self.entries.get(cur_index) {
                    Task::done(Message::Open(path.path.clone()))
                } else {
                    Task::none()
                }
            }
            Message::Rename => {
                let overlay = self.rename_modal.as_mut().unwrap();
                let name = &overlay.content.clone().trim().to_string();

                if name.is_empty() {
                    return Task::none();
                }

                // checking if the new name is valid?
                for char in NONO_CHARACTERS {
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
                Task::done(Message::CloseRenameModal)
                    .chain(Task::done(Message::UpdateEntries(None)))
            }
            Message::OpenRenameModal => {
                if self.modal_opened {
                    return Task::none();
                }

                let selection = self.selected.get(self.selected.len() - 1);

                if let Some(thing) = selection {
                    let selected = self.entries.get(thing.clone()).unwrap();
                    self.rename_modal = Some(RenameModal {
                        path: selected.path.clone(),
                        content: selected.name.clone(),
                        error: None,
                    })
                }

                self.modal_opened = true;

                Task::batch(vec![operation::focus(Id::new("rename"))])
            }
            Message::CloseRenameModal => {
                self.rename_modal = None;
                self.modal_opened = false;
                Task::none()
            }
            Message::UpdateRenameModal(content) => {
                let overlay = self.rename_modal.as_mut().unwrap();
                overlay.content = content;

                Task::none()
            }
            Message::CheckModals => operation::is_focused(Id::new("rename")).then(|o| {
                if !o {
                    Task::done(Message::CloseRenameModal)
                } else {
                    Task::none()
                }
            }),
            Message::OpenOperationModal => {
                self.operation_modal = Some(true);
                self.modal_opened = true;
                Task::none()
            }
            Message::CloseOperationModal => {
                self.operation_modal = None;
                self.modal_opened = false;
                Task::none()
            }
            Message::ToggleHiddenView => {
                self.view_hidden = !self.view_hidden;
                Task::done(Message::UpdateEntries(None))
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let entries = &self.entries;
        let buttons: Element<Message> =
            column!(button(text("...")).on_press(Message::Navigate(PathControl::Backward).into()))
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
                            .style(if self.selected.contains(&e.index) {
                                container::success
                            } else if e.hovered && !self.selected.contains(&e.index) {
                                container::danger
                            } else {
                                container::transparent
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

        let explorer_select =
            container(mouse_area(explorer_scroll).on_press(if !self.holding_ctrl {
                Message::ResetSelection
            } else {
                Message::None
            }))
            .width(Length::Fill)
            .height(Length::Fill);

        let clipboard_type = match self.clipboard_type {
            ClipboardType::Copy => "Copy",
            ClipboardType::Cut => "Cut",
        };

        let clipboard_entries = &self.clipboard;
        let clipboard: Element<Message> = column![text(format!("type: {}", clipboard_type))]
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
            text(format!(
                "showing hidden files: {}",
                if self.view_hidden { "yes" } else { "nah bro" }
            ))
            .height(20)
            .width(Length::Fill),
            clipboard
        ]
        .width(300)
        .padding(20)
        .spacing(10);

        let content = row![explorer_select, left_col]
            .width(Length::Fill)
            .height(Length::Fill);

        let mut stack = stack![content].width(Length::Fill).height(Length::Fill);

        if let Some(thing) = &self.rename_modal {
            let input = text_input("input the new name here :3", &thing.content)
                .on_input(Message::UpdateRenameModal)
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
                container(col).style(move |_t| style()).center(Length::Fill),
            ));

            stack = stack.push(overlay);
        }

        if let Some(_thing) = self.operation_modal {
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
                    .style(move |_t| style())
                    .center(Length::Fill),
            ));

            stack = stack.push(overlay);
        }

        stack.into()
    }

    fn subscription(&self) -> Subscription<Message> {
        event::listen_with(|event, status, _id| {
            if status == Status::Captured {
                return Some(Message::CheckModals);
            }

            match event {
                Event::Keyboard(keyboard::Event::ModifiersChanged(m)) => {
                    Some(Message::UpdateControlKey(m.control()))
                }

                Event::Keyboard(keyboard::Event::KeyPressed {
                    physical_key,
                    modifiers,
                    ..
                }) => match (physical_key, modifiers) {
                    (key::Physical::Code(Code::KeyC), keyboard::Modifiers::CTRL) => {
                        Some(Message::AddClipboard(ClipboardType::Copy))
                    }
                    (key::Physical::Code(Code::KeyX), keyboard::Modifiers::CTRL) => {
                        Some(Message::AddClipboard(ClipboardType::Cut))
                    }
                    (key::Physical::Code(Code::KeyV), keyboard::Modifiers::CTRL) => {
                        Some(Message::OpenOperationModal)
                    }
                    (key::Physical::Code(Code::Delete), _) => Some(Message::DeleteSelection),
                    (key::Physical::Code(Code::ArrowDown), _) => {
                        Some(Message::NavigateSelection(Direction::Down))
                    }
                    (key::Physical::Code(Code::ArrowUp), _) => {
                        Some(Message::NavigateSelection(Direction::Up))
                    }
                    (key::Physical::Code(Code::ArrowLeft), _) => {
                        Some(Message::Navigate(PathControl::Backward))
                    }
                    (key::Physical::Code(Code::ArrowRight), _) => Some(Message::OpenSelection),
                    (key::Physical::Code(Code::F2), _) => Some(Message::OpenRenameModal),
                    (key::Physical::Code(Code::Escape), _) => Some(Message::CloseOperationModal),
                    (key::Physical::Code(Code::KeyH), keyboard::Modifiers::CTRL) => {
                        Some(Message::ToggleHiddenView)
                    }
                    _ => None,
                },
                _ => None,
            }
        })
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new().0
    }
}

fn style() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.95))),
        ..Default::default()
    }
}

fn main() -> iced::Result {
    iced::application(Application::new, Application::update, Application::view)
        .subscription(Application::subscription)
        .title("buoyant")
        .run()
}
