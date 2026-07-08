pub mod config;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("LazySSH v0.1.0");
    Ok(())
}
