use std::{
    borrow::Cow,
    fmt::Display,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use eframe::egui::mutex::RwLock;
use rustc_hash::FxHashSet;

use crate::icons::IconKind;

#[derive(Clone, Copy)]
pub enum PasteKind {
    Replace,
    Duplicate,
}

#[derive(Default)]
pub struct Overlay {
    pub buffer: String,
    pub error: String,
    pub path: Option<PathBuf>,
    pub kind: Option<OverlayKind>,
    pub entry: Option<Entry>,
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

#[derive(Default)]
pub struct Field {
    pub buffer: String,
    pub kind: Option<FieldKind>,
    pub focused: bool,
}

impl Field {
    pub fn reset(&mut self) {
        self.buffer = String::new();
        self.kind = None;
        self.focused = false;
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FieldKind {
    Search,
}

#[derive(Default, Debug)]
pub struct Clipboard {
    pub entries: FxHashSet<PathBuf>,
    pub mode: Option<ClipboardMode>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ClipboardMode {
    Copy,
    Cut,
}

#[derive(Clone, Debug)]
// temporary entry that doesnt allocate memory
pub struct TempEntry<'a> {
    pub name: &'a str,
    pub file_type: &'static str,
    pub file_icon: IconKind,
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
    pub file_type: &'static str,
    pub file_icon: IconKind,
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
            file_icon: IconKind::File,
            file_type: "",
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
    pub title: &'static str,
    pub content: Cow<'static, str>,
    pub start_time: Instant,
    pub duration: Duration,
    pub kind: ToastKind,
}

impl Default for Toast {
    fn default() -> Self {
        Toast {
            title: "",
            content: Cow::Borrowed(""),
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

#[derive(PartialEq, Default)]
pub enum Property {
    #[default]
    Name,
    Accessed,
    Created,
    Type,
    Size,
    Path,
}

impl Display for Property {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let thing = match self {
            Property::Name => "name",
            Property::Accessed => "accessed",
            Property::Created => "created",
            Property::Type => "type",
            Property::Size => "size",
            Property::Path => "path",
        };

        write!(f, "{}", thing)
    }
}

#[derive(PartialEq)]
pub enum CreateType {
    File,
    Folder,
}

pub type Toasts = Arc<RwLock<Vec<Toast>>>;

#[derive(Debug)]
pub enum WorkerRequest {
    OperationType { path: PathBuf },
    Done { paths: Vec<PathBuf> },
}
