# LazySSH — SCP como Upload Nativo

**Objetivo:** Usar `scp` nativo como método principal de upload, substituindo o SFTP que tem limite de ~1GB.

---

## Por quê SCP como nativo

- SCP transfere o arquivo inteiro de uma vez — sem limite de ~1GB
- SCP mostra progresso no stderr automaticamente
- SCP é mais simples e confiável para uploads
- SFTP continua disponível para operações de filesystem (mkdir, rename, etc)

---

## Tarefa 1 — Upload via SCP

### Arquivos
- `src/sftp/service.rs` — método `upload_with_scp`
- `src/tui/app.rs` — método `sftp_upload_with_scp`
- `src/main.rs` — handler de upload usa SCP

### Implementação
1. Método `upload_with_scp` no App:
   - `std::process::Command::new("scp")` com argumentos seguros
   - `-P port`, `-i key_path`, `local_path`, `user@host:remote_path`
   - Para password: `sshpass -p password scp ...`
   - Retorna `ExitStatus` (não precisa de canal de progresso)
2. Handler de upload em `main.rs`:
   - Usa `spawn_blocking` para não bloquear TUI
   - Mostra notificação "Enviando via SCP..."
   - Após completar, refresh do diretório remoto
3. Barra de progresso:
   - SCP mostra progresso no stderr (parsear output)
   - Ou simplesmente mostrar "Enviando..." sem progresso granular

### Testes
- Teste de construção do comando SCP
- Teste de upload com mock

---

## Tarefa 2 — Progresso via output do SCP

### Arquivos
- `src/tui/app.rs` — parsing do output SCP
- `src/main.rs` — leitura do stderr do processo

### Implementação
1. SCP mostra no stderr: `filename 100% 1.2GB 00:30`
2. Ler stderr do processo SCP em thread separada
3. Parsear para extrair porcentagem
4. Enviar via canal de progresso para a TUI
5. Mostrar na barra de progresso

### Testes
- Teste de parse do output SCP

---

## Tarefa 3 — Integração com UI

### Arquivos
- `src/main.rs` — handler de upload unificado
- `src/tui/sftp_browser.rs` — status bar mostra "SCP"

### Implementação
1. Botão `u` usa SCP diretamente (não SFTP)
2. Barra de status: "Upload via SCP" quando ativo
3. Notificação: "Enviando via SCP..." / "SCP concluído"
4. Refresh automático do diretório após upload

### Testes
- Teste de integração upload + refresh

---

## Ordem

1. Tarefa 1 (upload via SCP)
2. Tarefa 2 (progresso)
3. Tarefa 3 (integração UI)

## Validação

- `cargo build`
- `cargo test`
- Teste manual: upload de arquivo grande via SCP
