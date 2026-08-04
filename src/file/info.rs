use crate::file::{icons::IconKind, types};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::SystemTime,
};

pub fn file_name_without_extension(path: &Path) -> String {
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

pub fn file_extension(path: &Path) -> &str {
    let ext = path.extension();

    if let Some(e) = ext {
        e.to_str().unwrap()
    } else {
        ""
    }
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

    buf[..n].par_iter().all(|&b| b.is_ascii())
}

pub fn file_type(path: &Path) -> (&'static str, IconKind) {
    if path.is_dir() {
        return ("Folder", IconKind::Folder);
    }

    let str_type: &str;
    let icon;

    if let Some(thing) = types::extension_to_file_type(file_extension(path)) {
        str_type = thing.0;
        icon = thing.1;
    } else if is_textfile(path) {
        str_type = "Text File";
        icon = IconKind::File;
    } else {
        str_type = "Unknown";
        icon = IconKind::QuestionMark;
    }

    if path.is_symlink() {
        if str_type == "Unknown" {
            return ("Symlink", IconKind::BrokenLink);
        }
        return ("Symlink", IconKind::Link);
    }

    (str_type, icon)
}

pub fn read_dir(path: &Path) -> Result<Vec<PathBuf>, String> {
    let read_results = fs::read_dir(path);

    if read_results.is_err() {
        return Err(String::from(
            "cannot read directory without root permissions",
        ));
    }

    Ok(read_results
        .unwrap()
        .map(|r| r.unwrap().path())
        .collect::<Vec<_>>())
}

pub fn is_hidden(path: &Path) -> bool {
    let point_to_file = path.file_name();

    if point_to_file.is_none() {
        return false;
    }

    point_to_file.unwrap().to_str().unwrap().starts_with(".")
}

const UNIX_EPOCH: SystemTime = SystemTime::UNIX_EPOCH;

pub fn accessed_and_created(
    path: &Path,
    fetch_accessed: &bool,
    fetch_created: &bool,
) -> (Option<i64>, Option<i64>) {
    if !*fetch_accessed && !*fetch_created {
        return (None, None);
    }

    match path.metadata() {
        Ok(res) => (
            if *fetch_accessed {
                Some(
                    res.accessed()
                        .unwrap_or(UNIX_EPOCH)
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                        .try_into()
                        .unwrap(),
                )
            } else {
                None
            },
            if *fetch_created {
                Some(
                    res.created()
                        .unwrap_or(UNIX_EPOCH)
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                        .try_into()
                        .unwrap(),
                )
            } else {
                None
            },
        ),

        Err(_) => (None, None),
    }
}

pub fn file_size(path: &Path, should_fetch: &bool) -> Option<u64> {
    if !should_fetch {
        return None;
    }
    if !path.exists() {
        return Some(0_u64);
    }

    let read_metadata = path.metadata();

    if !read_metadata.is_ok() {
        println!(
            "problem encountered when trying to read metadata of {}",
            path.display()
        );
    }

    let metadata = read_metadata.unwrap();
    Some(metadata.size())
}

pub fn folder_size(path: &Path, should_fetch: &bool) -> Option<usize> {
    if path.is_file() || !*should_fetch {
        return None;
    }
    fs::read_dir(path).ok().map(|d| d.count())
}
