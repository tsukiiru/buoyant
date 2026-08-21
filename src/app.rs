use std::{
  borrow::Cow,
  env,
  fmt::Display,
  fs,
  path::{Path, PathBuf},
  process::Command,
  sync::{Arc, mpsc},
  time::{Duration, Instant},
};

use eframe::egui::{Context, Event, Key, Modifiers, Theme, WidgetText, mutex::RwLock};
use rayon::{
  iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
  },
  slice::ParallelSliceMut,
};
use rustc_hash::FxHashSet;

use crate::{
  config::{self, *},
  file::{
    CreateType, PasteKind, WorkerRequest,
    icons::IconKind,
    info::{accessed_and_created, file_size, file_type, folder_size, is_hidden, read_dir},
    ops::{BANNED_CHARACTERS, copy_dir, create, delete, move_dir, rename},
  },
};

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

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FieldKind {
  Search,
}

#[derive(Default)]
pub struct WindowsManager {
  pub clipboard: Option<bool>,
}

pub enum WindowKind {
  Clipboard,
}

impl WindowsManager {
  pub fn close(&mut self, kind: WindowKind) {
    match kind {
      WindowKind::Clipboard => self.clipboard = None,
    }
  }

  fn open(&mut self, kind: WindowKind) {
    match kind {
      WindowKind::Clipboard => self.clipboard = Some(true),
    }
  }

  pub fn toggle(&mut self, kind: WindowKind) {
    match kind {
      WindowKind::Clipboard => {
        if self.clipboard.is_none() {
          self.open(kind);
        } else {
          self.close(kind);
        }
      }
    }
  }
}

#[derive(Default, Debug)]
pub struct ClipboardManager {
  pub entries: FxHashSet<PathBuf>,
  pub mode: Option<ClipboardMode>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ClipboardMode {
  Copy,
  Cut,
}

impl From<ClipboardMode> for String {
  fn from(value: ClipboardMode) -> Self {
    match value {
      ClipboardMode::Copy => String::from("copy"),
      ClipboardMode::Cut => String::from("cut"),
    }
  }
}

impl From<ClipboardMode> for WidgetText {
  fn from(value: ClipboardMode) -> Self { Self::Text(value.into()) }
}

#[derive(Clone, Debug)]
// temporary entry that doesnt allocate memory
struct TempEntry<'a> {
  name: &'a str,
  file_type: &'static str,
  file_icon: IconKind,
  path: &'a Path,
  is_hidden: bool,
  accessed: Option<i64>,
  created: Option<i64>,
  file_size: Option<u64>,
  folder_size: Option<usize>,
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
      file_icon: IconKind::QuestionMark,
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
pub struct EntriesManager {
  pub entries: Vec<Entry>,
  pub displaying: Vec<usize>,
  pub current_index: usize,
  pub scroll_signal: bool,
}

impl Default for EntriesManager {
  fn default() -> Self {
    let mut entries = Vec::with_capacity(30);

    for _ in 0..=30 {
      entries.push(Entry {
        ..Default::default()
      });
    }

    EntriesManager {
      entries,
      displaying: Vec::with_capacity(30),
      current_index: 0,
      scroll_signal: false,
    }
  }
}

impl EntriesManager {
  fn entry(&self, index: &usize) -> Option<&Entry> {
    self.entries.get(*self.displaying.get(*index).unwrap())
  }

  fn push(&mut self, temp_entry: &TempEntry, index: usize) {
    if let Some(entry) = self.entries.get_mut(index) {
      entry.using = true;

      entry.is_hidden = temp_entry.is_hidden;
      entry.file_size = temp_entry.file_size;
      entry.accessed = temp_entry.accessed;
      entry.created = temp_entry.created;
      entry.folder_size = temp_entry.folder_size;
      entry.file_type = temp_entry.file_type;
      entry.file_icon = temp_entry.file_icon.to_owned();

      entry.name.push_str(temp_entry.name);
      entry.path.push(temp_entry.path);

      return;
    }

    let mut entry = Entry {
      using: true,

      is_hidden: temp_entry.is_hidden,
      folder_size: temp_entry.folder_size,
      file_size: temp_entry.file_size,
      accessed: temp_entry.accessed,
      created: temp_entry.created,
      file_type: temp_entry.file_type,
      file_icon: temp_entry.file_icon.to_owned(),
      ..Default::default()
    };

    entry.name.push_str(temp_entry.name);
    entry.path.push(temp_entry.path);

    self.entries.push(entry);
  }

