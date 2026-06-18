use std::{collections::HashSet, path::PathBuf};

#[derive(Clone, Debug)]
pub enum Direction {
    Up,
    Down,
}

pub struct Entry {
    pub id: usize,
    pub name: String,
    pub path: PathBuf,

    pub accessed: i64,
    pub created: i64,
    pub filetype: String,
    pub filesize: u64,

    pub hidden: bool,
    pub hovered: bool,
}

pub struct RenameModal {
    pub path: PathBuf,
    pub content: String,
    pub error: &'static str,
}

pub struct CreateModal {
    pub content: String,
    pub error: &'static str,
}

pub struct Clipboard {
    pub entries: HashSet<PathBuf>,
    pub mode: Option<ClipboardMode>,
}

#[derive(Clone, Debug)]
pub enum ClipboardMode {
    Copy,
    Cut,
}

impl Default for Clipboard {
    fn default() -> Self {
        Clipboard {
            entries: HashSet::with_capacity(5),
            mode: None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum PasteType {
    Replace, // Replace for files, Merge for folder
    Duplicate,
}

#[derive(Clone, Debug)]
pub enum ModalType {
    Rename,
    Paste,
    Delete,
    CreateFile,
    CreateFolder,
}

#[derive(Clone, Debug)]
pub enum ModalMessage {
    Open,
    Close,
    Content(String),
}

pub struct Entries {
    pub children: Vec<Entry>,
}

impl Entries {
    pub fn new() -> Self {
        Entries {
            children: Vec::with_capacity(30),
        }
    }

    pub fn index(&self, id: &usize) -> usize {
        let mut res: usize = 0;

        self.children.iter().enumerate().for_each(|(index, entry)| {
            if entry.id == *id {
                res = index;
            }
        });

        res
    }
}
