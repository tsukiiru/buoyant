// "prepare for matching hell" - tsuki May 29th 2026
//
use iced::keyboard::{
    Modifiers,
    key::{Code, Physical},
};
use serde::Deserialize;
use std::{env::home_dir, fs};

use crate::Displaying;

pub struct Config {
    pub keybinds: KeybindsConfig,
    pub view: Displaying,
}

pub struct KeybindsConfig {
    pub navigate_up: KeybindEntry,
    pub navigate_down: KeybindEntry,
    pub navigate_forward: KeybindEntry,
    pub navigate_backward: KeybindEntry,
    pub copy_to_clipboard: KeybindEntry,
    pub cut_to_clipboard: KeybindEntry,
    pub paste_from_clipboard: KeybindEntry,
    pub delete_selections: KeybindEntry,
    pub rename_file: KeybindEntry,
    pub toggle_hidden_view: KeybindEntry,
    pub create_file_path: KeybindEntry,
    pub create_folder_path: KeybindEntry,
    pub toggle_visual_mode: KeybindEntry,
}

pub struct KeybindEntry {
    pub key: Physical,
    pub modifiers: Modifiers,
}

pub fn get_keybinds() -> Config {
    // default config
    let mut config = Config {
        view: Displaying {
            ..Default::default()
        },
        keybinds: KeybindsConfig {
            navigate_up: KeybindEntry {
                key: Physical::Code(Code::ArrowUp),
                modifiers: Modifiers::NONE,
            },
            navigate_down: KeybindEntry {
                key: Physical::Code(Code::ArrowDown),
                modifiers: Modifiers::NONE,
            },
            navigate_backward: KeybindEntry {
                key: Physical::Code(Code::ArrowLeft),
                modifiers: Modifiers::NONE,
            },
            navigate_forward: KeybindEntry {
                key: Physical::Code(Code::ArrowRight),
                modifiers: Modifiers::NONE,
            },
            copy_to_clipboard: KeybindEntry {
                key: Physical::Code(Code::KeyC),
                modifiers: Modifiers::CTRL,
            },
            cut_to_clipboard: KeybindEntry {
                key: Physical::Code(Code::KeyX),
                modifiers: Modifiers::CTRL,
            },
            paste_from_clipboard: KeybindEntry {
                key: Physical::Code(Code::KeyV),
                modifiers: Modifiers::CTRL,
            },
            delete_selections: KeybindEntry {
                key: Physical::Code(Code::Delete),
                modifiers: Modifiers::NONE,
            },
            rename_file: KeybindEntry {
                key: Physical::Code(Code::F2),
                modifiers: Modifiers::NONE,
            },
            toggle_hidden_view: KeybindEntry {
                key: Physical::Code(Code::KeyH),
                modifiers: Modifiers::CTRL,
            },
            create_file_path: KeybindEntry {
                key: Physical::Code(Code::KeyN),
                modifiers: Modifiers::CTRL,
            },
            create_folder_path: KeybindEntry {
                key: Physical::Code(Code::KeyN),
                modifiers: Modifiers::ALT,
            },
            toggle_visual_mode: KeybindEntry {
                key: Physical::Code(Code::KeyV),
                modifiers: Modifiers::NONE,
            },
        },
    };

    let home = home_dir();

    if home.is_none() {
        println!("cannot get home directory!");
        println!("please check HOME environment variable and set it properly");
    }

    let config_dir = home.unwrap().join(".config/buoyant/");
    let config_file = config_dir.join("buoyant.toml");
    let read_content = fs::read_to_string(&config_file);

    if let Ok(content) = read_content {
        let raw_config: RawConfig = toml::from_str(&content).unwrap();
        process_rawconfig(raw_config, &mut config);
    }

    config
}

#[derive(Deserialize)]
struct RawConfig {
    keybinds: Option<RawKeybindsConfig>,
    view: Option<RawViewConfig>,
}

#[derive(Deserialize)]
struct RawKeybindsConfig {
    navigate_up: Option<String>,
    navigate_down: Option<String>,
    navigate_forward: Option<String>,
    navigate_backward: Option<String>,
    copy_to_clipboard: Option<String>,
    cut_to_clipboard: Option<String>,
    paste_from_clipboard: Option<String>,
    delete_selections: Option<String>,
    rename_file: Option<String>,
    toggle_hidden_view: Option<String>,
    create_file_path: Option<String>,
    create_folder_path: Option<String>,
    toggle_visual_mode: Option<String>,
}

#[derive(Deserialize)]
struct RawViewConfig {
    hidden: Option<bool>,
    last_accessed: Option<bool>,
    created: Option<bool>,
    filetype: Option<bool>,
    filesize: Option<bool>,
}

fn process_rawconfig(raw_config: RawConfig, config: &mut Config) {
    if let Some(table) = raw_config.keybinds {
        process_keybinds(table, &mut config.keybinds);
    }

    if let Some(table) = raw_config.view {
        process_view(table, &mut config.view);
    }
}

