use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub enum PasteKind {
    Replace,
    Duplicate,
}

pub struct Overlay {
    pub buffer: String,
    pub error: String,
    pub path: Option<PathBuf>,
    pub kind: Option<OverlayKind>,
    pub entry: Option<Entry>,
}

impl Default for Overlay {
    fn default() -> Self {
        Overlay {
            buffer: String::new(),
            error: String::new(),
            path: None,
            kind: None,
            entry: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum OverlayKind {
    Rename,
    CreateFile,
    CreateFolder,
    Paste,
    Delete,
    Metadata,
}

pub struct Field {
    pub buffer: String,
    pub kind: Option<FieldKind>,
    pub focused: bool,
}

impl Default for Field {
    fn default() -> Self {
        Field {
            buffer: String::new(),
            kind: None,
            focused: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FieldKind {
    Search,
}

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
    pub file_type: Option<&'static str>,
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
    pub file_type: Option<&'static str>,
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
            file_type: None,
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

impl Entries {
    pub fn entry(&self, index: &usize) -> Option<&Entry> {
        self.children.get(*self.displaying.get(*index).unwrap())
    }
}

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

#[derive(Debug)]
pub struct Toast {
    pub title: String,
    pub content: String,
    pub start_time: Instant,
    pub duration: Duration,
    pub kind: ToastKind,
}

impl Default for Toast {
    fn default() -> Self {
        Toast {
            title: String::new(),
            content: String::new(),
            start_time: Instant::now(),
            duration: Duration::ZERO,
            kind: ToastKind::Info,
        }
    }
}

#[derive(Debug)]
pub enum ToastKind {
    Info,
    Danger,
    Success,
}

pub enum NavigateDirection {
    Up,
    Down,
}
