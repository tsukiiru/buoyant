pub fn extension_to_file_type(extension: &str) -> Option<&'static str> {
    let something: &'static str = match extension {
        // images
        "png" => "PNG Image",
        "jpg" => "JPEG Image",
        "jpeg" => "JPEG Image",
        "webp" => "WEBP Image",
        "avif" => "AVIF Image",
        "gif" => "GIF Animated Image",
        "svg" => "SVG Image",
        "ase" => "Aseprite Sprite",
        "aseprite" => "Aseprite Sprite",
        "xcf" => "GIMP Raw Image",
        // videos
        "mp4" => "MP4 Video",
        "avi" => "AVI Video",
        "mov" => "MOV Video",
        "wmv" => "WMV Video",
        "mkv" => "MKV Video",
        "m4v" => "M4V Video",
        // audio
        "mp3" => "MP3 Audio",
        "opus" => "OPUS Audio",
        "flac" => "FLAC Audio",
        "wav" => "WAV Audio",
        "aiff" => "AIFF Audio",
        "ogg" => "OGG Audio",
        // text
        "rs" => "Rust Source File",
        "py" => "Python Source File",
        "c" => "C Source File",
        "cpp" => "C++ Source File",
        "jar" => "Java Source File",
        "java" => "Java Source File",
        "js" => "JavaScript Source File",
        "ts" => "TypeScript Source File",
        "go" => "Golang Source File",
        "tsx" => "React Source File",
        "jsx" => "React Source File",
        "txt" => "Text File",
        "cs" => "C# Source File",
        "csx" => "C# Source File",
        "asm" => "Assembly Source File",
        "md" => "Markdown Document",
        "css" => "Cascading Stylesheets",
        "toml" => "TOML Document",
        "vue" => "VUE Source File",
        "sh" => "Shell Script",
        "bat" => "Batch Script",
        "json" => "JavaScript Object Notation",
        // archive
        "7z" => "7-Zip Archive",
        "zip" => "ZIP Archive",
        "rar" => "RAR Archive",
        "dmg" => "Apple Disk Image",
        "apk" => "Android Application Package",
        // something
        "db" => "Database",
        "sqlite" => "SQL Database",
        "sqlite3" => "SQL Database",
        "sqlite-wal" => "SQL Database",
        "blend" => "Blender Model",
        "bin" => "Binary File",
        "blend1" => "Blender Backup Model",
        "dll" => "Dynamic-Link Library",
        _ => "",
    };

    if something.is_empty() {
        None
    } else {
        Some(something)
    }
}
