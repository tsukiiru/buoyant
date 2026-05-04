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

    fn open(&mut self, new_path: PathBuf) -> Result<(), Box<dyn Error>> {
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

        Ok(())
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

        println!("new path is at: {}", self.path.display());
    }
}

fn show(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    println!("displaying children for path: {}", path.display());
    println!("");
    path::read_dir(path)?
        .iter()
        .for_each(|v| println!("{}", v.file_name().unwrap().display()));

    println!("--------------");
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut program = Program::init(home_dir().unwrap());

    // lets try moving a path
    path::move_dir(
        vec![
            PathBuf::from("/home/dew/img.png"),
            PathBuf::from("/home/dew/img.png"),
            PathBuf::from("/home/dew/img.png"),
        ],
        PathBuf::from("/home/dew/target/"),
    )?;

    Ok(())
}
