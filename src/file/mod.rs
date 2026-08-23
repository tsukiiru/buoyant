use std::path::PathBuf;

pub mod icons;
pub mod info;
pub mod ops;
pub mod types;

#[derive(Clone, Copy)]
pub enum PasteKind {
    Replace,
    Duplicate,
}

#[derive(PartialEq)]
pub enum CreateKind {
    File,
    Folder,
}

#[derive(Debug)]
pub enum WorkerRequest {
    OperationKind { path: PathBuf },
    Update { percent: f32 },
    Done { paths: Vec<PathBuf> },
}
