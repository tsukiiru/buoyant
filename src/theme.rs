// prepare for another matching hell - tsuki 22nd June 2026
use iced::Color;
use serde::Deserialize;
use std::{env::home_dir, error::Error, fs};
use toml;

pub struct Theme {
    pub palette: Palette,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            palette: Palette::default(),
        }
    }
}

pub struct Palette {
    pub text: Color,
    pub text_muted: Color,

    pub background: Color,
    pub overlay: Color,
    pub scrim: Color,

    pub accent: Color,
    pub accent_dark: Color,

    pub red: Color,
    pub yellow: Color,
    pub green: Color,
    pub blue: Color,
}

impl Default for Palette {
    fn default() -> Self {
        Palette {
            text: Color::from_rgb8(84, 84, 100),
            text_muted: Color::from_rgb8(67, 67, 108),
            background: Color::from_rgb8(242, 236, 188),
            overlay: Color::from_rgb8(220, 213, 172),
            scrim: Color::from_rgba8(231, 219, 160, 0.8),
            accent: Color::from_rgb8(199, 215, 224),
            accent_dark: Color::from_rgb8(159, 181, 201),
            red: Color::from_rgb8(232, 36, 36),
            yellow: Color::from_rgb8(233, 138, 0),
            green: Color::from_rgb8(111, 137, 78),
            blue: Color::from_rgb8(90, 119, 133),
        }
        // default palette is kanagawa lotus bc its so awesome
    }
}

#[derive(Deserialize)]
struct RawTheme {
    palette: Option<RawPalette>,
}

// not following any naming conventions here because they are horrible!!
#[derive(Deserialize)]
struct RawPalette {
    text: Option<String>,
    text_muted: Option<String>,

    background: Option<String>,
    overlay: Option<String>,
    scrim: Option<String>,

    accent: Option<String>,
    accent_dark: Option<String>,

    red: Option<String>,
    yellow: Option<String>,
    green: Option<String>,
    blue: Option<String>,
}

pub fn fetch(theme_name: Option<&str>) -> Theme {
    let mut theme = Theme::default();

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
            process_rawtheme(raw_theme, &mut theme);
        }
    }

    theme
}

fn process_rawtheme(raw_theme: RawTheme, theme: &mut Theme) {
    if let Some(raw_palette) = raw_theme.palette {
        process_palette(raw_palette, &mut theme.palette);
    }
}

fn process_palette(raw_palette: RawPalette, palette: &mut Palette) {
    if let Some(raw_color) = raw_palette.text {
        palette.text = match_color(&raw_color);
    }
    if let Some(raw_color) = raw_palette.text_muted {
        palette.text_muted = match_color(&raw_color);
    }
    if let Some(raw_color) = raw_palette.background {
        palette.background = match_color(&raw_color);
    }
    if let Some(raw_color) = raw_palette.overlay {
        palette.overlay = match_color(&raw_color);
    }
    if let Some(raw_color) = raw_palette.scrim {
        palette.scrim = match_color(&raw_color);
    }
    if let Some(raw_color) = raw_palette.accent {
        palette.accent = match_color(&raw_color);
    }
    if let Some(raw_color) = raw_palette.accent_dark {
        palette.accent_dark = match_color(&raw_color);
    }
    if let Some(raw_color) = raw_palette.red {
        palette.red = match_color(&raw_color);
    }
    if let Some(raw_color) = raw_palette.yellow {
        palette.yellow = match_color(&raw_color);
    }
    if let Some(raw_color) = raw_palette.green {
        palette.green = match_color(&raw_color);
    }
    if let Some(raw_color) = raw_palette.blue {
        palette.blue = match_color(&raw_color);
    }
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
