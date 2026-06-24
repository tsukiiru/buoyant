/// manually matching every (common) file types (i know) because im fucking insane and unemployed

pub fn extension_to_filetype(extension: &str) -> Option<String> {
    let something: &str = match extension {
        // images
        "png" => "PNG Image",
        "jpg" => "JPEG Image",
        "jpeg" => "JPEG Image",
        "webp" => "WEBP Image",
        "avif" => "AVIF Image",
        "gif" => "GIF Image",
        "svg" => "SVG Graphics",
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
        // text
        "rs" => "Rust Source File",
        "py" => "Python Source File",
        "c" => "C Source File",
        "cpp" => "C++ Source File",
        "jar" => "Java Source File",
        "java" => "Java Source File",
        "go" => "Go Source File",
        "js" => "JavaScript Source File",
        "ts" => "TypeScript Source File",
        "tsx" => "React Source File",
        "jsx" => "React Source File",
        "txt" => "Generic Text File",
        "cs" => "C# Source File",
        "csx" => "C# Source File",
        "asm" => "Assembly Source File",
        "md" => "Markdown Document",
        "toml" => "TOML Document",
        // archive
        "7z" => "7Zip Archive",
        "zip" => "Zip Archive",
        "rar" => "Rar Archive",
        "dmg" => "Apple Disk Image",
        "apk" => "Android Application Package",
        // something
        "db" => "Database",
        _ => "",
    };

    if something.is_empty() {
        None
    } else {
        Some(something.to_owned())
    }
}
