use std::{error::Error, fs, io, path::PathBuf, process::Command};

pub enum PathControl {
    Backward,
    Forward,
}

pub fn move_dir(old_files: Vec<PathBuf>, dest: PathBuf) -> Result<(), Box<dyn Error>> {
    if !dest.exists() || !dest.is_dir() {
        return Ok(());
        // check if path not exists and is not a folder
    }

    for path in old_files {
        let new_path = increment_suffix(get_filename(&path), get_fileextension(&path), &dest);

        let cmd = Command::new("mv").arg(&path).arg(new_path).output();
        // lmao this is so stupid
        if let Err(e) = cmd {
            println!("{}", e);
        }
    }

    Ok(())
}

pub fn copy_dir(old_files: Vec<PathBuf>, dest: PathBuf) -> Result<(), Box<dyn Error>> {
    if !dest.exists() || !dest.is_dir() {
        return Ok(());
        // check if path not exists and is not a folder
    }

    for path in old_files {
        let new_path = increment_suffix(get_filename(&path), get_fileextension(&path), &dest);

        let cmd = Command::new("cp").arg(&path).arg(&new_path).output();

        if let Err(e) = cmd {
            println!("{}", e);
        }
    }

    Ok(())
}

pub fn read_dir(path: &PathBuf) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    Ok(fs::read_dir(path)?
        .map(|r| r.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()?)
} // returns a vec of children of a directory

// helper functions
fn get_filename(path: &PathBuf) -> String {
    let mut name = path.file_name().unwrap().to_str().unwrap().to_string();
    let i = name.rfind(".").unwrap();

    name.truncate(i);
    name
}

fn get_fileextension(path: &PathBuf) -> String {
    path.extension().unwrap().to_str().unwrap().to_string()
}

pub fn is_hidden(path: &PathBuf) -> bool {
    let name = path.file_name().unwrap();
    name.to_str().unwrap().chars().nth(0) == Some('.')
}

// for checking if theres existing files at destination,
// if there is, increment the ending by one, [FILE_NAME] (number)
fn increment_suffix(file_name: String, file_extension: String, destination: &PathBuf) -> PathBuf {
    for k in 0usize.. {
        let name = if k == 0 {
            format!("{}.{}", file_name.clone(), file_extension)
        } else {
            format!("{} ({}).{}", file_name, k, file_extension)
        };

        let path = destination.join(&name);
        if !path.exists() {
            return path;
        }
    }

    unreachable!("infinite iterator exhausted")
}