  fn sort(&mut self, sorting_by: &Property, reversed: bool) {
    // sort hidden entries from non-hidden
    self.displaying.par_sort_by(|a, b| {
      let (x, y) = (&self.entries[*a].is_hidden, &self.entries[*b].is_hidden);
      y.cmp(x)
    });

    // find the split index between hidden and non-hidden
    let mut split_index = 0;
    for (index, entry_index) in self.displaying.iter().enumerate() {
      if !self.entries[*entry_index].is_hidden {
        split_index = index;
        break;
      }
    }

    let reference = &self.entries;
    let (first, second) = self.displaying.split_at_mut(split_index);
    let mut coll = [first, second];

    // sort them separately
    coll.iter_mut().for_each(|displaying| {
      match sorting_by {
        Property::Name => {
          let mut lowercased: Vec<(usize, String)> = displaying
            .par_iter()
            .map(|&entry_index| (entry_index, reference[entry_index].name.to_lowercase()))
            .collect();

          lowercased.par_sort_by(|a, b| a.1.cmp(&b.1));
          displaying
            .par_iter_mut()
            .zip(lowercased.par_iter())
            .for_each(|(d, (i, _))| *d = *i);
        }
        Property::Size => displaying.par_sort_by(|a, b| {
          let (x, y) = (&reference[*a].file_size, &reference[*b].file_size);
          x.cmp(y)
        }),
        Property::Created => displaying.par_sort_by(|a, b| {
          let (x, y) = (&reference[*a].created, &reference[*b].created);
          x.cmp(y)
        }),
        Property::Accessed => displaying.par_sort_by(|a, b| {
          let (x, y) = (&reference[*a].accessed, &reference[*b].accessed);
          x.cmp(y)
        }),
        Property::Type => displaying.par_sort_by(|a, b| {
          let (x, y) = (&reference[*a].file_type, &reference[*b].file_type);
          x.cmp(y)
        }),
        _ => {}
      }

      if reversed {
        displaying.reverse();
      }
    });
  }
}

#[derive(Default)]
pub struct ToastsManager {
  pub toasts: Arc<RwLock<Vec<Toast>>>,
}

#[derive(Debug)]
pub struct Toast {
  pub title: &'static str,
  pub content: Cow<'static, str>,
  pub start_time: Instant,
  pub duration: Option<Duration>,
  pub percent: Option<f32>,
  pub count: i8,
  pub kind: ToastKind,
  pub id: Instant,
}

impl Default for Toast {
  fn default() -> Self {
    Toast {
      title: "",
      count: 1,
      content: Cow::Borrowed(""),
      start_time: Instant::now(),
      duration: None,
      percent: None,
      kind: ToastKind::Info,
      id: Instant::now(),
    }
  }
}

#[derive(Debug, PartialEq)]
pub enum ToastKind {
  Info,
  Danger,
  Success,
  Operation,
}

enum NavigateDirection {
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

pub struct WorkerChannels {
  request_chan: mpsc::Receiver<WorkerRequest>,
  response_chan: mpsc::Sender<PasteKind>,
  id: Instant,
}

#[derive(Default)]
pub struct ChannelsManager {
  channels_queue: Vec<WorkerChannels>,
  queued_chan_index: usize,
}

#[derive(Default, Clone, Copy)]
pub struct Position {
  r: usize,
  c: usize,
}

impl Display for Position {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "({}, {})", self.r, self.c)
  }
}

#[allow(dead_code)]
pub enum AppendDirection {
  Up,
  Down,
  Left,
  Right,
}

pub struct PanelsManager {
  pub focused: Position,
  pub panels: Vec<Vec<Panel>>,
  // matrix representing the panels' positions
  pub width_proportion: Vec<Vec<f32>>,
  pub height_proportion: Vec<f32>,
}

impl Default for PanelsManager {
  fn default() -> Self {
    PanelsManager {
      focused: Position::default(),
      panels: Vec::with_capacity(1),
      width_proportion: vec![vec![1.0]],
      height_proportion: vec![1.0],
    }
  }
}

impl PanelsManager {
  pub fn current_panel(&self) -> &Panel { self.panel(self.focused).unwrap() }

  pub fn current_panel_mut(&mut self) -> &mut Panel { self.panel_mut(self.focused).unwrap() }

  fn panel(&self, pos: Position) -> Option<&Panel> {
    if let Some(row) = self.panels.get(pos.r) {
      return row.get(pos.c);
    }

    None
  }

  fn panel_mut(&mut self, pos: Position) -> Option<&mut Panel> {
    if let Some(row) = self.panels.get_mut(pos.r) {
      return row.get_mut(pos.c);
    }

    None
  }

