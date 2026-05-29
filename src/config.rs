// "prepare for matching hell" - tsuki May 29th 2026

use iced::keyboard::key::{Code, Physical};
use serde::Deserialize;
use std::{env::home_dir, fs};

pub struct Config {
    pub keybinds: KeybindsConfig,
}

pub struct KeybindsConfig {
    pub navigate_up: Physical,
    pub navigate_down: Physical,
    pub navigate_forward: Physical,
    pub navigate_backward: Physical,
}

pub fn get_keybinds() -> Config {
    // default config
    let mut config = Config {
        keybinds: KeybindsConfig {
            navigate_up: Physical::Code(Code::ArrowUp),
            navigate_down: Physical::Code(Code::ArrowDown),
            navigate_backward: Physical::Code(Code::ArrowLeft),
            navigate_forward: Physical::Code(Code::ArrowRight),
        },
    };

    let home = home_dir();

    if home.is_none() {
        println!("cannot get home directory!");
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
    keybinds: RawKeybindsConfig,
}

#[derive(Deserialize)]
struct RawKeybindsConfig {
    navigate_up: Option<String>,
    navigate_down: Option<String>,
    navigate_forward: Option<String>,
    navigate_backward: Option<String>,
}

fn process_rawconfig(raw_config: RawConfig, config: &mut Config) {
    process_keybinds(raw_config.keybinds, &mut config.keybinds);
}

fn match_key(raw_key: String) -> Physical {
    match raw_key.to_lowercase().as_str() {
        "up" => Physical::Code(Code::ArrowUp),
        "down" => Physical::Code(Code::ArrowDown),
        "right" => Physical::Code(Code::ArrowRight),
        "left" => Physical::Code(Code::ArrowLeft),
        "j" => Physical::Code(Code::KeyJ),
        "k" => Physical::Code(Code::KeyK),
        "h" => Physical::Code(Code::KeyH),
        "l" => Physical::Code(Code::KeyL),
        _ => Physical::Code(Code::End),
    }
}

fn process_keybinds(raw_keybinds: RawKeybindsConfig, keybinds: &mut KeybindsConfig) {
    if let Some(key_str) = raw_keybinds.navigate_up {
        keybinds.navigate_up = match_key(key_str);
    }

    if let Some(key_str) = raw_keybinds.navigate_down {
        keybinds.navigate_down = match_key(key_str);
    }

    if let Some(key_str) = raw_keybinds.navigate_forward {
        keybinds.navigate_forward = match_key(key_str);
    }

    if let Some(key_str) = raw_keybinds.navigate_backward {
        keybinds.navigate_backward = match_key(key_str);
    }
}
