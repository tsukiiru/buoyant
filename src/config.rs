use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{env, fs};

use eframe::egui::{Key, KeyboardShortcut, Modifiers};
use serde::Deserialize;

use crate::Property;

pub type Keybind = (KeybindAction, KeyboardShortcut);

#[derive(Clone, Debug)]
pub struct Actions {
    pub copy: KeybindAction,
    pub cut: KeybindAction,
    pub paste: KeybindAction,
}

impl Default for Actions {
    fn default() -> Self {
        Actions {
            copy: KeybindAction::Copy,
            cut: KeybindAction::Cut,
            paste: KeybindAction::Paste,
        }
    }
}

pub struct Config {
    pub keybinds: Keybinds,
    pub keybinds_list: Vec<Keybind>,
    pub sorting: Sorting,
    pub view: View,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            keybinds: Keybinds::default(),
            keybinds_list: Vec::with_capacity(16),
            sorting: Sorting::default(),
            view: View::default(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum KeybindAction {
    NavigateUp,
    NavigateDown,
    NavigateForward,
    NavigateBackward,
    Copy,
    Cut,
    Paste,
    ClearClipboard,
    Delete,
    Rename,
    ToggleHidden,
    CreateFile,
    CreateFolder,
    ToggleVisual,
    Refresh,
    Info,
    Search,
    Choice(usize),
}

pub struct Keybinds {
    pub navigate_up: KeyboardShortcut,
    pub navigate_down: KeyboardShortcut,
    pub navigate_forward: KeyboardShortcut,
    pub navigate_backward: KeyboardShortcut,
    pub copy_to_clipboard: KeyboardShortcut,
    pub cut_to_clipboard: KeyboardShortcut,
    pub paste_from_clipboard: KeyboardShortcut,
    pub clear_clipboard: KeyboardShortcut,
    pub delete_selections: KeyboardShortcut,
    pub rename_file: KeyboardShortcut,
    pub toggle_hidden_view: KeyboardShortcut,
    pub create_file_path: KeyboardShortcut,
    pub create_folder_path: KeyboardShortcut,
    pub toggle_visual_mode: KeyboardShortcut,
    pub refresh: KeyboardShortcut,
    pub view_info: KeyboardShortcut,
    pub search: KeyboardShortcut,
    pub choice_0: KeyboardShortcut,
    pub choice_1: KeyboardShortcut,
    pub choice_2: KeyboardShortcut,
    pub choice_3: KeyboardShortcut,
    pub choice_4: KeyboardShortcut,
    pub choice_5: KeyboardShortcut,
    pub choice_6: KeyboardShortcut,
    pub choice_7: KeyboardShortcut,
    pub choice_8: KeyboardShortcut,
    pub choice_9: KeyboardShortcut,
}

const NONE: Modifiers = Modifiers::NONE;
pub const CTRL: Modifiers = Modifiers::CTRL.plus(Modifiers::COMMAND);
// egui registers ctrl as ctrl + command
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;
const CTRL_SHIFT: Modifiers = Modifiers::CTRL.plus(SHIFT);

impl Default for Keybinds {
    fn default() -> Self {
        Keybinds {
            navigate_up: bind(NONE, Key::ArrowUp),
            navigate_down: bind(NONE, Key::ArrowDown),
            navigate_forward: bind(NONE, Key::ArrowRight),
            navigate_backward: bind(NONE, Key::ArrowLeft),
            copy_to_clipboard: bind(CTRL, Key::C),
            cut_to_clipboard: bind(CTRL, Key::X),
            paste_from_clipboard: bind(CTRL, Key::V),
            clear_clipboard: bind(CTRL_SHIFT, Key::V),
            delete_selections: bind(NONE, Key::Delete),
            rename_file: bind(NONE, Key::F2),
            toggle_hidden_view: bind(CTRL, Key::H),
            create_file_path: bind(CTRL, Key::N),
            create_folder_path: bind(ALT, Key::N),
            toggle_visual_mode: bind(NONE, Key::V),
            refresh: bind(CTRL, Key::R),
            search: bind(NONE, Key::Slash),
            view_info: bind(NONE, Key::F12),
            choice_0: bind(NONE, Key::Num0),
            choice_1: bind(NONE, Key::Num1),
            choice_2: bind(NONE, Key::Num2),
            choice_3: bind(NONE, Key::Num3),
            choice_4: bind(NONE, Key::Num4),
            choice_5: bind(NONE, Key::Num5),
            choice_6: bind(NONE, Key::Num6),
            choice_7: bind(NONE, Key::Num7),
            choice_8: bind(NONE, Key::Num8),
            choice_9: bind(NONE, Key::Num9),
        }
    }
}

fn bind(modifiers: Modifiers, key: Key) -> KeyboardShortcut {
    KeyboardShortcut::new(modifiers, key)
}

pub struct Sorting {
    pub sorting_by: Property,
    pub reversed: bool,
}

impl Default for Sorting {
    fn default() -> Self {
        Sorting {
            sorting_by: Property::Name,
            reversed: false,
        }
    }
}

pub struct View {
    pub explorer: Vec<Property>,
    pub metadata: Vec<Property>,
    pub dark_mode: bool,
    pub view_hidden_files: bool,
    pub format_date: String,
}

impl Default for View {
    fn default() -> Self {
        View {
            explorer: vec![Property::Name],
            metadata: vec![
                Property::Name,
                Property::Type,
                Property::Size,
                Property::Accessed,
                Property::Created,
            ],
            dark_mode: false,
            view_hidden_files: false,
            format_date: String::from("%d/%m/%Y, %I:%M:%S %p"),
        }
    }
}

pub fn fetch(config: &mut Config) {
    let home_dir = env::home_dir();

    if home_dir.is_none() {
        println!("cannot get HOME directory!");
    }

    let config_dir = home_dir.unwrap().join(".config/buoyant/buoyant.toml");
    let read_content = fs::read_to_string(&config_dir);

    if let Ok(content) = read_content {
        let raw_config: RawConfig = toml::from_str(&content).unwrap();
        process_raw_config(&raw_config, config);
    }

    listing_keybinds(&config.keybinds, &mut config.keybinds_list);
}

#[derive(Deserialize)]
struct RawConfig {
    keybinds: Option<RawKeybinds>,
    sorting: Option<RawSorting>,
    view: Option<RawView>,
}

fn process_raw_config(raw_config: &RawConfig, config: &mut Config) {
    if let Some(table) = &raw_config.keybinds {
        process_raw_keybinds(&table, &mut config.keybinds);
    }
    if let Some(table) = &raw_config.sorting {
        process_raw_sorting(&table, &mut config.sorting);
    }
    if let Some(table) = &raw_config.view {
        process_raw_view(&table, &mut config.view);
    }
}

#[derive(Deserialize)]
struct RawKeybinds {
    pub navigate_up: Option<String>,
    pub navigate_down: Option<String>,
    pub navigate_forward: Option<String>,
    pub navigate_backward: Option<String>,
    pub copy_to_clipboard: Option<String>,
    pub cut_to_clipboard: Option<String>,
    pub paste_from_clipboard: Option<String>,
    pub clear_clipboard: Option<String>,
    pub delete_selections: Option<String>,
    pub rename_file: Option<String>,
    pub toggle_hidden_view: Option<String>,
    pub create_file_path: Option<String>,
    pub create_folder_path: Option<String>,
    pub toggle_visual_mode: Option<String>,
    pub refresh: Option<String>,
    pub search: Option<String>,
    pub view_info: Option<String>,
    pub choice_0: Option<String>,
    pub choice_1: Option<String>,
    pub choice_2: Option<String>,
    pub choice_3: Option<String>,
    pub choice_4: Option<String>,
    pub choice_5: Option<String>,
    pub choice_6: Option<String>,
    pub choice_7: Option<String>,
    pub choice_8: Option<String>,
    pub choice_9: Option<String>,
}

fn process_raw_keybinds(raw_keybinds: &RawKeybinds, kb_config: &mut Keybinds) {
    if let Some(key_str) = &raw_keybinds.navigate_up
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.navigate_up = fresh_key;
    }
    if let Some(key_str) = &raw_keybinds.navigate_down
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.navigate_down = fresh_key;
    }
    if let Some(key_str) = &raw_keybinds.navigate_forward
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.navigate_forward = fresh_key;
    }
    if let Some(key_str) = &raw_keybinds.navigate_backward
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.navigate_backward = fresh_key;
    }
    if let Some(key_str) = &raw_keybinds.copy_to_clipboard
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.copy_to_clipboard = fresh_key;
    }
    if let Some(key_str) = &raw_keybinds.cut_to_clipboard
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.cut_to_clipboard = fresh_key;
    }
    if let Some(key_str) = &raw_keybinds.paste_from_clipboard
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.paste_from_clipboard = fresh_key;
    }
    if let Some(key_str) = &raw_keybinds.clear_clipboard
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.clear_clipboard = fresh_key;
    }
    if let Some(key_str) = &raw_keybinds.delete_selections
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.delete_selections = fresh_key;
    }
    if let Some(key_str) = &raw_keybinds.rename_file
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.rename_file = fresh_key;
    }
    if let Some(key_str) = &raw_keybinds.toggle_hidden_view
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.toggle_hidden_view = fresh_key;
    }
    if let Some(key_str) = &raw_keybinds.create_file_path
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.create_file_path = fresh_key;
    }
    if let Some(key_str) = &raw_keybinds.create_folder_path
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.create_folder_path = fresh_key;
    }
    if let Some(key_str) = &raw_keybinds.toggle_visual_mode
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.toggle_visual_mode = fresh_key;
    }
    if let Some(key_str) = &raw_keybinds.refresh
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.refresh = fresh_key;
    }
    if let Some(key_str) = &raw_keybinds.search
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.search = fresh_key
    }
    if let Some(key_str) = &raw_keybinds.view_info
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.view_info = fresh_key
    }
    if let Some(key_str) = &raw_keybinds.choice_0
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.choice_0 = fresh_key
    }

    if let Some(key_str) = &raw_keybinds.choice_1
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.choice_1 = fresh_key
    }

    if let Some(key_str) = &raw_keybinds.choice_2
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.choice_2 = fresh_key
    }

    if let Some(key_str) = &raw_keybinds.choice_3
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.choice_3 = fresh_key
    }

    if let Some(key_str) = &raw_keybinds.choice_4
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.choice_4 = fresh_key
    }

    if let Some(key_str) = &raw_keybinds.choice_5
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.choice_5 = fresh_key
    }

    if let Some(key_str) = &raw_keybinds.choice_6
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.choice_6 = fresh_key
    }

    if let Some(key_str) = &raw_keybinds.choice_7
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.choice_7 = fresh_key
    }

    if let Some(key_str) = &raw_keybinds.choice_8
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.choice_8 = fresh_key
    }

    if let Some(key_str) = &raw_keybinds.choice_9
        && let Some(fresh_key) = match_key(key_str)
    {
        kb_config.choice_9 = fresh_key
    }
}