  fn new_panel(&mut self, dir: AppendDirection, path: &Path) {
    let mut new_pos = self.focused;

    if self.panel(self.focused).is_some() {
      match dir {
        AppendDirection::Up => new_pos.r = (new_pos.r as i16 - 1).min(0) as usize,
        AppendDirection::Down => new_pos.r += 1,
        AppendDirection::Left => new_pos.c = (new_pos.c as i16 - 1).min(0) as usize,
        AppendDirection::Right => new_pos.c += 1,
      };
    }

    let panel = Panel::new(path);

    if let Some(row) = self.panels.get_mut(new_pos.r) {
      row.insert(new_pos.c, panel);
      self.width_proportion[new_pos.r].insert(new_pos.c, 1.0);
    } else {
      // create a new row
      self.panels.insert(new_pos.r, vec![panel]);
      self.height_proportion.insert(new_pos.r, 1.0);
    }
  }
}

pub struct Panel {
  pub current_path: PathBuf,
  pub entries_manager: EntriesManager,
  pub selected: FxHashSet<usize>,
  pub field: Field,
}

impl Panel {
  fn new(path: &Path) -> Self {
    Panel {
      current_path: path.to_path_buf(),
      entries_manager: EntriesManager::default(),
      selected: FxHashSet::default(),
      field: Field::default(),
    }
  }
}

#[derive(Default)]
pub struct App {
  pub ctx: Context,
  pub clipboard_manager: ClipboardManager,
  pub overlay: Overlay,
  pub config: Config,
  pub toasts_manager: ToastsManager, // mhm toasts :3
  pub windows_manager: WindowsManager,
  pub panels_manager: PanelsManager,
  pub channels_manager: ChannelsManager,
}

impl App {
  pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
    let ctx = cc.egui_ctx.clone();
    egui_extras::install_image_loaders(&ctx);

    let mut app = App {
      ctx,
      ..Default::default()
    };
    app.fetch_config();
    app
      .panels_manager
      .new_panel(AppendDirection::Right, &env::home_dir().unwrap());

