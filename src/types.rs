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
    pub delete: Option<ChoiceModal>,
}

impl Default for Modals {
    fn default() -> Self {
        Modals {
            rename: None,
            create_file: None,
            create_folder: None,
            paste: None,
            delete: None,
        }
    }
}

pub enum ModalType {
    Rename,
    CreateFile,
    CreateFolder,
    Paste,
    Delete,
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

impl Default for Clipboard {
    fn default() -> Self {
        Clipboard {
            entries: HashSet::with_capacity(5),
            mode: None,
        }
    }
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

#[derive(Debug, Clone)]
pub struct Entries {
    pub children: Vec<Entry>,
    pub displaying: Vec<usize>,
}
/*
impl Entries {
    pub fn entry(&self, index: &usize) -> Option<&Entry> {
        self.children.get(*self.displaying.get(*index).unwrap())
    }
}*/

impl Default for Entries {
    fn default() -> Self {
        let mut children = Vec::with_capacity(30);

        for _ in 0..=30 {
            children.push(Entry {
                ..Default::default()
            });
        }

        Entries {
            children,
            displaying: Vec::with_capacity(30),
        }
    }
}