fn match_key(raw_key: &str) -> Option<KeyboardShortcut> {
    // keybind format: [whatever modifiers you have here, separated by "+"] + [main key (the last one)]
    let raw_key = raw_key.to_lowercase();
    let mut splitted = raw_key.split("+").map(|s| s.trim()).collect::<Vec<&str>>();

    if splitted.len() <= 0 {
        println!("keybind cannot be 0 character long");
        return None;
    }

    let mut result = bind(NONE, Key::F35);
    let raw_key = splitted.pop().unwrap();

    result.logical_key = match raw_key.to_lowercase().as_str() {
        "a" => Key::A,
        "b" => Key::B,
        "c" => Key::C,
        "d" => Key::D,
        "e" => Key::E,
        "f" => Key::F,
        "g" => Key::G,
        "h" => Key::H,
        "i" => Key::I,
        "j" => Key::J,
        "k" => Key::K,
        "l" => Key::L,
        "m" => Key::M,
        "n" => Key::N,
        "o" => Key::O,
        "p" => Key::P,
        "q" => Key::Q,
        "r" => Key::R,
        "t" => Key::T,
        "u" => Key::U,
        "v" => Key::V,
        "y" => Key::Y,
        "w" => Key::W,
        "z" => Key::Z,
        "arrowup" => Key::ArrowUp,
        "arrowdown" => Key::ArrowDown,
        "arrowright" => Key::ArrowRight,
        "arrowleft" => Key::ArrowLeft,
        "`" => Key::Backtick,
        "[" => Key::OpenBracket,
        "]" => Key::CloseBracket,
        "," => Key::Comma,
        "=" => Key::Equals,
        "-" => Key::Minus,
        "." => Key::Period,
        "'" => Key::Quote,
        ";" => Key::Semicolon,
        "/" => Key::Slash,
        "?" => Key::Questionmark,
        "|" => Key::Pipe,
        "backspace" => Key::Backspace,
        "enter" => Key::Enter,
        "space" => Key::Space,
        "tab" => Key::Tab,
        "delete" => Key::Delete,
        "end" => Key::End,
        "home" => Key::Home,
        "insert" => Key::Insert,
        "pagedown" => Key::PageDown,
        "pageup" => Key::PageUp,
        "escape" => Key::Escape,
        "0" => Key::Num0,
        "1" => Key::Num1,
        "2" => Key::Num2,
        "3" => Key::Num3,
        "4" => Key::Num4,
        "5" => Key::Num5,
        "6" => Key::Num6,
        "7" => Key::Num7,
        "8" => Key::Num8,
        "9" => Key::Num9,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        "f13" => Key::F13,
        "f14" => Key::F14,
        "f15" => Key::F15,
        "f16" => Key::F16,
        "f17" => Key::F17,
        "f18" => Key::F18,
        "f19" => Key::F19,
        "f20" => Key::F20,
        "f21" => Key::F21,
        "f22" => Key::F22,
        "f23" => Key::F23,
        "f24" => Key::F24,
        "f25" => Key::F25,
        "f26" => Key::F26,
        "f27" => Key::F27,
        "f28" => Key::F28,
        "f29" => Key::F29,
        "f30" => Key::F30,
        "f31" => Key::F31,
        "f32" => Key::F32,
        "f33" => Key::F33,
        "f34" => Key::F34,
        "f35" => Key::F35,
        _ => Key::F35,
    };

    let raw_modifiers = splitted;

    if raw_modifiers.is_empty() {
        return Some(result);
    }

    let fresh_modifiers = NONE;

    for raw_mod in raw_modifiers.iter() {
        fresh_modifiers.plus(match *raw_mod {
            "ctrl" => CTRL,
            "shift" => SHIFT,
            "alt" => ALT,
            _ => NONE,
        });
    }

    result.modifiers = fresh_modifiers;

    Some(result)
}