    app.fetch_entries();
    app
  }

  pub fn disable_scroll_signal(&mut self) {
    self
      .panels_manager
      .current_panel_mut()
      .entries_manager
      .scroll_signal = false;
  }

  fn fetch_config(&mut self) {
    config::fetch(&mut self.config);

    self.ctx.set_theme(if self.config.view.dark_mode {
      Theme::Dark
    } else {
      Theme::Light
    });

    //self.fetch_entries();
  }

  pub fn transfer(&mut self, to: usize) {
    let current_panel = self.panels_manager.current_panel();
    if current_panel.selected.contains(&to) {
      return;
    }

    let selected: FxHashSet<PathBuf> = current_panel
      .selected
      .iter()
      .map(|e_index| current_panel.entries_manager.entries[*e_index].path.clone())
      .collect();

    let (worker_tx, worker_rx) = mpsc::channel::<PasteKind>(); // main use - worker recv
    let (user_tx, user_rx) = mpsc::channel::<WorkerRequest>(); // main recv - worker use

    let (id_tx, id_rx) = mpsc::channel::<Instant>();

    self.new_toast(
      "Moving",
      Cow::Owned(format!("Moving {} items", selected.len())),
      ToastKind::Operation,
      Some(id_tx),
    );
    let id = id_rx.recv().unwrap();

    self.channels_manager.channels_queue.push(WorkerChannels {
      request_chan: user_rx,
      response_chan: worker_tx,
      id,
    });

    let destination = current_panel.entries_manager.entries[to].path.clone();

    std::thread::spawn(move || {
      move_dir(selected, destination, &user_tx, &worker_rx);
    });
  }

  pub fn nav_forward(&mut self) {
    let current_panel = self.panels_manager.current_panel_mut();

    let to = &current_panel
      .entries_manager
      .entry(&current_panel.entries_manager.current_index)
      .unwrap()
      .path;

    if !to.is_file() && !to.is_dir() {
      return;
    }

    if to.is_file() {
      let res = Command::new("xdg-open").arg(to).spawn();

      if let Err(err) = res {
        self.new_toast(
          "xdg-open",
          Cow::Owned(err.to_string()),
          ToastKind::Danger,
          None,
        );
      }
      return;
    }

    current_panel.current_path = to.to_path_buf();
    self.fetch_entries();
  }

  pub fn nav_back(&mut self) {
    let current_panel = self.panels_manager.current_panel_mut();
    let old_path = current_panel.current_path.clone();

    current_panel.current_path.pop();
    self.fetch_entries();
    self.highlight_path(&old_path);
  }

  fn fetch_entries(&mut self) {
    {
      let current_panel = self.panels_manager.current_panel_mut();
      current_panel.field.reset();
      // clear entries
      current_panel
        .entries_manager
        .entries
        .iter_mut()
        .for_each(|e| {
          e.name.clear();
          e.file_type = "";
          e.path = PathBuf::new();
          e.using = false;
          e.accessed = None;
          e.created = None;
          e.folder_size = None;
        });
    }

    let fetch_current_path = read_dir(&self.panels_manager.current_panel().current_path);

    if let Err(err) = &fetch_current_path {
      self.new_toast(
        "Error",
        Cow::Owned(err.to_string()),
        ToastKind::Danger,
        None,
      );
    }

    let mut index: usize = 0;
    let current_panel = self.panels_manager.current_panel_mut();

    for path in fetch_current_path.unwrap() {
      let (fetch_accessed, fetch_created) = (
        self.config.should_fetch(Property::Accessed),
        self.config.should_fetch(Property::Created),
      );
      let fetch_size = self.config.should_fetch(Property::Size);
      let (accessed, created) = accessed_and_created(&path, &fetch_accessed, &fetch_created);
      let (file_type, file_icon) = file_type(&path);

      current_panel.entries_manager.push(
        &TempEntry {
          name: path.file_name().unwrap().to_str().unwrap(),
          path: &path,
          is_hidden: is_hidden(&path),
          folder_size: folder_size(&path, &fetch_size),
          file_size: file_size(&path, &fetch_size),
          file_type,
          file_icon,
          accessed,
          created,
        },
        index,
      );
      index += 1;
    }

    current_panel.entries_manager.entries.truncate(index);

    // freed some mem from the greedy alloc
    unsafe {
      unsafe extern "C" {
        fn malloc_trim(pad: usize) -> i32;
      }
      malloc_trim(0);
    }

    self.filter_and_sort();
  }

  fn toggle_view_hidden(&mut self) {
    let selected_paths: Vec<PathBuf> = {
      let current_panel = self.panels_manager.current_panel();
      current_panel
        .selected
        .par_iter()
        .map(|i| current_panel.entries_manager.entries[*i].path.clone())
        .collect()
    };

    self.filter_and_sort();

    {
      let current_panel = self.panels_manager.current_panel();
      let current_path = current_panel
        .entries_manager
        .entry(&current_panel.entries_manager.current_index)
        .unwrap()
        .path
        .clone();

      self.highlight_path(&current_path);
    }

    let current_panel = self.panels_manager.current_panel_mut();
    for p in selected_paths {
      for (id, e) in current_panel.entries_manager.entries.iter().enumerate() {
        if p == e.path
          && let Some(i) = current_panel
            .entries_manager
            .displaying
            .iter()
            .find(|i| **i == id)
        {
          current_panel.selected.insert(*i);
        }
      }
    }
  }

  fn highlight_path(&mut self, path: &Path) {
    let current_panel = self.panels_manager.current_panel_mut();

    current_panel
      .entries_manager
      .displaying
      .iter()
      .enumerate()
      .for_each(|(index, entry_index)| {
        if let Some(entry) = current_panel.entries_manager.entries.get(*entry_index)
          && entry.path == path
        {
          current_panel.entries_manager.current_index = index;
          current_panel.entries_manager.scroll_signal = true;
        }
      });
  }

  pub fn filter_and_sort(&mut self) {
    let current_panel = self.panels_manager.current_panel_mut();

    current_panel.entries_manager.current_index = 0;
    current_panel.entries_manager.displaying.clear();
    current_panel.selected.clear();

    let mut filter = "";
    let view_hidden = self.config.view.view_hidden_files;

    if let Some(kind) = &current_panel.field.kind
      && *kind == FieldKind::Search
    {
      filter = current_panel.field.buffer.trim();
    }

    for (i, entry) in current_panel.entries_manager.entries.iter().enumerate() {
      if !entry.using || (!view_hidden && entry.is_hidden) || !entry.name.contains(filter) {
        continue;
      }

      current_panel.entries_manager.displaying.push(i);
    }

    current_panel.entries_manager.sort(
      &self.config.sorting.sorting_by,
      self.config.sorting.reversed,
    );
  }

  fn delete(&mut self) {
    let current_panel = self.panels_manager.current_panel();

    if let Err(e) = delete(current_panel.selected.iter().map(|entry_index| {
      current_panel
        .entries_manager
        .entries
        .get(*entry_index)
        .unwrap()
        .path
        .as_ref()
    })) {
      self.new_toast("Delete", Cow::Owned(e), ToastKind::Danger, None);
    }
    self.fetch_entries();
  }

  pub fn new_field(&mut self, kind: &FieldKind) {
    let current_panel = self.panels_manager.current_panel_mut();

    let field = &mut current_panel.field;
    field.focused = true;

    if field.kind.is_some() {
      field.buffer = String::new();
      self.filter_and_sort();
      return;
    }

    field.kind = Some(*kind);
  }

  pub fn close_field(&mut self) {
    self.panels_manager.current_panel_mut().field.reset();
    self.filter_and_sort();
  }

  pub fn unfocus_field(&mut self) { self.panels_manager.current_panel_mut().field.unfocus(); }

  pub fn buffer_field(&mut self, buffer: String) {
    self.panels_manager.current_panel_mut().field.buffer(buffer);
  }

  pub fn logic_field(&mut self, kind: &FieldKind) {
    match kind {
      FieldKind::Search => self.filter_and_sort(),
    }
  }

  fn handle_actions(&mut self, action: &KeybindAction, is_ctrled: bool, is_shifted: bool) {
    if self.overlay.kind.is_some() {
      if let KeybindAction::Choice(choice) = action {
        self.finalize_overlay_choice(*choice);
      }
      return;
    }

    let current_panel = self.panels_manager.current_panel();

    if current_panel.field.focused {
      return;
      // block input
    }

    match action {
      KeybindAction::NavigateUp => {
        self.navigate_index(&NavigateDirection::Up, is_ctrled, is_shifted)
      }
      KeybindAction::NavigateDown => {
        self.navigate_index(&NavigateDirection::Down, is_ctrled, is_shifted);
      }
      KeybindAction::NavigateForward => self.nav_forward(),
      KeybindAction::NavigateBackward => self.nav_back(),
      KeybindAction::Copy => {
        if current_panel.selected.is_empty() {
          self.new_toast(
            "Clipboard",
            Cow::Borrowed("nothing is selected to be copied!"),
            ToastKind::Info,
            None,
          );
          return;
        }

        self.new_toast(
          "Clipboard",
          Cow::Owned(format!(
            "successfully added {} items into clipboard!",
            current_panel.selected.len(),
          )),
          ToastKind::Success,
          None,
        );
        self.add_to_clipboard(ClipboardMode::Copy);
      }
      KeybindAction::Cut => {
        if current_panel.selected.is_empty() {
          self.new_toast(
            "Clipboard",
            Cow::Borrowed("nothing is selected to be cut!"),
            ToastKind::Info,
            None,
          );
          return;
        }

        self.new_toast(
          "Clipboard",
          Cow::Owned(format!(
            "successfully added {} items into clipboard!",
            current_panel.selected.len()
          )),
          ToastKind::Success,
          None,
        );
        self.add_to_clipboard(ClipboardMode::Cut);
      }
      KeybindAction::Paste => {
        self.paste();
      }

      KeybindAction::Delete => {
        if current_panel.selected.is_empty() {
          self.new_toast(
            "Delete",
            Cow::Borrowed("nothing is selected to be deleted!"),
            ToastKind::Info,
            None,
          );
          return;
        }

        self.new_overlay(OverlayKind::Delete, None);
      }
      KeybindAction::Rename => self.new_overlay(OverlayKind::Rename, None),
      KeybindAction::ClearClipboard => {
        self.clipboard_manager.reset();
        self.new_toast(
          "Success!",
          Cow::Borrowed("successfully cleared clipboard!"),
          ToastKind::Success,
          None,
        );
      }
      KeybindAction::ToggleHidden => self.toggle_view_hidden(),
      KeybindAction::CreateFile => self.new_overlay(OverlayKind::CreateFile, None),
      KeybindAction::CreateFolder => self.new_overlay(OverlayKind::CreateFolder, None),
      KeybindAction::Info => self.new_overlay(OverlayKind::Metadata, None),
      KeybindAction::Search => self.new_field(&FieldKind::Search),
      KeybindAction::Refresh => {
        self.fetch_config();
        self.fetch_entries();
      }
      _ => {}
    }
  }

  pub fn create(&mut self, mode: CreateType) {
    let overlay = &mut self.overlay;
    let mut content = overlay.buffer.trim();
    if content.starts_with("/") {
      content = &content[1..];
    }

    let current_panel = self.panels_manager.current_panel_mut();

    let try_create = create(&current_panel.current_path, Path::new(content), &mode);

    if let Err(error) = try_create {
      overlay.error.clear();
      overlay.error.push_str(&error);
      return;
    }

    self.overlay.reset();
    self.fetch_entries();
    self.highlight_path(&try_create.unwrap());
  }

  pub fn rename(&mut self) {
    let overlay = &mut self.overlay;
    if overlay.kind.unwrap() != OverlayKind::Rename {
      return;
    }

    for char in BANNED_CHARACTERS {
      if overlay.buffer.trim().contains(char) {
        overlay.error.clear();
        overlay.error.push_str("Containing invalid characters!");
        return;
      }
    }

    let rename_res = rename(overlay.path.as_ref().unwrap(), &overlay.buffer);

    if let Err(err) = &rename_res {
      self.new_toast(
        "Rename",
        Cow::Owned(err.to_string()),
        ToastKind::Danger,
        None,
      );
    }

    self.overlay.reset();
    self.fetch_entries();
    self.highlight_path(&rename_res.unwrap());
  }

  fn new_toast(
    &self,
    title: &'static str,
    content: Cow<'static, str>,
    kind: ToastKind,
    id_chan: Option<mpsc::Sender<Instant>>,
  ) {
    let toasts = Arc::clone(&self.toasts_manager.toasts);
    let view_conf = &self.config.view;

    let duration = match kind {
      ToastKind::Info => Some(Duration::from_millis(view_conf.info_toast_time)),
      ToastKind::Success => Some(Duration::from_millis(view_conf.success_toast_time)),
      ToastKind::Danger => Some(Duration::from_millis(view_conf.danger_toast_time)),
      ToastKind::Operation => None,
    };

    tokio::spawn(async move {
      let id_chan = id_chan;
      let id = {
        let mut list = toasts.write();
        let mut toast = list
          .iter_mut()
          .find(|t| t.title == title && t.content == content);

        if let Some(toast) = toast.as_mut() {
          toast.count += 1;
          toast.start_time = Instant::now();

          toast.id
        } else {
          let toast = Toast {
            title,
            content,
            kind,
            duration,
            ..Default::default()
          };
          let id = toast.id;
          list.push(toast);

          list.par_sort_by(|a, b| {
            let (x, y) = (
              a.kind == ToastKind::Operation,
              b.kind == ToastKind::Operation,
            );
            x.cmp(&y)
          });

          id
        }
      };

      if let Some(duration) = duration {
        tokio::time::sleep(duration).await;

        let mut list = toasts.write();
        let toast = list.iter_mut().find(|t| t.id == id);

        if let Some(toast) = toast
          && toast.count > 1
        {
          toast.count -= 1;
          return;
        }

        list.retain(|t| t.id != id);
      }

      if let Some(chan) = id_chan {
        let _ = chan.send(id);
      }
    });
  }

  pub fn new_overlay(&mut self, kind: OverlayKind, path: Option<&Path>) {
    let overlay = &mut self.overlay;
    overlay.kind = Some(kind);

    match kind {
      OverlayKind::Rename => {
        let current_panel = self.panels_manager.current_panel();

        let path = &current_panel
          .entries_manager
          .entry(&current_panel.entries_manager.current_index)
          .unwrap()
          .path;

        overlay.path = Some(path.to_path_buf());
        overlay.buffer = path.file_name().unwrap().to_str().unwrap().to_string();
      }
      OverlayKind::Metadata => {
        let current_panel = self.panels_manager.current_panel();
        let metadata_conf = &self.config.view.metadata;
        let selected_entry = current_panel
          .entries_manager
          .entry(&current_panel.entries_manager.current_index)
          .unwrap();
        let mut new_entry = Entry {
          name: selected_entry.name.clone(),
          file_type: selected_entry.file_type,
          path: selected_entry.path.clone(),
          ..Default::default()
        };

        if let Some(t) = selected_entry.accessed
          && let Some(p) = selected_entry.created
        {
          new_entry.accessed = Some(t);
          new_entry.created = Some(p);
        } else {
          let (acc, cr) = accessed_and_created(
            &selected_entry.path,
            &metadata_conf.contains(&Property::Accessed),
            &metadata_conf.contains(&Property::Created),
          );
          new_entry.accessed = acc;
          new_entry.created = cr;
        }

        if let Some(t) = selected_entry.file_size {
          new_entry.file_size = Some(t);
          new_entry.folder_size = selected_entry.folder_size;
        } else {
          let fetch_size = &metadata_conf.contains(&Property::Size);
          new_entry.file_size = file_size(&selected_entry.path, fetch_size);
          new_entry.folder_size = folder_size(&selected_entry.path, fetch_size);
        }

        overlay.entry = Some(new_entry);
      }
      OverlayKind::Paste => {
        if let Some(p) = path {
          overlay.path = Some(p.to_path_buf());
        }
      }
      _ => {}
    }
  }

  pub fn finalize_overlay_choice(&mut self, choice: usize) {
    match self.overlay.kind.unwrap() {
      OverlayKind::Paste => {
        let response_chan = &self.channels_manager.channels_queue
          [self.channels_manager.queued_chan_index]
          .response_chan;

        if choice == 0 {
          let _ = response_chan.send(PasteKind::Replace);
        } else if choice == 1 {
          let _ = response_chan.send(PasteKind::Duplicate);
        }
      }
      OverlayKind::Delete => {
        if choice != 0 {
          self.overlay.reset();
        }

        self.delete();
      }
      _ => {}
    }

    self.overlay.reset();
  }

  pub fn add_to_clipboard(&mut self, clipboard_mode: ClipboardMode) {
    use wl_clipboard_rs::copy::{MimeType, Options, Source};

    if self.config.clipboard.behaviour == ClipboardBehaviour::Replace {
      self.clipboard_manager.entries.clear();
    }

    let is_copy = clipboard_mode == ClipboardMode::Copy;
    let current_panel = self.panels_manager.current_panel();

    for i in &current_panel.selected {
      let path = current_panel
        .entries_manager
        .entries
        .get(*i)
        .unwrap()
        .path
        .clone();

      if is_copy && path.is_file() {
        let opts = Options::new();
        let buf = fs::read(&path);
        if buf.is_err() {
          self.new_toast(
            "System Clipboard",
            Cow::Owned(format!(
              "failed to read file contents for {}",
              path.display()
            )),
            ToastKind::Info,
            None,
          );
          continue;
        }
        opts
          .copy(
            Source::Bytes(fs::read(&path).unwrap().into()),
            MimeType::Autodetect,
          )
          .unwrap();
      }

      let _ = self.clipboard_manager.entries.insert(path);
    }

    self.clipboard_manager.mode = Some(clipboard_mode);
  }

  pub fn paste(&mut self) {
    if let Some(mode) = &self.clipboard_manager.mode {
      let (worker_tx, worker_rx) = mpsc::channel::<PasteKind>(); // main use - worker recv
      let (user_tx, user_rx) = mpsc::channel::<WorkerRequest>(); // main recv - worker use

      let entries = self.clipboard_manager.entries.clone();

      let (id_tx, id_rx) = mpsc::channel::<Instant>();
      self.new_toast(
        "Pasting",
        Cow::Owned(format!("Pasting {} items", entries.len())),
        ToastKind::Operation,
        Some(id_tx),
      );
      let id = id_rx.recv().unwrap();

      self.channels_manager.channels_queue.push(WorkerChannels {
        request_chan: user_rx,
        response_chan: worker_tx,
        id,
      });

      let current_panel = self.panels_manager.current_panel();
      let current_path = current_panel.current_path.clone();

      match mode {
        ClipboardMode::Copy => {
          std::thread::spawn(move || {
            copy_dir(entries, current_path, &user_tx, &worker_rx);
          });
        }
        ClipboardMode::Cut => {
          std::thread::spawn(move || {
            move_dir(entries, current_path, &user_tx, &worker_rx);
          });
        }
      }
      return;
    };

    // if there's nothing in app's clipboard, use wayland's selection (clipboard)
    // NOTE: i will get this working soon, i promise
    use std::io::Read;

    use wl_clipboard_rs::paste::{ClipboardType, Error, MimeType, Seat, get_contents};
    let result = get_contents(ClipboardType::Regular, Seat::Unspecified, MimeType::Any);

    match result {
      Ok((mut pipe, mime_type)) => {
        println!("mime-type: {}", mime_type);
        let mut contents = vec![];
        pipe.read_to_end(&mut contents).unwrap();
        println!("{}", String::from_utf8_lossy(&contents));
      }
      Err(Error::NoSeats) | Err(Error::ClipboardEmpty) | Err(Error::NoMimeType) => {
        println!("boog");
      }
      Err(_err) => {
        println!("goog");
      }
    };
  }

  fn navigate_index(&mut self, direction: &NavigateDirection, is_ctrled: bool, is_shifted: bool) {
    let current_panel = self.panels_manager.current_panel_mut();
    let mut current_index: usize = current_panel.entries_manager.current_index;

    match direction {
      NavigateDirection::Down => {
        if current_index < current_panel.entries_manager.displaying.len() - 1 {
          current_index += 1;
        }
      }
      NavigateDirection::Up => {
        if !(current_index == 0) {
          current_index -= 1;
        }
      }
    }

    current_panel.entries_manager.scroll_signal = true;
    self.modify_selected(current_index, is_ctrled, is_shifted);
  }

  pub fn swap_selected(&mut self, index: &usize) {
    // - 1 selected: swapping
    // - >= 2 selected: add to the selected
    let current_panel = self.panels_manager.current_panel_mut();
    let selected = &mut current_panel.selected;

    if let Some(entry_index) = current_panel.entries_manager.displaying.get(*index)
      && !selected.contains(entry_index)
    {
      if selected.len() == 1 {
        selected.clear();
      }
      selected.insert(*entry_index);
    }

    current_panel.entries_manager.current_index = *index;
  }

  pub fn modify_selected(&mut self, index: usize, is_ctrled: bool, is_shifted: bool) {
    let current_panel = self.panels_manager.current_panel_mut();

    if !is_shifted && !is_ctrled {
      current_panel.selected.clear();
    }

    let end_index = if is_shifted {
      current_panel.entries_manager.current_index
    } else {
      index
    };

    if is_shifted {
      for i in index.min(end_index)..=end_index.max(index) {
        current_panel
          .selected
          .insert(*current_panel.entries_manager.displaying.get(i).unwrap());
      } // selecting everything between the two indicies
    }

    let entry_index = current_panel.entries_manager.displaying.get(index).unwrap();
    if is_ctrled {
      if current_panel.selected.contains(entry_index) {
        current_panel.selected.remove(entry_index);
        current_panel.entries_manager.current_index = index;
        return;
      } else {
        current_panel.selected.insert(*entry_index);
      }
    }

    current_panel.entries_manager.current_index = index;
    current_panel.selected.insert(*entry_index);
  }

  pub fn clear_selected(&mut self) { self.panels_manager.current_panel_mut().selected.clear(); }
}

