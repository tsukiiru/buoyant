use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::file_types;
use iced::advanced::svg::Handle;

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

pub struct SearchModal {
    pub content: String,
    pub focused: bool,
}

impl Default for SearchModal {
    fn default() -> Self {
        SearchModal {
            content: String::new(),
            focused: false,
        }
    }
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
    Search,
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
    pub icon: &'static Handle,

    pub accessed: i64,
    pub created: i64,
    pub filesize: u64,
    pub foldersize: Option<usize>,

    pub hidden: bool,
}

#[derive(Debug)]
pub struct Item {
    pub name: String,
    pub path: PathBuf,
    pub icon: &'static Handle,

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
            name: String::with_capacity(16),
            path: PathBuf::new(),
            icon: &file_types::FILE,
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
    pub children: Vec<Item>,    // stored entries
    pub displaying: Vec<usize>, // filtered one referencing the ones in memory
}

impl Entries {
    pub fn new() -> Self {
        let mut children = Vec::with_capacity(30);

        for _ in 0..=30 {
            children.push(Item {
                ..Default::default()
            });
        }

        Entries {
            children,
            displaying: Vec::with_capacity(30),
        }
    }

    pub fn item(&self, index: &usize) -> Option<&Item> {
        self.children.get(*self.displaying.get(*index).unwrap())
    }

    /*
    pub fn item_mut(&mut self, index: &usize) -> Option<&mut Item> {
        self.children.get_mut(*self.displaying.get(*index).unwrap())
    }*/
}