fn listing_keybinds(keybinds: &Keybinds, list: &mut Vec<Keybind>) {
    list.clear();

    list.push((KeybindAction::NavigateUp, keybinds.navigate_up));
    list.push((KeybindAction::NavigateDown, keybinds.navigate_down));
    list.push((KeybindAction::NavigateForward, keybinds.navigate_forward));
    list.push((KeybindAction::NavigateBackward, keybinds.navigate_backward));

    list.push((KeybindAction::Copy, keybinds.copy_to_clipboard));
    list.push((KeybindAction::Cut, keybinds.cut_to_clipboard));
    list.push((KeybindAction::Paste, keybinds.paste_from_clipboard));
    list.push((KeybindAction::ClearClipboard, keybinds.clear_clipboard));

    list.push((KeybindAction::Delete, keybinds.delete_selections));
    list.push((KeybindAction::Rename, keybinds.rename_file));
    list.push((KeybindAction::ToggleHidden, keybinds.toggle_hidden_view));

    list.push((KeybindAction::CreateFile, keybinds.create_file_path));
    list.push((KeybindAction::CreateFolder, keybinds.create_folder_path));

    list.push((KeybindAction::ToggleVisual, keybinds.toggle_visual_mode));
    list.push((KeybindAction::Refresh, keybinds.refresh));
    list.push((KeybindAction::Search, keybinds.search));
    list.push((KeybindAction::Info, keybinds.view_info));
    list.push((KeybindAction::Choice(0), keybinds.choice_0));
    list.push((KeybindAction::Choice(1), keybinds.choice_1));
    list.push((KeybindAction::Choice(2), keybinds.choice_2));
    list.push((KeybindAction::Choice(3), keybinds.choice_3));
    list.push((KeybindAction::Choice(4), keybinds.choice_4));
    list.push((KeybindAction::Choice(5), keybinds.choice_5));
    list.push((KeybindAction::Choice(6), keybinds.choice_6));
    list.push((KeybindAction::Choice(7), keybinds.choice_7));
    list.push((KeybindAction::Choice(8), keybinds.choice_8));
    list.push((KeybindAction::Choice(9), keybinds.choice_9));
}

