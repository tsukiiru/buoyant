use iced::advanced::image::Handle;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    collections::HashSet,
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Command,
    str::from_utf8,
    time::SystemTime,
};

use crate::file_types;
use crate::types::PasteType;
pub const NONO_CHARACTERS: [&str; 3] = ["\0", "\"", "/"];

pub fn delete(path: &Path) {
    if !path.exists() {
        return;
    }

    let command = Command::new("rm").arg("-rf").arg(path).output();

    if let Err(e) = command {
        println!("{}", e);
    }
}

pub fn rename(path: &PathBuf, name: &str) {
    let mut new_path = path.clone();
    new_path.set_file_name(name);

    let command = Command::new("mv").arg(path).arg(new_path).output();

    if let Err(err) = command {
        println!("{}", err);
    }
}

pub fn create(current_path: &Path, new_path: &Path, last_is_file: bool) -> Option<&'static str> {
    let layers: Vec<_> = new_path.components().collect();

    if layers.len() == 0 {
        return None;
    }

    let mut clean_path = current_path.to_path_buf();

    for layer in layers {
        let name = layer.as_os_str().to_str().unwrap();
        for c in NONO_CHARACTERS {
            if name.contains(c) {
                return Some("invalid characters");
            }
        }
        clean_path.push(layer);
    }

    let mut path_without_last = clean_path.clone();
    path_without_last.pop();

    let try_create = fs::create_dir_all(path_without_last);
    if let Err(err) = try_create {
        println!("{}", err);
    }

    if last_is_file {
        let command = Command::new("touch").arg(clean_path).output();
        if let Err(err) = command {
            println!("{}", err);
        }
    } else {
        let try_create = fs::create_dir(clean_path);
        if let Err(err) = try_create {
            println!("{}", err);
        }
    }

    None
}

fn paste<'a>(
    dest: &Path,
    prevs: &mut Vec<&'a str>,
    paste_type: &PasteType,
    path: &'a Path,
    is_cut: bool, // true - cut. false - copy
) {
    let name = path.file_name().unwrap().to_str().unwrap();
    let mut final_path = dest.to_path_buf();
    prevs.iter().for_each(|prev| final_path.push(prev));

    let joined = &final_path.join(&name);
    // check if not exists in the destination
    if !joined.exists() {
        move_file(path, joined, is_cut);
        return;
    }

    match paste_type {
        PasteType::Duplicate => {
            let result = file_extension(path);
            let ext = if result == "" {
                String::new()
            } else {
                format!(".{}", result)
            };
            // since both file/folder has the same outcome for choosing duplicate
            let new_path = increment_suffix(&file_name(path), ext.as_str(), &final_path);
            move_file(path, &new_path, is_cut);
        }
        PasteType::Replace => {
            if path == joined {
                return;
                // does nothing if trying to merge with the same destination as start
            }

            if !final_path.is_file() {
                replace_file(path, joined, is_cut);
            } else {
                prevs.push(name);
                paste(dest, prevs, paste_type, path, is_cut);
            }
        }
    }
}

pub fn move_dir(old_files: &HashSet<PathBuf>, dest: &Path, operation: &PasteType) {
    if !dest.exists() || !dest.is_dir() {
        return;
    }

    // check if file with same name exists
    // if not, move like usual
    // if yes, check if merge or duplicate

    // if duplicate, increment suffix normally, move to the next file
    // if merge, check if folder or file, if folder, get into that folder and repeat, if file, replace the file in destination

    old_files.par_iter().for_each(|path| {
        let mut clean_path = path.clone();
        clean_path.pop();

        if clean_path != dest {
            paste(&dest, &mut Vec::with_capacity(5), operation, &path, true);
        }
    })
}

pub fn copy_dir(old_files: &HashSet<PathBuf>, dest: &Path, operation: &PasteType) {
    if !dest.exists() || !dest.is_dir() {
        return;
    }

    old_files
        .par_iter()
        .for_each(|p| paste(dest, &mut Vec::with_capacity(5), operation, &p, false));
}

fn move_file(old_path: &Path, new_path: &Path, is_cut: bool) {
    let command;

    if is_cut {
        command = Command::new("mv").arg(old_path).arg(new_path).output();
    } else {
        command = Command::new("cp")
            .arg(old_path)
            .arg(new_path)
            .arg("-r")
            .output();
    }

    if let Err(e) = command {
        println!("{}", e);
    }
}

