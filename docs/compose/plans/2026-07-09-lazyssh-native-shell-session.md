# LazySSH — Sessão SSH Nativa Fora da TUI

**Goal:** Trocar o fluxo atual de conexão integrada por uma sessão nativa em foreground fora da camada TUI, com handoff temporário do raw mode/alternate screen e retorno automático ao app quando a sessão externa terminar (`exit`).

## Contexto / problema atual
- O caminho atual passa pelo `SshTerminalState` e por PTY interno da pilha russh: `src/main.rs:389` chama `app.connect_ssh()`, que por sua vez cria o estado, sobe o canal `data_rx/shell_writer` e renderiza output/input dentro do loop de eventos.
- Como consequência, a TUI continua comandando raw screen/alternate mode enquanto programas remotos nativos também precisam assumir o tty; isso costuma gerar conflitos com paste mode, clear screen e apps como `less`/`vim`.
- Para esse novo fluxo, **não faz sentido reutilizar** `key_event_to_bytes()`, `parse_ansi_spans()` e forwarding de bytes via TUI como canal principal.

## Proposta recomendada
1. Criar um helper local (ou módulo pequeno específico) na ação de `Enter` do servidor selecionado para abrir o shell nativo em foreground:
   - sair temporariamente da área TUI;
   - desligar raw mode, alternate screen e mouse capture;
   - permitir que stdin/stdout sejam usados diretamente pelo processo nativo;
   - restaurar tudo depois que a sessão terminar.
2. Manter separadas as rotinas atuais de PTY integrado (`src/tui/ssh_terminal.rs`, `SshTerminalState`) para não misturar os dois caminhos na mesma sessão.
3. Ao terminar, voltar para um estado anterior previsível (geralmente ServerList) e limpar qualquer state/buffer antigo antes de renderizar de novo.

## Roadmap implementável
- [ ] `src/main.rs` — substituir a conexão direta via PTY pelo helper de session nativa na ação de servidor selecionado; talvez incluir aqui um pequeno guard/RAII para garantir restore do raw mode em qualquer saída inesperada.
- [ ] `src/tui/app.rs` — garantir que o estado da view volte ao ponto correto depois que a sessão externa terminar e não fique `ShellChannel/data_rx` vivo além do necessário.
- [ ] Se o helper crescer, extrair para um módulo pequeno dedicado; nada de feature flag)
- [ ] Definir antes da implementação como `Auth::Password` vai funcionar fora da TUI (stdin/env/prompt controlado) sem reinventar o loop de teclado atual]
  
## Critérios de aceite
- A sessão externa é iniciada fora do controle direto da TUI durante todo seu lifecycle.
- `exit` no shell encerrado faz o app voltar automaticamente para a tela anterior/ServerList.
- Raw mode, alt screen e mouse capture são preservados/restaurados corretamente mesmo em erro)
- Os testes devem cobrir transição de state com launcher mock/fake process; validação manual em SSH remoto deve confirmar comportamento natural sem poluição da TUI)

Arquivos prováveis: `src/main.rs`, `src/tui/app.rs` e, se couber, `tests/integration_test.rs`.