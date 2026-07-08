use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

pub enum PasteType {
    Replace,
    Duplicate,
}

pub struct Modals {
    pub rename: Option<InputModal>,
    pub create_file: Option<InputModal>,
    pub create_folder: Option<InputModal>,
    pub paste: Option<ChoiceModal>,
}

impl Default for Modals {
    fn default() -> Self {
        Modals {
            rename: None,
            create_file: None,
            create_folder: None,
            paste: None,
        }
    }
}

pub enum ModalType {
    Rename,
    CreateFile,
    CreateFolder,
    Paste,
}

// modal for inputting text
pub struct InputModal {
    pub path: Option<PathBuf>,
    pub content: String,
    pub error: String,
}

// modal with some choices
pub struct ChoiceModal {}

#[derive(Debug)]
pub struct Clipboard {
    pub entries: HashSet<PathBuf>,
    pub mode: Option<ClipboardMode>,
}

#[derive(Debug)]
pub enum ClipboardMode {
    Copy,
    Cut,
}

#[derive(Clone, Debug)]
// temporary entry that doesnt allocate memory
pub struct TempEntry<'a> {
    pub name: &'a str,
    pub path: &'a Path,
    pub is_hidden: bool,
    pub accessed: Option<i64>,
    pub created: Option<i64>,
    pub file_size: Option<u64>,
    pub folder_size: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub using: bool,
    pub is_hidden: bool,
    pub accessed: Option<i64>,
    pub created: Option<i64>,
    pub file_size: Option<u64>,
    pub folder_size: Option<usize>,
}

impl Default for Entry {
    fn default() -> Self {
        Entry {
            name: String::with_capacity(16),
            path: PathBuf::new(),
            using: false,
            is_hidden: false,
            accessed: None,
            created: None,
            file_size: None,
            folder_size: None,
        }
    }
}
