use eframe::egui::{ImageSource, include_image};

use crate::file_types::IconKind;

pub fn match_icon(kind: &IconKind) -> ImageSource<'_> {
    match kind {
        IconKind::Image => include_image!("../assets/icons/image.svg"),
        IconKind::Painting => include_image!("../assets/icons/paint-brush.svg"),
        IconKind::Video => include_image!("../assets/icons/video.svg"),
        IconKind::Audio => include_image!("../assets/icons/file-audio.svg"),
        IconKind::RustSrc => include_image!("../assets/icons/file-rs.svg"),
        IconKind::PySrc => include_image!("../assets/icons/file-py.svg"),
        IconKind::CSrc => include_image!("../assets/icons/file-c.svg"),
        IconKind::CsSrc => include_image!("../assets/icons/file-c-sharp.svg"),
        IconKind::CppSrc => include_image!("../assets/icons/file-cpp.svg"),
        IconKind::JavaSrc => include_image!("../assets/icons/coffee.svg"),
        IconKind::JsSrc => include_image!("../assets/icons/file-js.svg"),
        IconKind::TsSrc => include_image!("../assets/icons/file-ts.svg"),
        IconKind::Src => include_image!("../assets/icons/file-code.svg"),
        IconKind::TsxSrc => include_image!("../assets/icons/file-tsx.svg"),
        IconKind::JsxSrc => include_image!("../assets/icons/file-jsx.svg"),
        IconKind::MdSrc => include_image!("../assets/icons/file-md.svg"),
        IconKind::CssSrc => include_image!("../assets/icons/file-css.svg"),
        IconKind::HtmlSrc => include_image!("../assets/icons/file-html.svg"),
        IconKind::Script => include_image!("../assets/icons/terminal.svg"),
        IconKind::Archive => include_image!("../assets/icons/archive.svg"),
        IconKind::Database => include_image!("../assets/icons/database.svg"),
        IconKind::Sqlite => include_image!("../assets/icons/file-sql.svg"),
        IconKind::Cube => include_image!("../assets/icons/cube.svg"),
        IconKind::File => include_image!("../assets/icons/file.svg"),
        IconKind::QuestionMark => include_image!("../assets/icons/question-mark.svg"),
        IconKind::BrokenLink => include_image!("../assets/icons/link-break.svg"),
        IconKind::Folder => include_image!("../assets/icons/folder.svg"),
        IconKind::VueSrc => include_image!("../assets/icons/file-vue.svg"),
        IconKind::Link => include_image!("../assets/icons/link.svg"),
    }
}
