use rayon::{
    iter::{IntoParallelRefIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use std::{env, fs};

use eframe::egui::{Key, KeyboardShortcut, Modifiers};
use serde::Deserialize;

use crate::app::Property;

pub type Keybind = (KeybindAction, KeyboardShortcut);

pub struct Config {
    pub keybinds: Keybinds,
    pub keybinds_list: Vec<Keybind>,
    pub sorting: Sorting,
    pub view: View,
    pub clipboard: ClipboardConf,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            keybinds: Keybinds::default(),
            keybinds_list: Vec::with_capacity(27),
            // NOTE: update allocation size matching the number of keybinds
            sorting: Sorting::default(),
            view: View::default(),
            clipboard: ClipboardConf::default(),
        }
    }
}

impl Config {
    pub fn should_fetch(&self, property: Property) -> bool {
        self.view.explorer.contains(&property) || self.sorting.sorting_by == property
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

macro_rules! create_keybinds {
    ($($field:ident),* $(,)?) => {
        pub struct Keybinds {
        $(
            pub $field: KeyboardShortcut,
        )*
        }
    };
}

create_keybinds!(
    navigate_up,
    navigate_down,
    navigate_forward,
    navigate_backward,
    copy_to_clipboard,
    cut_to_clipboard,
    paste_from_clipboard,
    clear_clipboard,
    delete_selections,
    rename_file,
    toggle_hidden_view,
    create_file_path,
    create_folder_path,
    toggle_visual_mode,
    refresh,
    view_info,
    search,
    choice_0,
    choice_1,
    choice_2,
    choice_3,
    choice_4,
    choice_5,
    choice_6,
    choice_7,
    choice_8,
    choice_9,
);

const NONE: Modifiers = Modifiers::NONE;
pub const CTRL: Modifiers = Modifiers::CTRL;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;
const CTRL_SHIFT: Modifiers = CTRL.plus(SHIFT);

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

#[derive(Default)]
pub struct Sorting {
    pub sorting_by: Property,
    pub reversed: bool,
}

pub struct View {
    pub explorer: Vec<Property>,
    pub metadata: Vec<Property>,
    pub dark_mode: bool,
    pub view_hidden_files: bool,
    pub format_date: String,
    pub info_toast_time: u64,
    pub danger_toast_time: u64,
    pub success_toast_time: u64,
}

impl Default for View {
    fn default() -> Self {
        View {
            explorer: vec![Property::Name, Property::Size, Property::Type],
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
            info_toast_time: 5_000,
            danger_toast_time: 7_000,
            success_toast_time: 3_000,
        }
    }
}

#[derive(Default, PartialEq)]
pub enum ClipboardBehaviour {
    #[default]
    Replace,
    Addition,
}

#[derive(Default)]
pub struct ClipboardConf {
    pub behaviour: ClipboardBehaviour,
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
    clipboard: Option<RawClipboard>,
}

fn process_raw_config(raw_config: &RawConfig, config: &mut Config) {
    macro_rules! process_field {
        ($n:ident, $f:expr) => {
            if let Some(table) = &raw_config.$n {
                $f(table, &mut config.$n);
            }
        };
    }

    process_field!(keybinds, process_raw_keybinds);
    process_field!(sorting, process_raw_sorting);
    process_field!(view, process_raw_view);
    process_field!(clipboard, process_raw_clipboard);
}

// create an entry for every keybinds as Option<String>
macro_rules! create_raw_keybinds {
    ($($field:ident),* $(,)?) => {
        #[derive(Deserialize)]
        struct RawKeybinds {
        $(
            $field: Option<String>,
        )*
        }
    };
}

create_raw_keybinds!(
    navigate_up,
    navigate_down,
    navigate_forward,
    navigate_backward,
    copy_to_clipboard,
    cut_to_clipboard,
    paste_from_clipboard,
    clear_clipboard,
    delete_selections,
    rename_file,
    toggle_hidden_view,
    create_file_path,
    create_folder_path,
    toggle_visual_mode,
    refresh,
    view_info,
    search,
    choice_0,
    choice_1,
    choice_2,
    choice_3,
    choice_4,
    choice_5,
    choice_6,
    choice_7,
    choice_8,
    choice_9,
);

fn process_raw_keybinds(raw_config: &RawKeybinds, config: &mut Keybinds) {
    macro_rules! process_field {
        ($n:ident) => {
            if let Some(v) = &raw_config.$n
                && let Some(keybind) = match_key(v)
            {
                config.$n = keybind;
            }
        };
    }

    process_field!(navigate_up);
    process_field!(navigate_down);
    process_field!(navigate_forward);
    process_field!(navigate_backward);
    process_field!(copy_to_clipboard);
    process_field!(cut_to_clipboard);
    process_field!(paste_from_clipboard);
    process_field!(clear_clipboard);
    process_field!(delete_selections);
    process_field!(rename_file);
    process_field!(toggle_hidden_view);
    process_field!(create_file_path);
    process_field!(create_folder_path);
    process_field!(toggle_visual_mode);
    process_field!(refresh);
    process_field!(view_info);
    process_field!(search);
    process_field!(choice_0);
    process_field!(choice_1);
    process_field!(choice_2);
    process_field!(choice_3);
    process_field!(choice_4);
    process_field!(choice_5);
    process_field!(choice_6);
    process_field!(choice_7);
    process_field!(choice_8);
    process_field!(choice_9);
}

fn match_key(raw_key: &str) -> Option<KeyboardShortcut> {
    // keybind format: [whatever modifiers you have here, separated by "+"] + [main key (the last one)]
    let raw_key = raw_key.to_lowercase();
    let mut splitted = raw_key.split("+").map(|s| s.trim()).collect::<Vec<&str>>();

    if splitted.is_empty() {
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

    list.par_sort_by(|a, b| {
        let (x, y) = (count_mod(&a.1.modifiers), count_mod(&b.1.modifiers));
        y.cmp(&x)
    });
}

fn count_mod(modifiers: &Modifiers) -> u8 {
    let mut count = 0;
    if modifiers.contains(CTRL) {
        count += 1;
    }
    if modifiers.contains(SHIFT) {
        count += 1;
    }
    if modifiers.contains(ALT) {
        count += 1;
    }
    count
}

#[derive(Deserialize)]
struct RawSorting {
    sorting_by: Option<String>,
    reversed: Option<bool>,
}

fn process_raw_sorting(raw_config: &RawSorting, config: &mut Sorting) {
    macro_rules! process_match {
        ($n:ident) => {
            if let Some(v) = &raw_config.$n {
                config.$n = match_property(v);
            }
        };
    }
    macro_rules! process_owned {
        ($n:ident) => {
            if let Some(v) = &raw_config.$n {
                config.$n = v.to_owned();
            }
        };
    }

    process_match!(sorting_by);
    process_owned!(reversed);
}

#[derive(Deserialize)]
struct RawView {
    explorer: Option<Vec<String>>,
    metadata: Option<Vec<String>>,
    format_date: Option<String>,
    dark_mode: Option<bool>,
    view_hidden_files: Option<bool>,
    info_toast_time: Option<u64>,
    danger_toast_time: Option<u64>,
    success_toast_time: Option<u64>,
}

fn process_raw_view(raw_config: &RawView, config: &mut View) {
    macro_rules! process_match {
        ($n:ident) => {
            if let Some(v) = &raw_config.$n {
                config.$n = v.par_iter().map(|i| match_property(i)).collect();
            }
        };
    }
    macro_rules! process_deref {
        ($n:ident) => {
            if let Some(v) = &raw_config.$n {
                config.$n = *v;
            }
        };
    }
    macro_rules! process_owned {
        ($n:ident) => {
            if let Some(v) = &raw_config.$n {
                config.$n = v.to_owned();
            }
        };
    }

    process_match!(explorer);
    process_match!(metadata);
    process_deref!(dark_mode);
    process_deref!(view_hidden_files);
    process_owned!(format_date);
    process_deref!(danger_toast_time);
    process_deref!(info_toast_time);
    process_deref!(success_toast_time);
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

#[derive(Deserialize)]
struct RawClipboard {
    behaviour: Option<String>,
}

fn process_raw_clipboard(raw_config: &RawClipboard, config: &mut ClipboardConf) {
    if let Some(i) = &raw_config.behaviour {
        config.behaviour = match_behav(i);
    }
}

fn match_behav(i: &str) -> ClipboardBehaviour {
    match i {
        "replace" => ClipboardBehaviour::Replace,
        "add" => ClipboardBehaviour::Addition,
        _ => ClipboardBehaviour::default(),
    }
}
