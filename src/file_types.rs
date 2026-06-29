use iced::advanced::svg::Handle;
use std::sync::LazyLock;

pub static FOLDER: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/folder.svg").as_ref()));
pub static FILE: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file.svg").as_ref()));
pub static IMAGE: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/image.svg").as_ref()));
pub static VIDEO: LazyLock<Handle> = LazyLock::new(|| {
    Handle::from_memory(include_bytes!("../assets/icons/file-video.svg").as_ref())
});
pub static AUDIO: LazyLock<Handle> = LazyLock::new(|| {
    Handle::from_memory(include_bytes!("../assets/icons/file-audio.svg").as_ref())
});

pub static DATABASE: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/database.svg").as_ref()));
pub static ARCHIVE: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/archive.svg").as_ref()));
pub static SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-code.svg").as_ref()));
pub static RUST_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-rs.svg").as_ref()));
pub static PYTHON_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-py.svg").as_ref()));
pub static C_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-c.svg").as_ref()));
pub static CS_SRC: LazyLock<Handle> = LazyLock::new(|| {
    Handle::from_memory(include_bytes!("../assets/icons/file-c-sharp.svg").as_ref())
});
pub static CPP_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-cpp.svg").as_ref()));
pub static CSS_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-css.svg").as_ref()));
pub static JAVA_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/coffee.svg").as_ref()));
pub static HTML_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-html.svg").as_ref()));
pub static JS_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-js.svg").as_ref()));
pub static TS_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-ts.svg").as_ref()));
pub static JSX_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-jsx.svg").as_ref()));
pub static TSX_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-tsx.svg").as_ref()));
pub static MD_SRC: LazyLock<Handle> = LazyLock::new(|| {
    Handle::from_memory(include_bytes!("../assets/icons/markdown-logo.svg").as_ref())
});
pub static VUE_SRC: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/file-vue.svg").as_ref()));
pub static BROKEN_LINK: LazyLock<Handle> = LazyLock::new(|| {
    Handle::from_memory(include_bytes!("../assets/icons/link-break.svg").as_ref())
});
pub static LINK: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(include_bytes!("../assets/icons/link.svg").as_ref()));

pub static QUESTION_MARK: LazyLock<Handle> = LazyLock::new(|| {
    Handle::from_memory(include_bytes!("../assets/icons/question-mark.svg").as_ref())
});

// manually matching every (common) file types (i know) because im fucking insane and unemployed
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
        // archive
        "7z" => ("7-Zip Archive", &ARCHIVE),
        "zip" => ("ZIP Archive", &ARCHIVE),
        "rar" => ("RAR Archive", &ARCHIVE),
        "dmg" => ("Apple Disk Image", &ARCHIVE),
        "apk" => ("Android Application Package", &ARCHIVE),
        // something
        "db" => ("Database", &DATABASE),
        _ => ("", &FILE),
    };

    if something.0.is_empty() {
        None
    } else {
        Some((something.0.to_owned(), something.1))
    }
}
