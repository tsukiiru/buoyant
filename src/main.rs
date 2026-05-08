use chrono::DateTime;
use file::PathControl;
use iced::{
    Element, Length, Subscription, Task, alignment, keyboard,
    widget::{MouseArea, button, column, container, row, scrollable, text},
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

    fn open(&mut self, new_path: PathBuf) {
        if new_path.is_dir() {
            self.path = new_path;
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
enum Message {
    None,
    // The blank message, does nothing
    Open(PathBuf),
    Navigate(PathControl),
    UpdateEntries,
    Select(PathBuf),
    ResetSelection,
    UpdateControlKey(bool),
}

struct Entry {
    name: String,
    path: PathBuf,
    accessed: i64,
    created: i64,
    hidden: bool,
}

struct Application {
    program: Program,
    entries: Vec<Entry>,
    view_hidden: bool,
    selected: Vec<PathBuf>,
    holding_ctrl: bool,
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
            },
            Task::done(Message::UpdateEntries),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Open(path) => {
                self.program.open(path);
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
                for path in cur_paths {
                    self.entries.push(Entry {
                        name: path.file_name().unwrap().to_str().unwrap().to_string(),
                        path: path.clone(),
                        created: file::get_filecreated(&path),
                        accessed: file::get_fileaccessed(&path),
                        hidden: file::is_hidden(&path),
                    })
                }

                if !self.view_hidden {
                    self.entries.retain(|entry| !entry.hidden);
                }

                Task::none()
            }
            Message::UpdateControlKey(state) => {
                self.holding_ctrl = state;
                Task::none()
            }
            Message::Select(path) => {
                if !self.holding_ctrl {
                    self.selected.clear();
                }

                self.selected.push(path);

                Task::none()
            }
            Message::ResetSelection => {
                self.selected.clear();
                Task::none()
            }
            Message::None => Task::none(),
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
                                MouseArea::new(
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
                                .on_press(Message::Select(e.path.clone())),
                            )
                            .style(if self.selected.contains(&e.path) {
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
        let explorer_select = container(MouseArea::new(explorer_scroll).on_press(
            if !self.holding_ctrl {
                Message::ResetSelection
            } else {
                Message::None
            },
        ))
        .width(Length::Fill)
        .height(Length::Fill);
        let content: Element<Message> = column![explorer_select]
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        keyboard::listen().filter_map(|e| match e {
            keyboard::Event::ModifiersChanged(m) => Some(Message::UpdateControlKey(m.control())),
            _ => None,
        })
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new().0
    }
}

fn main() -> iced::Result {
    iced::application(Application::new, Application::update, Application::view)
        .subscription(Application::subscription)
        .title("buoyant")
        .run()
}