fn process_view(raw_view: RawViewConfig, config: &mut Displaying) {
    if let Some(conf) = raw_view.hidden {
        config.hidden = conf;
    }

    if let Some(conf) = raw_view.last_accessed {
        config.last_accessed = conf;
    }

    if let Some(conf) = raw_view.created {
        config.created = conf;
    }

    if let Some(conf) = raw_view.filetype {
        config.filetype = conf;
    }

    if let Some(conf) = raw_view.filesize {
        config.filesize = conf;
    }
}

fn match_key(raw_key: String) -> Option<KeybindEntry> {
    // keybind format: [whatever modifiers you have here, separated by "+"] + [main key (the last one)]
    let raw_key = raw_key.to_lowercase();
    let mut splitted = raw_key
        .split("+") // split
        .map(|s| s.trim()) // trim
        .collect::<Vec<&str>>();

    if splitted.len() <= 0 {
        println!("keybind cannot be 0 character long....");
        return None;
    }

    let mut result = KeybindEntry {
        key: Physical::Code(Code::F35),
        modifiers: Modifiers::NONE,
    };

    let raw_key = splitted.pop().unwrap();

    result.key = match raw_key.to_lowercase().as_str() {
        "a" => Physical::Code(Code::KeyA),
        "b" => Physical::Code(Code::KeyB),
        "c" => Physical::Code(Code::KeyC),
        "d" => Physical::Code(Code::KeyD),
        "e" => Physical::Code(Code::KeyE),
        "f" => Physical::Code(Code::KeyF),
        "g" => Physical::Code(Code::KeyG),
        "h" => Physical::Code(Code::KeyH),
        "i" => Physical::Code(Code::KeyI),
        "j" => Physical::Code(Code::KeyJ),
        "k" => Physical::Code(Code::KeyK),
        "l" => Physical::Code(Code::KeyL),
        "m" => Physical::Code(Code::KeyM),
        "n" => Physical::Code(Code::KeyN),
        "o" => Physical::Code(Code::KeyO),
        "p" => Physical::Code(Code::KeyP),
        "q" => Physical::Code(Code::KeyQ),
        "r" => Physical::Code(Code::KeyR),
        "t" => Physical::Code(Code::KeyT),
        "u" => Physical::Code(Code::KeyU),
        "v" => Physical::Code(Code::KeyV),
        "y" => Physical::Code(Code::KeyY),
        "w" => Physical::Code(Code::KeyW),
        "z" => Physical::Code(Code::KeyZ),
        "arrowup" => Physical::Code(Code::ArrowUp),
        "arrowdown" => Physical::Code(Code::ArrowDown),
        "arrowright" => Physical::Code(Code::ArrowRight),
        "arrowleft" => Physical::Code(Code::ArrowLeft),
        "`" => Physical::Code(Code::Backquote),
        "[" => Physical::Code(Code::BracketLeft),
        "]" => Physical::Code(Code::BracketRight),
        "," => Physical::Code(Code::Comma),
        "=" => Physical::Code(Code::Equal),
        "-" => Physical::Code(Code::Minus),
        "." => Physical::Code(Code::Period),
        "'" => Physical::Code(Code::Quote),
        ";" => Physical::Code(Code::Semicolon),
        "/" => Physical::Code(Code::Slash),
        "backspace" => Physical::Code(Code::Backspace),
        "enter" => Physical::Code(Code::Enter),
        "space" => Physical::Code(Code::Space),
        "tab" => Physical::Code(Code::Tab),
        "delete" => Physical::Code(Code::Delete),
        "end" => Physical::Code(Code::End),
        "home" => Physical::Code(Code::Home),
        "insert" => Physical::Code(Code::Insert),
        "pagedown" => Physical::Code(Code::PageDown),
        "pageup" => Physical::Code(Code::PageUp),
        "numpadsubtract" => Physical::Code(Code::NumpadSubtract),
        "escape" => Physical::Code(Code::Escape),
        "printscreen" => Physical::Code(Code::PrintScreen),
        "pausebreak" => Physical::Code(Code::Pause),
        "numpad0" => Physical::Code(Code::Numpad0),
        "numpad1" => Physical::Code(Code::Numpad1),
        "numpad2" => Physical::Code(Code::Numpad2),
        "numpad3" => Physical::Code(Code::Numpad3),
        "numpad4" => Physical::Code(Code::Numpad4),
        "numpad5" => Physical::Code(Code::Numpad5),
        "numpad6" => Physical::Code(Code::Numpad6),
        "numpad7" => Physical::Code(Code::Numpad7),
        "numpad8" => Physical::Code(Code::Numpad8),
        "numpad9" => Physical::Code(Code::Numpad9),
        "0" => Physical::Code(Code::Digit0),
        "1" => Physical::Code(Code::Digit1),
        "2" => Physical::Code(Code::Digit2),
        "3" => Physical::Code(Code::Digit3),
        "4" => Physical::Code(Code::Digit4),
        "5" => Physical::Code(Code::Digit5),
        "6" => Physical::Code(Code::Digit6),
        "7" => Physical::Code(Code::Digit7),
        "8" => Physical::Code(Code::Digit8),
        "9" => Physical::Code(Code::Digit9),
        "f1" => Physical::Code(Code::F1),
        "f2" => Physical::Code(Code::F2),
        "f3" => Physical::Code(Code::F3),
        "f4" => Physical::Code(Code::F4),
        "f5" => Physical::Code(Code::F5),
        "f6" => Physical::Code(Code::F6),
        "f7" => Physical::Code(Code::F7),
        "f8" => Physical::Code(Code::F8),
        "f9" => Physical::Code(Code::F9),
        "f10" => Physical::Code(Code::F10),
        "f11" => Physical::Code(Code::F11),
        "f12" => Physical::Code(Code::F12),
        "f13" => Physical::Code(Code::F13),
        "f14" => Physical::Code(Code::F14),
        "f15" => Physical::Code(Code::F15),
        "f16" => Physical::Code(Code::F16),
        "f17" => Physical::Code(Code::F17),
        "f18" => Physical::Code(Code::F18),
        "f19" => Physical::Code(Code::F19),
        "f20" => Physical::Code(Code::F20),
        "f21" => Physical::Code(Code::F21),
        "f22" => Physical::Code(Code::F22),
        "f23" => Physical::Code(Code::F23),
        "f24" => Physical::Code(Code::F24),
        "f25" => Physical::Code(Code::F25),
        "f26" => Physical::Code(Code::F26),
        "f27" => Physical::Code(Code::F27),
        "f28" => Physical::Code(Code::F28),
        "f29" => Physical::Code(Code::F29),
        "f30" => Physical::Code(Code::F30),
        "f31" => Physical::Code(Code::F31),
        "f32" => Physical::Code(Code::F32),
        "f33" => Physical::Code(Code::F33),
        "f34" => Physical::Code(Code::F34),
        "f35" => Physical::Code(Code::F35),
        _ => Physical::Code(Code::F35),
    };

    let raw_modifiers = splitted;

    if raw_modifiers.is_empty() {
        //       println!("no modifier provided !");
        return Some(result);
    }

    let mut fresh_modifiers = Modifiers::empty();

    for raw_mod in raw_modifiers.iter() {
        fresh_modifiers.insert(match *raw_mod {
            "ctrl" => Modifiers::CTRL,
            "shift" => Modifiers::SHIFT,
            "alt" => Modifiers::ALT,
            _ => Modifiers::NONE,
        });
    }

    result.modifiers = fresh_modifiers;

    Some(result)
}

