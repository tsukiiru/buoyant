use iced::advanced::svg::Handle;
use std::sync::LazyLock;

pub static FOLDER: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/folder.svg").as_ref()));
pub static FILE: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file.svg").as_ref()));
pub static IMAGE: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/image.svg").as_ref()));
pub static RUST_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-rs.svg")));

// manually matching every (common) file types (i know) because im fucking insane and unemployed
pub fn extension_to_filetype(extension: &str) -> Option<(String, &'static LazyLock<Handle>)> {
    let something: (&str, &LazyLock<Handle>) = match extension {
        // images
        "png" => ("PNG Image", &IMAGE),
        "jpg" => ("JPEG Image", &IMAGE),
        "jpeg" => ("JPEG Image", &IMAGE),
        "webp" => ("WEBP Image", &IMAGE),
        "avif" => ("AVIF Image", &IMAGE),
        "gif" => ("GIF Animated Image", &IMAGE),
        "svg" => ("SVG Image", &IMAGE),
        // videos
        "mp4" => ("MP4 Video", &FILE),
        "avi" => ("AVI Video", &FILE),
        "mov" => ("MOV Video", &FILE),
        "wmv" => ("WMV Video", &FILE),
        "mkv" => ("MKV Video", &FILE),
        "m4v" => ("M4V Video", &FILE),
        // audio
        "mp3" => ("MP3 Audio", &FILE),
        "opus" => ("OPUS Audio", &FILE),
        "flac" => ("FLAC Audio", &FILE),
        "wav" => ("WAV Audio", &FILE),
        "aiff" => ("AIFF Audio", &FILE),
        // text
        "rs" => ("Rust Source File", &RUST_SRC),
        "py" => ("Python Source File", &FILE),
        "c" => ("C Source File", &FILE),
        "cpp" => ("C++ Source File", &FILE),
        "jar" => ("Java Source File", &FILE),
        "java" => ("Java Source File", &FILE),
        "go" => ("Go Source File", &FILE),
        "js" => ("Javascript Source File", &FILE),
        "ts" => ("Typescript Source File", &FILE),
        "tsx" => ("React Source File", &FILE),
        "jsx" => ("React Source File", &FILE),
        "txt" => ("Text File", &FILE),
        "cs" => ("C# Source File", &FILE),
        "csx" => ("C# Source File", &FILE),
        "asm" => ("Assembly Source File", &FILE),
        "md" => ("Markdown Document", &FILE),
        "toml" => ("TOML Document", &FILE),
        // archive
        "7z" => ("7-Zip Archive", &FILE),
        "zip" => ("ZIP Archive", &FILE),
        "rar" => ("RAR Archive", &FILE),
        "dmg" => ("Apple Disk Image", &FILE),
        "apk" => ("Android Application Package", &FILE),
        // something
        "db" => ("Database", &FILE),
        _ => ("", &FILE),
    };

    if something.0.is_empty() {
        None
    } else {
        Some((something.0.to_owned(), something.1))
    }
}
