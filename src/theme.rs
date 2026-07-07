// prepare for another matching hell - tsuki 22nd June 2026
use iced::{Color, theme::Theme};
use serde::Deserialize;
use std::{env::home_dir, error::Error, fs};
use toml;

use crate::fonts::search_fonts;

#[derive(Deserialize)]
struct RawTheme {
    palette: Option<RawPalette>,
    font: Option<String>,
}

// not following any naming conventions here because they are horrible!!
#[derive(Deserialize)]
struct RawPalette {
    text: Option<String>,
    background: Option<String>,
    primary: Option<String>,
    success: Option<String>,
    warning: Option<String>,
    danger: Option<String>,
}

pub fn fetch(theme_name: Option<&str>) -> (Option<Vec<String>>, Theme) {
    let mut theme = Theme::Light;
    let mut fonts = None;

    if let Some(name) = theme_name {
        let home = home_dir();

        if home.is_none() {
            println!("cannot get home directory!");
            println!("please check HOME environment variable and set it properly");
        }

        let config_dir = home
            .unwrap()
            .join(".config/buoyant")
            .join(format!("{}.toml", name));
        let read_content = fs::read_to_string(&config_dir);

        if let Ok(content) = read_content {
            let raw_theme: RawTheme = toml::from_str(&content).unwrap();
            (fonts, theme) = process_rawtheme(raw_theme, theme);
        }
    }

    (fonts, theme)
}

fn process_rawtheme(raw_theme: RawTheme, theme: Theme) -> (Option<Vec<String>>, Theme) {
    let f;
    let mut t = Theme::Light;

    if let Some(raw_palette) = raw_theme.palette {
        t = process_palette(raw_palette, theme);
    }

    if let Some(raw_font) = raw_theme.font {
        f = process_font(raw_font);
    } else {
        f = None;
    }

    (f, t)
}

fn process_font(raw_font: String) -> Option<Vec<String>> {
    let font_path = search_fonts(&raw_font);

    if let Ok(paths) = font_path {
        Some(paths)
    } else {
        None
    }
}

fn process_palette(raw_palette: RawPalette, theme: Theme) -> Theme {
    let mut palette = theme.palette();
    if let Some(raw_color) = raw_palette.text {
        palette.text = match_color(&raw_color);
    }
    if let Some(raw_color) = raw_palette.background {
        palette.background = match_color(&raw_color);
    }
    if let Some(raw_color) = raw_palette.primary {
        palette.primary = match_color(&raw_color);
    }
    if let Some(raw_color) = raw_palette.success {
        palette.success = match_color(&raw_color);
    }
    if let Some(raw_color) = raw_palette.warning {
        palette.warning = match_color(&raw_color);
    }
    if let Some(raw_color) = raw_palette.danger {
        palette.danger = match_color(&raw_color);
    }

    Theme::custom("custom", palette)
}

fn match_color(raw_color: &str) -> Color {
    let (r, g, b, a) = hex_to_rgba(raw_color).unwrap_or((255, 255, 255, 1.0));
    // dont blame me for flashbanging you

    Color::from_rgba8(r as u8, g as u8, b as u8, a)
}

// https://github.com/0Itsuki0/rust_color_conversion/blob/main/color_conversion.rs
fn hex_to_rgba(hex_str: &str) -> Result<(u32, u32, u32, f32), Box<dyn Error>> {
    let mut s = hex_str;
    if s.starts_with("#") {
        s = s.trim_start_matches("#");
    }

    // hex without alpha
    if s.len() == 6 {
        let num = u32::from_str_radix(s, 16)?;
        let r = (num & 0xFF0000) >> 16;
        let g = (num & 0x00FF00) >> 8;
        let b = (num & 0x0000FF) >> 0;
        return Ok((r, g, b, 1.0));
    }

    let num = u32::from_str_radix(s, 16)?;
    let r = (num & 0xFF000000) >> 24;
    let g = (num & 0x00FF0000) >> 16;
    let b = (num & 0x0000FF00) >> 8;
    let a = (num & 0x000000FF) >> 0;
    return Ok((r, g, b, (a as f32) / 255.0));
}