impl Overlay {
  pub fn reset(&mut self) {
    self.kind = None;
    self.error.clear();
    self.buffer = String::new();
  }

  pub fn buffer(&mut self, buffer: String) { self.buffer = buffer; }
}

impl Field {
  pub fn reset(&mut self) {
    self.buffer = String::new();
    self.kind = None;
    self.focused = false;
  }

  pub fn buffer(&mut self, buffer: String) { self.buffer = buffer; }

  pub fn unfocus(&mut self) { self.focused = false; }
}

impl ClipboardManager {
  pub fn reset(&mut self) {
    self.entries.clear();
    self.mode = None;
  }
}

impl eframe::App for App {
  fn logic(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
    let mut action_to_handle: Option<KeybindAction> = None;
    let (mut is_ctrled, mut is_shifted) = (false, false);
    ctx.clone().input_mut(|i| {
      is_ctrled = i.modifiers.ctrl;
      is_shifted = i.modifiers.shift;

      let mut pressed_modifiers = Modifiers::NONE.plus(i.modifiers);
      let mut pressed_key = None;

      for event in &i.events {
        if let Event::Copy = event {
          pressed_modifiers = pressed_modifiers.plus(CTRL);
          pressed_key = Some(Key::C);
        } else if let Event::Cut = event {
          pressed_modifiers = pressed_modifiers.plus(CTRL);
          pressed_key = Some(Key::X);
        } else if let Event::Paste { .. } = event {
          pressed_modifiers = pressed_modifiers.plus(CTRL);
          pressed_key = Some(Key::V);
        }
      }

      for (action, shortcut) in &self.config.keybinds_list {
        if i
          .modifiers
          .matches_logically(pressed_modifiers.plus(shortcut.modifiers))
        {
          if let Some(key) = pressed_key
            && shortcut.logical_key == key
          {
            action_to_handle = Some(*action);
            return;
          } else if i.key_pressed(shortcut.logical_key) {
            action_to_handle = Some(*action);
            return;
          }
        }
      }
    });

    if let Some(action) = action_to_handle {
      self.handle_actions(&action, is_ctrled, is_shifted);
    }

    let mut pending_removal_indicies = vec![];
    let (mut pending_fetch_entries, mut pending_highlight_path, mut pending_using_index) =
      (false, None, None);
    for (i, worker_chans) in self.channels_manager.channels_queue.iter().enumerate() {
      let from_worker = &worker_chans.request_chan;
      if let Ok(req) = from_worker.try_recv() {
        match req {
          WorkerRequest::OperationType { path } => {
            self.new_overlay(OverlayKind::Paste, Some(&path));
            pending_using_index = Some(i);
            break;
          }
          WorkerRequest::Update { percent } => {
            let mut toasts = self.toasts_manager.toasts.write();
            let mut toasts = toasts
              .iter_mut()
              .filter(|t| t.id == worker_chans.id)
              .collect::<Vec<&mut Toast>>();
            let toast: &mut Toast = toasts[0];

            toast.percent = Some(percent);
          }
          WorkerRequest::Done { paths } => {
            pending_fetch_entries = true;
            if !paths.is_empty() {
              pending_highlight_path = Some(paths[0].to_owned());
            }

            for p in paths {
              let current_panel = self.panels_manager.current_panel_mut();

              for (id, e) in current_panel.entries_manager.entries.iter().enumerate() {
                if p == e.path
                  && let Some(i) = current_panel
                    .entries_manager
                    .displaying
                    .iter()
                    .find(|i| **i == id)
                {
                  current_panel.selected.insert(*i);
                }
              }
            }

            if let Some(mode) = &self.clipboard_manager.mode
              && mode == &ClipboardMode::Cut
            {
              self.clipboard_manager.reset();
            }

            let mut toasts = self.toasts_manager.toasts.write();
            toasts.retain(|t| t.id != worker_chans.id);

            pending_removal_indicies.push(i);
          }
        }
      }
    }

    if pending_fetch_entries {
      self.fetch_entries();
    }
    if let Some(path) = pending_highlight_path {
      self.highlight_path(&path);
    }
    if let Some(i) = pending_using_index {
      self.channels_manager.queued_chan_index = i;
    }
  }

  fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) { self.ui(ui); }
}
