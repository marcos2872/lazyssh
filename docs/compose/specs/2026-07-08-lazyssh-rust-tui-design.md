# LazySSH - Rust TUI SSH Manager

## [S1] Problem

Gerenciar múltiplas conexões SSH/SFTP via terminal é trabalhoso: lembrar IPs, digitar comandos longos, alternar entre terminais. Não existe uma solução Rust que una TUI interativo com SFTP dual-pane e vault de senhas.

## [S2] Solution Overview

TUI interativo em Rust para gerenciamento de conexões SSH/SFTP. Inspirado no lazyssh (Go) mas com funcionalidades adicionais do SSH_Orchestrator (SFTP dual-pane, vault de senhas).

## [S3] Stack

| Camada | Tecnologia |
|--------|------------|
| TUI | Ratatui + crossterm |
| SSH/SFTP | russh |
| Config | TOML (~/.config/lazyssh/servers.toml) |
| Async | tokio |
| Crypto | ring (AES-256-GCM + PBKDF2) |

## [S4] Módulos

### 1. config
- Parse/escrita de TOML
- Modela `Server` com: host, port, user, auth (key path ou password), tags, pinned
- Backup automático antes de escrita

### 2. ssh
- Conexão SSH via russh
- Autenticação por chave (Ed25519, RSA, ECDSA) com passphrase opcional
- Autenticação por senha com vault criptografado

### 3. sftp
- Dual-pane: listing local ↔ remoto
- Upload/download recursivo
- Navegação por diretórios com Enter

### 4. tui
- Ratatui com views: Server List | SSH Terminal | SFTP Browser
- Busca fuzzy por nome, IP ou tag
- Pin/despin servidores

### 5. vault
- Senhas protegidas com AES-256-GCM + PBKDF2 (100k iterações)
- Master password nunca armazenada

## [S5] Fluxo Principal

1. Usuário abre `lazyssh` → TUI mostra lista de servidores do TOML
2. Seleciona servidor → opção: Connect (SSH) ou SFTP
3. SSH: abre terminal integrado via russh
4. SFTP: abre dual-pane com navegação local ↔ remoto

## [S6] Comandos TUI

| Tecla | Ação |
|-------|------|
| `/` | Busca fuzzy |
| `Enter` | Conectar SSH |
| `s` | Abrir SFTP |
| `a` | Adicionar servidor |
| `e` | Editar servidor |
| `d` | Deletar servidor |
| `p` | Pin/despin |
| `q` | Sair |

## [S7] Segurança

- Senhas nunca em texto claro no TOML
- Vault com AES-256-GCM + PBKDF2
- Backup automático antes de escritas
- Permissões de arquivo preservadas
