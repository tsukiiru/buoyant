use crate::file::icons::IconKind;

pub fn extension_to_file_type(extension: &str) -> Option<(&'static str, IconKind)> {
  let something: (&'static str, IconKind) = match extension {
    // images
    "png" => ("PNG Image", IconKind::Image),
    "jpg" => ("JPEG Image", IconKind::Image),
    "jpeg" => ("JPEG Image", IconKind::Image),
    "webp" => ("WEBP Image", IconKind::Image),
    "avif" => ("AVIF Image", IconKind::Image),
    "gif" => ("GIF Animated Image", IconKind::Image),
    "svg" => ("SVG Image", IconKind::Image),
    "ase" => ("Aseprite Sprite", IconKind::Painting),
    "aseprite" => ("Aseprite Sprite", IconKind::Painting),
    "xcf" => ("GIMP Raw Image", IconKind::Image),
    // videos
    "mp4" => ("MP4 Video", IconKind::Video),
    "avi" => ("AVI Video", IconKind::Video),
    "mov" => ("MOV Video", IconKind::Video),
    "wmv" => ("WMV Video", IconKind::Video),
    "mkv" => ("MKV Video", IconKind::Video),
    "m4v" => ("M4V Video", IconKind::Video),
    // audio
    "mp3" => ("MP3 Audio", IconKind::Audio),
    "opus" => ("OPUS Audio", IconKind::Audio),
    "flac" => ("FLAC Audio", IconKind::Audio),
    "wav" => ("WAV Audio", IconKind::Audio),
    "aiff" => ("AIFF Audio", IconKind::Audio),
    "ogg" => ("OGG Audio", IconKind::Audio),
    // text
    "rs" => ("Rust Source File", IconKind::RustSrc),
    "py" => ("Python Source File", IconKind::PySrc),
    "c" => ("C Source File", IconKind::CSrc),
    "cpp" => ("C++ Source File", IconKind::CppSrc),
    "jar" => ("Java Source File", IconKind::JavaSrc),
    "java" => ("Java Source File", IconKind::JavaSrc),
    "js" => ("JavaScript Source File", IconKind::JsSrc),
    "ts" => ("TypeScript Source File", IconKind::TsSrc),
    "go" => ("Golang Source File", IconKind::Src),
    "tsx" => ("React Source File", IconKind::TsxSrc),
    "jsx" => ("React Source File", IconKind::JsxSrc),
    "txt" => ("Text File", IconKind::Src),
    "cs" => ("C# Source File", IconKind::CsSrc),
    "csx" => ("C# Source File", IconKind::CsSrc),
    "asm" => ("Assembly Source File", IconKind::Src),
    "md" => ("Markdown Document", IconKind::MdSrc),
    "css" => ("Cascading Stylesheets", IconKind::CssSrc),
    "toml" => ("TOML Document", IconKind::Src),
    "vue" => ("VUE Source File", IconKind::VueSrc),
    "sh" => ("Shell Script", IconKind::Script),
    "bat" => ("Batch Script", IconKind::Script),
    "json" => ("JavaScript Object Notation", IconKind::Src),
    "html" => ("HTML", IconKind::HtmlSrc),
    // archive
    "7z" => ("7-Zip Archive", IconKind::Archive),
    "zip" => ("ZIP Archive", IconKind::Archive),
    "rar" => ("RAR Archive", IconKind::Archive),
    "dmg" => ("Apple Disk Image", IconKind::Archive),
    "apk" => ("Android Application Package", IconKind::Archive),
    // something
    "db" => ("Database", IconKind::Database),
    "sqlite" => ("SQL Database", IconKind::Sqlite),
    "sqlite3" => ("SQL Database", IconKind::Sqlite),
    "sqlite-wal" => ("SQL Database", IconKind::Sqlite),
    "blend" => ("Blender Model", IconKind::Cube),
    "bin" => ("Binary File", IconKind::File),
    "blend1" => ("Blender Backup Model", IconKind::Cube),
    "dll" => ("Dynamic-Link Library", IconKind::Link),
    _ => ("", IconKind::File),
  };

  if something.0.is_empty() {
    None
  } else {
    Some(something)
  }
}
