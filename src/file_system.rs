use crate::{
    file_types,
    icons::IconKind,
    types::{CreateType, PasteKind, WorkerRequest},
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rustc_hash::FxHashSet;
use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc,
    time::SystemTime,
};

pub const BANNED_CHARACTERS: [&str; 4] = ["\0", "\"", "/", "*"];

pub fn rename(path: &Path, name: &str) -> Result<PathBuf, String> {
    let mut new_path = path.to_path_buf();
    new_path.set_file_name(name);

    let command = Command::new("mv").arg(path).arg(&new_path).output();

    if let Err(err) = command {
        return Err(err.to_string());
    }

    Ok(new_path)
}

pub fn delete<'a, I>(paths: I) -> Result<(), String>
where
    I: Iterator<Item = &'a Path>,
{
    let mut return_error = None;

    paths.for_each(|path| {
        if !path.exists() {
            return_error = Some(String::from("provided path doesn't exist?"));
        }

        let command = Command::new("rm").arg("-rf").arg(path).output();

        if let Err(e) = command {
            return_error = Some(e.to_string());
        }
    });

    if let Some(err) = return_error {
        return Err(err);
    }

    Ok(())
}

pub fn create(
    current_path: &Path,
    new_path: &Path,
    create_type: &CreateType,
) -> Result<PathBuf, String> {
    let layers = new_path.components();

    if layers.clone().count() == 0 {
        return Err(String::from("maybe dont leave the input box blank?"));
    }

    let mut clean_path = current_path.to_path_buf();

    for layer in layers {
        let name = layer.as_os_str().to_str().unwrap();
        for c in BANNED_CHARACTERS {
            if name.contains(c) {
                return Err(String::from("invalid characters"));
            }
        }
        clean_path.push(layer);
    }

    let mut path_without_last = clean_path.clone();
    path_without_last.pop();

    let try_create = fs::create_dir_all(path_without_last);
    if let Err(err) = try_create {
        return Err(err.to_string());
    }

    if *create_type == CreateType::File {
        let command = Command::new("touch").arg(&clean_path).output();
        if let Err(err) = command {
            return Err(err.to_string());
        }
    } else {
        let try_create = fs::create_dir(&clean_path);
        if let Err(err) = try_create {
            return Err(err.to_string());
        }
    }

    Ok(clean_path)
}

fn paste<'a>(
    dest: &Path,
    prevs: &mut Vec<&'a str>,
    path: &'a Path,
    is_cut: bool, // true - cut. false - copy
    from_worker: &mpsc::Sender<WorkerRequest>,
    to_worker: &mpsc::Receiver<PasteKind>,
) -> Option<PathBuf> {
    let name = path.file_name().unwrap().to_str().unwrap();
    let mut final_path = dest.to_path_buf();
    prevs.iter().for_each(|prev| final_path.push(prev));

    let joined = &final_path.join(name);
    // check if not exists in the destination
    if !joined.exists() {
        move_file(path, joined, is_cut);
        return Some(joined.clone());
    }

    let _ = from_worker.send(WorkerRequest::OperationType {
        path: joined.to_path_buf(),
    });

    let paste_kind = to_worker.recv();
    if paste_kind.is_err() {
        return None;
    }
    match paste_kind.unwrap() {
        PasteKind::Duplicate => {
            let result = file_extension(path);
            let ext = if result.is_empty() {
                String::new()
            } else {
                format!(".{}", result)
            };
            // since both file/folder has the same outcome for choosing duplicate
            let new_path = increment_suffix(&file_name(path), ext.as_str(), &final_path);
            move_file(path, &new_path, is_cut);
            Some(new_path)
        }
        PasteKind::Replace => {
            if path == joined {
                return Some(joined.clone());
                // does nothing if trying to merge with the same destination as start
            }

            if !final_path.is_file() {
                replace_file(path, joined, is_cut);
                Some(joined.clone())
            } else {
                prevs.push(name);
                paste(dest, prevs, path, is_cut, from_worker, to_worker)
            }
        }
    }
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

fn file_extension(path: &Path) -> &str {
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

    if let Some(thing) = file_types::extension_to_file_type(file_extension(path)) {
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

pub fn move_dir(
    old_files: FxHashSet<PathBuf>,
    dest: PathBuf,
    from_worker: &mpsc::Sender<WorkerRequest>,
    to_worker: &mpsc::Receiver<PasteKind>,
) {
    if !dest.exists() || !dest.is_dir() {
        let _ = from_worker.send(WorkerRequest::Done { paths: Vec::new() });
        return;
    }

    let mut resulte = Vec::with_capacity(old_files.len());
    for path in old_files {
        let mut clean_path = path.clone();
        clean_path.pop();

        if clean_path != dest
            && let Some(p) = paste(
                &dest,
                &mut Vec::with_capacity(5),
                &path,
                true,
                from_worker,
                to_worker,
            )
        {
            resulte.push(p);
            continue;
        }
    }

    let _ = from_worker.send(WorkerRequest::Done { paths: resulte });
}

pub fn copy_dir(
    old_files: FxHashSet<PathBuf>,
    dest: PathBuf,
    from_worker: &mpsc::Sender<WorkerRequest>,
    to_worker: &mpsc::Receiver<PasteKind>,
) {
    if !dest.exists() || !dest.is_dir() {
        let _ = from_worker.send(WorkerRequest::Done { paths: Vec::new() });
        return;
    }

    let mut resulte = Vec::with_capacity(old_files.len());

    for path in old_files {
        if let Some(p) = paste(
            &dest,
            &mut Vec::with_capacity(5),
            &path,
            false,
            from_worker,
            to_worker,
        ) {
            resulte.push(p);
            continue;
        }
    }

    let _ = from_worker.send(WorkerRequest::Done { paths: resulte });
}

fn move_file(old_path: &Path, new_path: &Path, is_cut: bool) {
    let command = if is_cut {
        Command::new("mv").arg(old_path).arg(new_path).output()
    } else {
        Command::new("cp")
            .arg(old_path)
            .arg(new_path)
            .arg("-r")
            .output()
    };

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

    let cmd = Command::new(program).arg(old_path).arg(new_path).output();

    if let Err(e) = cmd {
        println!("{}", e);
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

pub fn bytes_to_string(size: u64) -> String {
    // i dont think someone would have petabytes of data on their personal computer,,,
    if size >= 10_u64.pow(12) {
        // TiB
        let round = size / 10_u64.pow(12);
        format!("{:.2}TiB", round)
    } else if size >= 10_u64.pow(9) {
        // GiB
        let round = size / 10_u64.pow(9);
        format!("{:.2}GiB", round)
    } else if size >= 10_u64.pow(6) {
        // MiB
        let round = size / 10_u64.pow(6);
        format!("{:.2}MiB", round)
    } else if size >= 10_u64.pow(3) {
        // KiB
        let round = size / 10_u64.pow(3);
        format!("{:.2}KiB", round)
    } else {
        // bytes
        format!("{} bytes", size)
    }
}
