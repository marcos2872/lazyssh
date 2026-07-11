use crate::config::models::Server;
use crate::ssh::args::build_ssh_args;
use std::io::{Read, Write};

pub fn upload_via_ssh(server: &Server, local: &str, remote: &str) -> Result<(), String> {
    let (cmd, mut args) = build_ssh_args(server);
    args.push(format!("cat > {}", remote));

    let file = std::fs::File::open(local)
        .map_err(|e| format!("Erro ao ler {}: {}", local, e))?;

    let mut child = std::process::Command::new(&cmd)
        .args(&args)
        .stdin(file)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("ssh erro: {}", e))?;

    child.wait().map_err(|e| format!("ssh wait erro: {}", e))?;
    Ok(())
}

pub fn download_via_ssh(server: &Server, remote: &str, local: &str) -> Result<(), String> {
    let (cmd, mut args) = build_ssh_args(server);
    args.push(format!("cat {}", remote));

    let mut child = std::process::Command::new(&cmd)
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("ssh erro: {}", e))?;

    let mut stdout = child.stdout.take().ok_or("No stdout")?;
    let mut file = std::fs::File::create(local)
        .map_err(|e| format!("Erro ao criar {}: {}", local, e))?;

    let mut buf = [0u8; 65536];
    loop {
        match stdout.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let _ = file.write_all(&buf[..n]);
            }
            Err(_) => break,
        }
    }

    child.wait().map_err(|e| format!("ssh wait erro: {}", e))?;
    Ok(())
}
