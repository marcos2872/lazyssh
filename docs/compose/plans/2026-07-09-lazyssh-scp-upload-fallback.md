# LazySSH — SCP Upload para Arquivos Grandes

**Objetivo:** Usar `scp` nativo para uploads quando o SFTP falha por limite de ~1GB, ou como método alternativo.

---

## Contexto

O SFTP tem um limite de ~1GB por sessão no servidor. Arquivos maiores falham. O `scp` não tem essa limitação porque transfere o arquivo inteiro de uma vez sem o protocolo SFTP de packets.

---

## Tarefa 1 — Detectar limite e fallback automático

### Arquivos
- `src/sftp/service.rs` — método `upload_with_scp`
- `src/tui/app.rs` — método `sftp_upload_with_scp`
- `src/main.rs` — handler de upload com fallback

### Implementação
1. Adicionar método `upload_with_scp` ao `SftpServiceSession`:
   - Usa `std::process::Command::new("scp")` com argumentos seguros
   - Argumentos: `-P port`, `-i key_path`, `local_path`, `user@host:remote_path`
   - Para password: usar `sshpass -p password scp ...`
2. No handler de upload em `main.rs`:
   - Tentar SFTP primeiro
   - Se falhar com erro de sessão/conexão, tentar SCP
   - Notificar o usuário: "SFTP falhou, tentando SCP..."
3. Método `sftp_upload_with_scp` no App:
   - Não bloqueia a TUI (usa `spawn_blocking`)
   - Reporta progresso via canal

### Testes
- Teste de construção do comando SCP
- Teste de fallback quando SFTP falha

---

## Tarefa 2 — Progresso via SCP

### Arquivos
- `src/sftp/service.rs` — parse de output do SCP
- `src/main.rs` — handler de progresso SCP

### Implementação
1. SCP mostra progresso no stderr (formato: `filename 100% 1.2GB 00:30`)
2. Parsear o output do SCP para extrair porcentagem
3. Enviar progresso via canal `progress_tx`
4. Mostrar na barra de progresso da TUI

### Testes
- Teste de parse do output SCP

---

## Tarefa 3 — Integração com UI existente

### Arquivos
- `src/main.rs` — unificar handler de upload
- `src/tui/sftp_browser.rs` — status bar mostra "SCP" quando ativo

### Implementação
1. No handler de upload, verificar tamanho do arquivo:
   - Se < 1GB: tentar SFTP primeiro
   - Se >= 1GB: usar SCP diretamente
2. Mostrar na barra de status: "Upload via SCP" ou "Upload via SFTP"
3. Mantar botão `u` para upload — a detecção é automática

### Testes
- Teste de detecção de tamanho
- Teste de notificação do método usado

---

## Ordem

1. Tarefa 1 (fallback automático)
2. Tarefa 2 (progresso SCP)
3. Tarefa 3 (integração UI)

## Validação

- `cargo build`
- `cargo test`
- Teste manual: upload de arquivo < 1GB (SFTP)
- Teste manual: upload de arquivo > 1GB (SCP)
