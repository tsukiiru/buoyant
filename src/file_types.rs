use iced::advanced::svg::Handle;
use std::sync::LazyLock;

pub static LEFT_ARROW: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/arrow-left.svg")));
pub static FOLDER: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/folder.svg")));
pub static FILE: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file.svg")));
pub static IMAGE: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/image.svg")));
pub static VIDEO: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-video.svg")));
pub static AUDIO: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-audio.svg")));
pub static SCRIPT: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/terminal.svg")));
pub static DATABASE: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/database.svg")));
pub static SQLITE: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-sql.svg")));
pub static ARCHIVE: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/archive.svg")));
pub static SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-code.svg")));
pub static RUST_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-rs.svg")));
pub static PYTHON_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-py.svg")));
pub static C_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-c.svg")));
pub static CS_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-c-sharp.svg")));
pub static CPP_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-cpp.svg")));
pub static CSS_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-css.svg")));
pub static JAVA_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/coffee.svg")));
pub static HTML_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-html.svg")));
pub static JS_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-js.svg")));
pub static TS_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-ts.svg")));
pub static JSX_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-jsx.svg")));
pub static TSX_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-tsx.svg")));
pub static MD_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/markdown-logo.svg")));
pub static VUE_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-vue.svg")));
pub static BROKEN_LINK: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/link-break.svg")));
pub static LINK: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/link.svg")));
pub static QUESTION_MARK: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/question-mark.svg")));

pub fn extension_to_filetype(extension: &str) -> Option<(String, &'static Handle)> {
    let something: (&str, &Handle) = match extension {
        // images
        "png" => ("PNG Image", &IMAGE),
        "jpg" => ("JPEG Image", &IMAGE),
        "jpeg" => ("JPEG Image", &IMAGE),
        "webp" => ("WEBP Image", &IMAGE),
        "avif" => ("AVIF Image", &IMAGE),
        "gif" => ("GIF Animated Image", &IMAGE),
        "svg" => ("SVG Image", &IMAGE),
        // videos
        "mp4" => ("MP4 Video", &VIDEO),
        "avi" => ("AVI Video", &VIDEO),
        "mov" => ("MOV Video", &VIDEO),
        "wmv" => ("WMV Video", &VIDEO),
        "mkv" => ("MKV Video", &VIDEO),
        "m4v" => ("M4V Video", &VIDEO),
        // audio
        "mp3" => ("MP3 Audio", &AUDIO),
        "opus" => ("OPUS Audio", &AUDIO),
        "flac" => ("FLAC Audio", &AUDIO),
        "wav" => ("WAV Audio", &AUDIO),
        "aiff" => ("AIFF Audio", &AUDIO),
        // text
        "rs" => ("Rust Source File", &RUST_SRC),
        "py" => ("Python Source File", &PYTHON_SRC),
        "c" => ("C Source File", &C_SRC),
        "cpp" => ("C++ Source File", &CPP_SRC),
        "jar" => ("Java Source File", &JAVA_SRC),
        "java" => ("Java Source File", &JAVA_SRC),
        "go" => ("Go Source File", &SRC),
        "js" => ("Javascript Source File", &JS_SRC),
        "ts" => ("Typescript Source File", &TS_SRC),
        "tsx" => ("React Source File", &TSX_SRC),
        "jsx" => ("React Source File", &JSX_SRC),
        "txt" => ("Text File", &FILE),
        "cs" => ("C# Source File", &CS_SRC),
        "csx" => ("C# Source File", &CS_SRC),
        "asm" => ("Assembly Source File", &SRC),
        "md" => ("Markdown Document", &MD_SRC),
        "css" => ("Cascading Stylesheets", &CSS_SRC),
        "toml" => ("TOML Document", &SRC),
        "html" => ("Hypertext Markup", &HTML_SRC),
        "vue" => ("VUE Source File", &VUE_SRC),
        "sh" => ("Shell Script", &SCRIPT),
        // archive
        "7z" => ("7-Zip Archive", &ARCHIVE),
        "zip" => ("ZIP Archive", &ARCHIVE),
        "rar" => ("RAR Archive", &ARCHIVE),
        "dmg" => ("Apple Disk Image", &ARCHIVE),
        "apk" => ("Android Application Package", &ARCHIVE),
        // something
        "db" => ("Database", &DATABASE),
        "sqlite" => ("SQL Database", &SQLITE),
        "sqlite3" => ("SQL Database", &SQLITE),
        "sqlite-wal" => ("SQL Database", &SQLITE),
        _ => ("", &FILE),
    };

    if something.0.is_empty() {
        None
    } else {
        Some((something.0.to_owned(), something.1))
    }
}
