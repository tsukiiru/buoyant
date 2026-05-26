use file_type::{self, FileType};
use std::{fs, path::PathBuf, process::Command, time::SystemTime};

#[derive(Clone, Debug)]
pub enum OperationChoice {
    Merge,
    Duplicate,
}

pub fn delete(path: &PathBuf) {
    if !path.exists() {
        return;
    }

    let command = Command::new("rm").arg("-rf").arg(path).output();

    if let Err(e) = command {
        println!("{}", e);
    }
}

pub fn rename(path: &PathBuf, name: String) {
    let mut new_path = path.clone();
    new_path.set_file_name(name);

    let command = Command::new("mv").arg(path).arg(new_path).output();

    if let Err(err) = command {
        println!("{}", err);
    }
}

fn operation_recursion(
    dest: &PathBuf,
    prevs: &mut Vec<String>,
    operation: &OperationChoice,
    path: &PathBuf,
    is_cut: bool,
) {
    let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
    let mut final_path = dest.clone();
    prevs.iter().for_each(|prev| final_path.push(prev));

    let joined = &final_path.join(&file_name);
    // check if not exists in the destination
    if !joined.exists() {
        move_file(path, joined, is_cut);
        return;
    }

    match operation {
        OperationChoice::Duplicate => {
            // since both file/folder has the same outcome for choosing duplicate
            let new_path = increment_suffix(
                get_filename(path),
                format!(".{}", get_fileextension(path)),
                &final_path,
            );
            move_file(path, &new_path, is_cut);
        }
        OperationChoice::Merge => {
            if final_path == *dest {
                return;
                // does nothing if trying to merge with the same destination as start
            }

            if !final_path.is_file() {
                // if is not folder
                //               println!("selected file is not a folder");
                replace_file(path, &final_path.join(&file_name), is_cut);
            } else {
                // println!("folder");
                prevs.push(file_name);
                operation_recursion(dest, prevs, operation, path, is_cut);
            }
        }
    }
}

pub fn move_dir(old_files: Vec<PathBuf>, dest: PathBuf, operation: &OperationChoice) {
    if !dest.exists() || !dest.is_dir() {
        return;
        // check if path not exists and is not a folder
    }

    // check if file with same name exists
    // if not, move like usual
    // if yes, check if merge or duplicate

    // if duplicate, increment suffix normally, move to the next file
    // if merge, check if folder or file, if folder, get into that folder and repeat, if file, replace the file in destination

    old_files
        .iter()
        .for_each(|p| operation_recursion(&dest, &mut vec![], operation, &p, true));
}

pub fn copy_dir(old_files: Vec<PathBuf>, dest: PathBuf, operation: &OperationChoice) {
    if !dest.exists() || !dest.is_dir() {
        return;
        // check if path not exists and is not a folder
    }

    old_files
        .iter()
        .for_each(|p| operation_recursion(&dest, &mut vec![], operation, &p, false));
}

fn move_file(old_path: &PathBuf, new_path: &PathBuf, is_cut: bool) {
    let program = if is_cut { "mv" } else { "cp" };
    let cmd = Command::new(program).arg(&old_path).arg(new_path).output();

    if let Err(e) = cmd {
        println!("{}", e);
    }
}

fn replace_file(old_path: &PathBuf, new_path: &PathBuf, is_cut: bool) {
    let program = if is_cut { "mv" } else { "cp" };

    Command::new("rm")
        .arg("-rf")
        .arg(new_path)
        .output()
        .unwrap();
    // remove before copying / moving

    let cmd = Command::new(program).arg(&old_path).arg(new_path).output();

    if let Err(e) = cmd {
        println!("{}", e);
    }
}

// helper functions
pub fn read_dir(path: &PathBuf) -> Vec<PathBuf> {
    let read_results = fs::read_dir(path);

    if let Err(hi) = &read_results {
        eprintln!("{}", hi);
    }

    read_results
        .unwrap()
        .map(|r| r.map(|e| e.path()).unwrap())
        .collect::<Vec<_>>()
} // returns a vec of children of a directory

// get file name but stripped the extension
pub fn get_filename(path: &PathBuf) -> String {
    let mut name = path.file_name().unwrap().to_str().unwrap().to_string();
    if path.is_dir() {
        return name;
    }

    let i = name.rfind(".");
    // trim the extension off

    if let Some(size) = i {
        name.truncate(size);
    }
    name
}

// file name but with extension
fn get_filenameext(path: &PathBuf) -> String {
    path.file_name().unwrap().to_str().unwrap().to_string()
}

pub fn get_filetype(path: &PathBuf) -> String {
    if path.is_dir() {
        return String::from("Folder");
    }

    let ext = get_fileextension(path);
    let opt_type = FileType::from_extension(ext).first();

    if let Some(thing) = opt_type {
        thing.name().to_string()
    } else {
        String::from("unknown")
    }
    // i'll try to get a more accurate file type later
}

const UNIX_EPOCH: SystemTime = SystemTime::UNIX_EPOCH;

pub fn get_fileaccessed(path: &PathBuf) -> i64 {
    match path.metadata() {
        Ok(res) => res
            .accessed()
            .unwrap_or(UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .try_into()
            .unwrap(),
        // bring the file back to the prehistoric time period if cannot find the last accessed time
        // okay ima actually kms for real with this horrendous shit :sob:
        Err(_e) => 0, // i'll probably add permission issues someday
    }
}

pub fn get_filecreated(path: &PathBuf) -> i64 {
    match path.metadata() {
        Ok(res) => res
            .created()
            .unwrap_or(UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .try_into()
            .unwrap(),
        // bring the file back to the prehistoric time period if cannot find the last accessed time
        Err(_e) => 0,
    }
}

pub fn is_hidden(path: &PathBuf) -> bool {
    // basically check if theres a dot at the start
    get_filenameext(path).starts_with(".")
}

fn get_fileextension(path: &PathBuf) -> String {
    let ext = path.extension();

    if let Some(e) = ext {
        // println!("this is in extension");
        e.to_str().unwrap().to_string()
    } else {
        String::from("")
    }
}

// for checking if theres existing files at destination,
// if there is, increment the ending by one, [FILE_NAME] (number)
fn increment_suffix(file_name: String, file_extension: String, destination: &PathBuf) -> PathBuf {
    for k in 0usize.. {
        let name = if k == 0 {
            format!("{}{}", file_name.clone(), file_extension)
        } else {
            format!("{} ({}){}", file_name, k, file_extension)
        };

        let path = destination.join(&name);
        if !path.exists() {
            return path;
        }
    }

    unreachable!("infinite iterator exhausted")
}
