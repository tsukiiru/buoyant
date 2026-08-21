use eframe::egui::{ImageSource, include_image};

#[derive(Clone, Debug)]
pub enum IconKind {
  Image,
  Painting,
  Video,
  Audio,
  RustSrc,
  PySrc,
  CSrc,
  CppSrc,
  JavaSrc,
  JsSrc,
  TsSrc,
  Src,
  TsxSrc,
  JsxSrc,
  CsSrc,
  MdSrc,
  CssSrc,
  VueSrc,
  HtmlSrc,
  Script,
  Archive,
  Database,
  Sqlite,
  Cube,
  File,
  Files,
  Link,
  QuestionMark,
  BrokenLink,
  Folder,
  Scissors,
  Copy,
}

macro_rules! root_include_image {
  ($path:expr) => {
    include_image!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path))
  };
}

pub fn match_icon(kind: &IconKind) -> ImageSource<'_> {
  match kind {
    IconKind::Image => root_include_image!("assets/icons/image.svg"),
    IconKind::Painting => root_include_image!("assets/icons/paint-brush.svg"),
    IconKind::Video => root_include_image!("assets/icons/video.svg"),
    IconKind::Audio => root_include_image!("assets/icons/file-audio.svg"),
    IconKind::RustSrc => root_include_image!("assets/icons/file-rs.svg"),
    IconKind::PySrc => root_include_image!("assets/icons/file-py.svg"),
    IconKind::CSrc => root_include_image!("assets/icons/file-c.svg"),
    IconKind::CsSrc => root_include_image!("assets/icons/file-c-sharp.svg"),
    IconKind::CppSrc => root_include_image!("assets/icons/file-cpp.svg"),
    IconKind::JavaSrc => root_include_image!("assets/icons/coffee.svg"),
    IconKind::JsSrc => root_include_image!("assets/icons/file-js.svg"),
    IconKind::TsSrc => root_include_image!("assets/icons/file-ts.svg"),
    IconKind::Src => root_include_image!("assets/icons/file-code.svg"),
    IconKind::TsxSrc => root_include_image!("assets/icons/file-tsx.svg"),
    IconKind::JsxSrc => root_include_image!("assets/icons/file-jsx.svg"),
    IconKind::MdSrc => root_include_image!("assets/icons/file-md.svg"),
    IconKind::CssSrc => root_include_image!("assets/icons/file-css.svg"),
    IconKind::HtmlSrc => root_include_image!("assets/icons/file-html.svg"),
    IconKind::Script => root_include_image!("assets/icons/terminal.svg"),
    IconKind::Archive => root_include_image!("assets/icons/archive.svg"),
    IconKind::Database => root_include_image!("assets/icons/database.svg"),
    IconKind::Sqlite => root_include_image!("assets/icons/file-sql.svg"),
    IconKind::Cube => root_include_image!("assets/icons/cube.svg"),
    IconKind::File => root_include_image!("assets/icons/file.svg"),
    IconKind::QuestionMark => root_include_image!("assets/icons/question-mark.svg"),
    IconKind::BrokenLink => root_include_image!("assets/icons/link-break.svg"),
    IconKind::Folder => root_include_image!("assets/icons/folder.svg"),
    IconKind::VueSrc => root_include_image!("assets/icons/file-vue.svg"),
    IconKind::Link => root_include_image!("assets/icons/link.svg"),
    IconKind::Scissors => root_include_image!("assets/icons/scissors.svg"),
    IconKind::Copy => root_include_image!("assets/icons/copy-simple.svg"),
    IconKind::Files => root_include_image!("assets/icons/files.svg"),
  }
}
