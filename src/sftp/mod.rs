pub mod local;
pub mod remote;
pub mod service;

#[derive(Debug)]
pub struct FileInfo {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

pub use local::LocalFs;
pub use remote::RemoteFs;
pub use service::{SftpService, SftpServiceSession, SessionStatus};
