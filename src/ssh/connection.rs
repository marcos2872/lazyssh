use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use russh::client::Handler;
use russh::keys::*;
use russh::*;

use crate::config::models::{Auth, Server};

struct SshClient;

impl Handler for SshClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // TODO: Implement known_hosts verification before production use
        // For now, warn and accept (MITM vulnerable)
        eprintln!("WARNING: Host key verification not implemented - vulnerable to MITM attacks");
        Ok(true)
    }
}

pub struct SshSession {
    session: client::Handle<SshClient>,
}

impl SshSession {
    pub async fn connect(server: &Server, auth: &Auth) -> Result<Self> {
        let config = client::Config {
            inactivity_timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        };

        let config = Arc::new(config);
        let sh = SshClient;

        let mut session =
            client::connect(config, (server.host.as_str(), server.port), sh).await?;

        match auth {
            Auth::Key { .. } => {
                let key_with_hash = super::auth::load_key(auth)?;
                let auth_res = session
                    .authenticate_publickey(&server.user, key_with_hash)
                    .await?;

                if !auth_res.success() {
                    anyhow::bail!("Public key authentication failed");
                }
            }
            Auth::Password { vault_key: _ } => {
                return Err(anyhow::anyhow!("Password authentication not yet implemented - use key auth"));
            }
        }

        Ok(Self { session })
    }

    pub async fn execute(&self, command: &str) -> Result<String> {
        let mut channel = self.session.channel_open_session().await?;
        channel.exec(true, command).await?;

        let mut output = String::new();
        while let Some(msg) = channel.wait().await {
            match msg {
                ChannelMsg::Data { data } => {
                    output.push_str(&String::from_utf8_lossy(&data));
                }
                ChannelMsg::ExitStatus { .. } => break,
                _ => {}
            }
        }

        Ok(output)
    }

    pub async fn shell(&self) -> Result<()> {
        todo!("Interactive shell not yet implemented - will be added in Task 9")
    }

    pub async fn close(&mut self) -> Result<()> {
        self.session
            .disconnect(Disconnect::ByApplication, "", "English")
            .await?;
        Ok(())
    }
}
