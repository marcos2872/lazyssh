use crate::config::models::Server;
use std::io::{Read, Write};

/// Envia um arquivo local para um servidor remoto via pipe SSH (`cat local | ssh 'cat > remote'`).
///
/// Contorna o limite de buffer de ~1GB do SFTP transmitindo diretamente via SSH.
/// Retorna `Ok(())` em caso de sucesso, ou mensagem de erro em caso de falha.
pub fn upload_via_ssh(server: &Server, local: &str, remote: &str) -> Result<(), String> {
    let file = std::fs::File::open(local)
        .map_err(|e| format!("Erro ao ler {}: {}", local, e))?;

    let extra = vec![format!("cat > {}", remote)];
    let mut child = crate::ssh::args::spawn_ssh_process(
        server,
        extra,
        Some(file.into()),
        Some(std::process::Stdio::piped()),
        Some(std::process::Stdio::piped()),
    )?;

    child.wait().map_err(|e| format!("ssh wait erro: {}", e))?;
    Ok(())
}

/// Baixa um arquivo remoto para um caminho local via pipe SSH (`ssh 'cat remote' > local`).
///
/// Contorna o limite de buffer de ~1GB do SFTP transmitindo diretamente via SSH.
/// Retorna `Ok(())` em caso de sucesso, ou mensagem de erro em caso de falha.
pub fn download_via_ssh(server: &Server, remote: &str, local: &str) -> Result<(), String> {
    let extra = vec![format!("cat {}", remote)];
    let mut child = crate::ssh::args::spawn_ssh_process(
        server,
        extra,
        None,
        Some(std::process::Stdio::piped()),
        Some(std::process::Stdio::piped()),
    )?;

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
