/// manually matching every (common) file types because im fucking insane and unemployed

pub fn convert_to_filetype(extension: &str) -> Option<String> {
    let something: &str = match extension {
        // images
        "png" => "PNG Image",
        "jpg" => "JPEG Image",
        "jpeg" => "JPEG Image",
        "webp" => "WEBP Image",
        "avif" => "AVIF Image",
        "gif" => "GIF Image",
        // videos
        // text
        "rs" => "Rust Source File",
        "py" => "Python Source File",
        "txt" => "Generic Text File",
        "md" => "Markdown Document",
        "toml" => "TOML Document",
        // archive
        "7z" => "7Zip Archive",
        "zip" => "Zip Archive",
        "rar" => "Rar Archive",
        _ => "",
    };

    if something.is_empty() {
        None
    } else {
        Some(something.to_owned())
    }
}
