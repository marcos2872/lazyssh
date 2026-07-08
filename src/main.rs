pub mod config;
pub mod sftp;
pub mod ssh;
pub mod vault;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("LazySSH v0.1.0");
    Ok(())
}