#[derive(Deserialize)]
struct RawSorting {
    sorting_by: Option<String>,
    reversed: Option<bool>,
}

fn process_raw_sorting(raw_sorting: &RawSorting, sorting_config: &mut Sorting) {
    if let Some(by) = &raw_sorting.sorting_by {
        sorting_config.sorting_by = match_property(&by);
    }
    if let Some(bol) = &raw_sorting.reversed {
        sorting_config.reversed = bol.to_owned();
    }
}

#[derive(Deserialize)]
struct RawView {
    explorer: Option<Vec<String>>,
    metadata: Option<Vec<String>>,
    dark_mode: Option<bool>,
    view_hidden_files: Option<bool>,
    format_date: Option<String>,
}

fn process_raw_view(raw_view: &RawView, view_conf: &mut View) {
    if let Some(list) = &raw_view.explorer {
        view_conf.explorer = list.par_iter().map(|i| match_property(&i)).collect();
    }
    if let Some(list) = &raw_view.metadata {
        view_conf.metadata = list.par_iter().map(|i| match_property(&i)).collect();
    }
    if let Some(b) = &raw_view.dark_mode {
        view_conf.dark_mode = *b;
    }
    if let Some(b) = &raw_view.view_hidden_files {
        view_conf.view_hidden_files = *b;
    }
    if let Some(s) = &raw_view.format_date {
        view_conf.format_date = s.to_string();
    }
}

fn match_property(input: &str) -> Property {
    match input.to_lowercase().as_str() {
        "name" => Property::Name,
        "accessed" => Property::Accessed,
        "created" => Property::Created,
        "type" => Property::Type,
        "size" => Property::Size,
        "path" => Property::Path,
        _ => Property::Name,
    }
}
