use std::{fs, path};

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