fn replace_file(old_path: &Path, new_path: &Path, is_cut: bool) {
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
pub fn read_dir(path: &Path) -> Result<Vec<PathBuf>, String> {
    let read_results = fs::read_dir(path);

    if read_results.is_err() {
        return Err(String::from(
            "cannot read directory without root permissions",
        ));
    }

    Ok(read_results
        .unwrap()
        .map(|r| r.map(|e| e.path()).unwrap())
        .collect::<Vec<_>>())
}

// get file name but stripped the extension
pub fn file_name(path: &Path) -> String {
    let mut name = path.file_name().unwrap().to_str().unwrap().to_string();

    if path.is_dir() {
        return name;
    }

    let i = name.rfind(".");
    // trim the extension off

    if let Some(size) = i
        && size != 0
    {
        name.truncate(size);
    }

    name
}

fn is_textfile(path: &Path) -> bool {
    use std::io::Read;
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };

    let mut buf = [0u8; 512];
    let Ok(n) = file.read(&mut buf) else {
        return false;
    };

    buf[..n].iter().all(|&b| b.is_ascii())
}

pub fn file_type(path: &Path) -> (String, &'static Handle) {
    if path.is_dir() {
        return (String::from("Folder"), &*file_types::FOLDER);
    }

    let ext = file_extension(path);
    let opt_type = file_types::extension_to_filetype(ext);
    let str_type: &str;
    let icon: &Handle;

    if let Some(thing) = &opt_type {
        str_type = &thing.0;
        icon = &thing.1;
    } else if is_textfile(path) {
        str_type = "Text File";
        icon = &file_types::FILE;
    } else {
        str_type = "Unknown";
        icon = &file_types::QUESTION_MARK;
    }

    if path.is_symlink() {
        let text = "Symlink".to_owned();

        if str_type == "Unknown" {
            return (text + " (broken)", &file_types::BROKEN_LINK);
        } else {
            return (text + " -> " + str_type, &file_types::LINK);
        };
    }

    (str_type.to_owned(), icon)
}

const UNIX_EPOCH: SystemTime = SystemTime::UNIX_EPOCH;

pub fn accessed_and_created(path: &Path) -> (i64, i64) {
    match path.metadata() {
        Ok(res) => (
            res.accessed()
                .unwrap_or(UNIX_EPOCH)
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .try_into()
                .unwrap(),
            res.created()
                .unwrap_or(UNIX_EPOCH)
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .try_into()
                .unwrap(),
        ),

        Err(_) => (0, 0),
    }
}

pub fn accurate_filesize(path: &Path) -> u64 {
    if !path.exists() {
        return 0_u64;
    }

    let command = Command::new("du").arg("-s").arg(path).output();

    if let Err(error) = &command {
        println!(
            "problem encountered when trying to read {}: {}",
            path.display(),
            error
        );
    }

    let output = command.unwrap().stdout;
    let result = from_utf8(&output).unwrap();
    let (a, _) = result.split_once(char::is_whitespace).unwrap();

    let size: u64 = a.parse::<u64>().unwrap() * 1024;
    // since the size from du is in KiB
    //
    size
}
// THIS IS REALLY EXPENSIVE AND SLOW WHEN THERE ARE TOO MANY FILES
// Though similar approaches should be considered for more accurate file size.

pub fn file_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0_u64;
    }

    let read_metadata = path.metadata();

    if !read_metadata.is_ok() {
        println!(
            "problem encountered when trying to read metadata of {}",
            path.display()
        );
    }

    let metadata = read_metadata.unwrap();
    metadata.size()
}

pub fn folder_size(path: &Path) -> Option<usize> {
    if path.is_file() {
        return None;
    }
    fs::read_dir(path).ok().map(|d| d.count())
}

pub fn bytes_to_string(size: u64) -> String {
    // i dont think someone would have petabytes of data on their personal computer,,,
    if size >= 10_u64.pow(12) {
        // TiB
        let round = size / 10_u64.pow(12);
        return format!("{:.2}TiB", round);
    } else if size >= 10_u64.pow(9) {
        // GiB
        let round = size / 10_u64.pow(9);
        return format!("{:.2}GiB", round);
    } else if size >= 10_u64.pow(6) {
        // MiB
        let round = size / 10_u64.pow(6);
        return format!("{:.2}MiB", round);
    } else if size >= 10_u64.pow(3) {
        // KiB
        let round = size / 10_u64.pow(3);
        return format!("{:.2}KiB", round);
    } else {
        // bytes
        return format!("{} bytes", size);
    }
}

pub fn is_hidden(path: &Path) -> bool {
    file_name(path).starts_with(".")
}

fn file_extension(path: &Path) -> &str {
    let ext = path.extension();

    if let Some(e) = ext {
        e.to_str().unwrap()
    } else {
        ""
    }
}

// for checking if theres existing files at destination,
// if there is, increment the ending by one, [FILE_NAME] (number)
fn increment_suffix(file_name: &str, file_extension: &str, destination: &Path) -> PathBuf {
    for k in 0usize.. {
        let name = if k == 0 {
            format!("{}{}", file_name, file_extension)
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
