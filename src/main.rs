pub mod config;
pub mod sftp;
pub mod ssh;
pub mod tui;
pub mod vault;

use anyhow::Result;
use tui::App;

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::load_or_default();
    let app = App::new(config.servers);
    
    println!("LazySSH v0.1.0 - {} servers loaded", app.servers.len());
    Ok(())
}
