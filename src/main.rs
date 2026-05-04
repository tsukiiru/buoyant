use iced::{
    Element, Task, debug,
    widget::{button, column, container, scrollable, text},
};
use path::PathControl;
use std::{
    env::home_dir,
    error::Error,
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
                    println!("current path is already at root!, {}", cur_path.display());
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

#[cfg(debug_assertions)]
fn show(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    println!("displaying children for path: {}", path.display());
    println!("");
    path::read_dir(path)?
        .iter()
        .for_each(|v| println!("{}", v.file_name().unwrap().display()));

    println!("--------------");
    Ok(())
}

#[derive(Clone, Debug)]
enum Message {
    Open(PathBuf),
    Navigate(PathControl),
}

struct Application {
    program: Program,
}

impl Application {
    fn new() -> Self {
        Application {
            program: Program::init(home_dir().unwrap()),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Open(path) => {
                self.program.open(path);

                Task::none()
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
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let files = path::read_dir(&self.program.path).unwrap();
        let buttons: Element<Message> =
            column!(button(text("...")).on_press(Message::Navigate(PathControl::Backward).into()))
                .extend(
                    files
                        .iter()
                        .map(|f| {
                            let name = f.file_name().unwrap().to_str().unwrap().to_string();
                            button(text(name))
                                .on_press(Message::Open(f.to_path_buf()))
                                .into()
                        })
                        .collect::<Vec<_>>(),
                )
                .spacing(10)
                .into();

        let explorer_scroll = scrollable(buttons);
        let content: Element<Message> = column![explorer_scroll].into();
        container(content).into()
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}

fn main() -> iced::Result {
    iced::application(Application::new, Application::update, Application::view).run()
}
