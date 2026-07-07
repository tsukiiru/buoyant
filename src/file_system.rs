use std::{fs, path, process};

pub const BANNED_CHARACTERS: [&str; 4] = ["\0", "\"", "/", "*"];

pub fn rename(path: &path::Path, name: &str) {
    let mut new_path = path.to_path_buf();
    new_path.set_file_name(name);

    let command = process::Command::new("mv").arg(path).arg(new_path).output();

    if let Err(err) = command {
        println!("{}", err);
    }
}

pub fn delete(path: &path::Path) {
    if !path.exists() {
        return;
    }

    let command = process::Command::new("rm").arg("-rf").arg(path).output();

    if let Err(e) = command {
        println!("{}", e);
    }
}

pub fn create(
    current_path: &path::Path,
    new_path: &path::Path,
    last_is_file: bool,
) -> Option<&'static str> {
    let layers: Vec<_> = new_path.components().collect();

    if layers.len() == 0 {
        return None;
    }

    let mut clean_path = current_path.to_path_buf();

    for layer in layers {
        let name = layer.as_os_str().to_str().unwrap();
        for c in BANNED_CHARACTERS {
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
        let command = process::Command::new("touch").arg(clean_path).output();
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

pub fn read_dir(path: &path::Path) -> Result<Vec<path::PathBuf>, String> {
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

pub fn is_hidden(path: &path::Path) -> bool {
    let point_to_file = path.file_name();

    if point_to_file.is_none() {
        return false;
    }

    point_to_file.unwrap().to_str().unwrap().starts_with(".")
}
