use std::{error::Error, fs, io, path::PathBuf, process::Command};

#[derive(Clone, Debug)]
pub enum PathControl {
    Backward,
    Forward,
}

pub enum OperationChoice {
    Merge,
    Duplicate,
}

pub fn rename(path: &mut PathBuf, name: String) {
    path.set_file_name(name);
}

fn operation_recursion(dest: &PathBuf, prevs: &mut Vec<String>, path: &PathBuf, is_cut: bool) {
    let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
    println!("{}", file_name);
    let mut final_path = dest.clone();
    prevs.iter().for_each(|prev| final_path.push(prev));

    let joined = &final_path.join(&file_name);
    println!("{}", joined.display());
    // check if not exists in the destination
    if !joined.exists() {
        println!("selected doesnt exist in destination");
        move_file(path, joined, is_cut);
        return;
    }

    // TODO: prompt to choose between replace or duplicate
    // assuming choice here for now
    let choice = OperationChoice::Merge;

    match choice {
        OperationChoice::Duplicate => {
            // since both file/folder has the same outcome for choosing duplicate
            let new_path =
                increment_suffix(get_filename(path), get_fileextension(path), &final_path);
            move_file(path, &new_path, is_cut);
        }
        OperationChoice::Merge => {
            if !final_path.is_file() {
                // if is not folder
                println!("selected file is not a folder");
                replace_file(path, &final_path.join(&file_name), is_cut);
            } else {
                println!("folder");
                prevs.push(file_name);
                operation_recursion(dest, prevs, path, is_cut);
            }
        }
    }
}

pub fn move_dir(old_files: Vec<PathBuf>, dest: PathBuf) -> Result<(), Box<dyn Error>> {
    if !dest.exists() || !dest.is_dir() {
        return Ok(());
        // check if path not exists and is not a folder
    }

    // check if file with same name exists
    // if not, move like usual
    // if yes, check if merge or duplicate

    // if duplicate, increment suffix normally, move to the next file
    // if merge, check if folder or file, if folder, get into that folder and repeat, if file, replace the file in destination

    old_files
        .iter()
        .for_each(|p| operation_recursion(&dest, &mut vec![], &p, true));

    Ok(())
}

pub fn copy_dir(old_files: Vec<PathBuf>, dest: PathBuf) -> Result<(), Box<dyn Error>> {
    if !dest.exists() || !dest.is_dir() {
        return Ok(());
        // check if path not exists and is not a folder
    }

    old_files
        .iter()
        .for_each(|p| operation_recursion(&dest, &mut vec![], &p, false));

    Ok(())
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
pub fn read_dir(path: &PathBuf) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    Ok(fs::read_dir(path)?
        .map(|r| r.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()?)
} // returns a vec of children of a directory

// get file name but stripped the extension
fn get_filename(path: &PathBuf) -> String {
    let mut name = path.file_name().unwrap().to_str().unwrap().to_string();
    if path.is_dir() {
        return name;
    }
    let i = name.rfind(".").unwrap();
    // trim the extension off

    name.truncate(i);
    name
}

fn get_fileextension(path: &PathBuf) -> String {
    let ext = path.extension();
    let mut result = String::from("");

    if let Some(e) = ext {
        println!("this is in extension");
        result.push_str(".");
        result.push_str(e.to_str().unwrap());
    }

    result
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
