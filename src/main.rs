use chrono::DateTime;
use file::PathControl;
use iced::advanced::Widget;
use iced::event::Status;
use iced::widget::text_input::State;
use iced::widget::{container, mouse_area, opaque, text_input};
use iced::{Background, Event, event};
use iced::{
    Color, Element, Length, Subscription, Task, alignment,
    keyboard::{
        self,
        key::{self, Code},
    },
    widget::{button, column, float, row, scrollable, stack, text},
};
use path as file;
use std::{
    env::home_dir,
    path::PathBuf,
    process::{Command, Stdio},
};

mod path;

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
                let cur_path = &self.path;

                if cur_path.iter().count() <= 1 {
                    //println!("current path is already at root!, {}", cur_path.display());
                    return;
                }

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
    UpdateEntries,
    Select(usize),
    ResetSelection,
    DeleteSelection,
    NavigateSelection(Direction),
    RenameSelection,
    OpenSelection,
    UpdateControlKey(bool),
    AddClipboard(ClipboardType),
    PasteClipboard,
    Rename,
    UpdateRenameModal(String),
}

struct Entry {
    name: String,
    path: PathBuf,
    accessed: i64,
    created: i64,
    index: usize,
}

struct RenameModal {
    path: PathBuf,
    content: String,
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
}

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
            },
            Task::done(Message::UpdateEntries),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Open(path) => {
                self.program.open(&path);

                if path.is_file() {
                    return Task::none();
                }

                Task::done(Message::UpdateEntries)
            }
            Message::Navigate(to) => {
                match to {
                    PathControl::Backward => {
                        self.program.relative_nav(PathControl::Backward);
                    }
                    PathControl::Forward => {
                        self.program.relative_nav(PathControl::Forward);
                    }
                }
                Task::done(Message::UpdateEntries)
            }
            Message::UpdateEntries => {
                self.entries.clear();
                self.selected.clear();

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
                    });
                    i += 1;
                }

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
            Message::RenameSelection => {
                let selection = self.selected.get(self.selected.len() - 1);

                if let Some(thing) = selection {
                    let selected = self.entries.get(thing.clone()).unwrap();
                    self.rename_modal = Some(RenameModal {
                        path: selected.path.clone(),
                        content: selected.name.clone(),
                    })
                }

                Task::none()
            }
            Message::None => Task::none(),
            Message::AddClipboard(t) => {
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
            Message::PasteClipboard => {
                if self.clipboard.is_empty() {
                    return Task::none();
                }

                match self.clipboard_type {
                    ClipboardType::Copy => {
                        file::copy_dir(self.clipboard.clone(), self.program.path.clone());
                    }
                    ClipboardType::Cut => {
                        file::move_dir(self.clipboard.clone(), self.program.path.clone());
                        self.clipboard.clear();
                    }
                }
                Task::done(Message::UpdateEntries)
            }
            Message::NavigateSelection(direction) => {
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
                self.selected = vec![new_index];
                // TODO: update position of view following the selection index
                Task::none()
            }
            Message::OpenSelection => {
                let index_opt = self.selected.get(self.selected.len() - 1);
                let cur_index: usize;

                if let Some(thing) = index_opt {
                    cur_index = *thing;
                } else {
                    return Task::none();
                }

                let path = self.entries.get(cur_index).unwrap().path.clone();
                Task::done(Message::Open(path))
            }
            Message::Rename => {
                let overlay = self.rename_modal.as_mut().unwrap();

                println!(
                    "trying to rename {} to {}",
                    file::get_filename(&overlay.path),
                    overlay.content
                );

                self.rename_modal = None;

                Task::none()
            }
            Message::UpdateRenameModal(content) => {
                let overlay = self.rename_modal.as_mut().unwrap();
                overlay.content = content;

                Task::none()
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
                                            .width(400)
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
                                    .spacing(5),
                                )
                                .on_double_click(Message::Open(e.path.clone()))
                                .on_press(Message::Select(e.index.clone())),
                            )
                            .style(if self.selected.contains(&e.index) {
                                container::danger
                            } else {
                                container::transparent
                            })
                            .padding(5)
                            .into()
                        })
                        .collect::<Vec<_>>(),
                )
                .spacing(10)
                .padding(20)
                .width(Length::Fill)
                .into();

        let explorer_scroll = scrollable(buttons).width(Length::Fill).height(Length::Fill);
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
            .padding(20)
            .width(500)
            .into();

        let content = row![explorer_select, clipboard]
            .width(Length::Fill)
            .height(Length::Fill);

        let stack = stack![content].width(Length::Fill).height(Length::Fill);

        if let Some(thing) = &self.rename_modal {
            let input = text_input("input the new name here :3", &thing.content)
                .on_input(Message::UpdateRenameModal)
                .on_submit(Message::Rename);

            let overlay = opaque(float(
                container(
                    column![
                        text(format!("you are renaming, {}", thing.path.display())),
                        input
                    ]
                    .width(500),
                )
                .style(move |_t| style())
                .center(Length::Fill),
            ));

            stack.push(overlay).into()
        } else {
            stack.into()
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        event::listen_with(|event, status, _id| {
            if status == Status::Captured {
                return None;
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
                        Some(Message::PasteClipboard)
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
                    (key::Physical::Code(Code::F2), _) => Some(Message::RenameSelection),
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
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.6))),
        ..Default::default()
    }
}

fn main() -> iced::Result {
    iced::application(Application::new, Application::update, Application::view)
        .subscription(Application::subscription)
        .title("buoyant")
        .run()
}
