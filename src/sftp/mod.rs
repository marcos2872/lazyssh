pub mod local;
mod service;

#[derive(Debug)]
pub struct FileInfo {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub permissions: Option<u32>,
}

pub use local::LocalFs;
pub use service::{SftpService, SftpServiceSession, SessionStatus, upload_file};