fn process_keybinds(raw_keybinds: RawKeybindsConfig, keybinds: &mut KeybindsConfig) {
    if let Some(key_str) = raw_keybinds.navigate_up
        && let Some(fresh_key) = match_key(key_str)
    {
        keybinds.navigate_up = fresh_key;
    }

    if let Some(key_str) = raw_keybinds.navigate_down
        && let Some(fresh_key) = match_key(key_str)
    {
        keybinds.navigate_down = fresh_key;
    }

    if let Some(key_str) = raw_keybinds.navigate_forward
        && let Some(fresh_key) = match_key(key_str)
    {
        keybinds.navigate_forward = fresh_key;
    }

    if let Some(key_str) = raw_keybinds.navigate_backward
        && let Some(fresh_key) = match_key(key_str)
    {
        keybinds.navigate_backward = fresh_key;
    }

    if let Some(key_str) = raw_keybinds.copy_to_clipboard
        && let Some(fresh_key) = match_key(key_str)
    {
        keybinds.copy_to_clipboard = fresh_key;
    }

    if let Some(key_str) = raw_keybinds.cut_to_clipboard
        && let Some(fresh_key) = match_key(key_str)
    {
        keybinds.cut_to_clipboard = fresh_key;
    }

    if let Some(key_str) = raw_keybinds.paste_from_clipboard
        && let Some(fresh_key) = match_key(key_str)
    {
        keybinds.paste_from_clipboard = fresh_key;
    }

    if let Some(key_str) = raw_keybinds.delete_selections
        && let Some(fresh_key) = match_key(key_str)
    {
        keybinds.delete_selections = fresh_key;
    }

    if let Some(key_str) = raw_keybinds.rename_file
        && let Some(fresh_key) = match_key(key_str)
    {
        keybinds.rename_file = fresh_key;
    }

    if let Some(key_str) = raw_keybinds.toggle_hidden_view
        && let Some(fresh_key) = match_key(key_str)
    {
        keybinds.toggle_hidden_view = fresh_key;
    }

    if let Some(key_str) = raw_keybinds.create_file_path
        && let Some(fresh_key) = match_key(key_str)
    {
        keybinds.create_file_path = fresh_key;
    }

    if let Some(key_str) = raw_keybinds.create_folder_path
        && let Some(fresh_key) = match_key(key_str)
    {
        keybinds.create_folder_path = fresh_key;
    }

    if let Some(key_str) = raw_keybinds.toggle_visual_mode
        && let Some(fresh_key) = match_key(key_str)
    {
        keybinds.toggle_visual_mode = fresh_key;
    }
}
