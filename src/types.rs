use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub enum Direction {
    Up,
    Down,
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

#[derive(Debug)]
pub struct TempItem<'a> {
    pub name: &'a str,
    pub filetype: &'a str,
    pub path: &'a Path,

    pub accessed: i64,
    pub created: i64,
    pub filesize: u64,
    pub foldersize: Option<usize>,

    pub hidden: bool,
}

pub trait Entry {
    fn name(&self) -> &str;
    fn filetype(&self) -> &str;
    fn accessed(&self) -> i64;
    fn created(&self) -> i64;
    fn filesize(&self) -> u64;
}

impl Entry for Item {
    fn name(&self) -> &str {
        &self.name
    }
    fn filetype(&self) -> &str {
        &self.filetype
    }
    fn accessed(&self) -> i64 {
        self.accessed
    }
    fn created(&self) -> i64 {
        self.created
    }
    fn filesize(&self) -> u64 {
        self.filesize
    }
}

impl<'a> Entry for TempItem<'a> {
    fn name(&self) -> &str {
        self.name
    }
    fn filetype(&self) -> &str {
        self.filetype
    }
    fn accessed(&self) -> i64 {
        self.accessed
    }
    fn created(&self) -> i64 {
        self.created
    }
    fn filesize(&self) -> u64 {
        self.filesize
    }
}

#[derive(Debug)]
pub struct Item {
    pub id: usize,
    pub name: String,
    pub path: PathBuf,

    pub accessed: i64,
    pub created: i64,
    pub filetype: String,
    pub filesize: u64,
    pub foldersize: Option<usize>, // number of items in the folder (if it is)

    pub hidden: bool,
    pub hovered: bool,
    pub using: bool,
}

impl Default for Item {
    fn default() -> Self {
        Item {
            id: 0,
            name: String::with_capacity(16),
            path: PathBuf::with_capacity(60),
            accessed: 0,
            created: 0,
            filetype: String::with_capacity(20),
            filesize: 0,
            foldersize: None,
            hidden: false,
            hovered: false,
            using: false,
        }
    }
}

pub struct Entries {
    pub children: Vec<Item>,
}

impl Entries {
    pub fn new() -> Self {
        let mut children = Vec::with_capacity(30);

        for i in 0..=30 {
            children.push(Item {
                id: i,
                ..Default::default()
            });
        }

        // TODO: configurable pre-allocated size

        Entries { children }
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
