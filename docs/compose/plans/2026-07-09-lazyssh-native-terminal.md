# LazySSH — Shell SSH Nativo Fora da TUI

**Goal:** Trocar o fluxo atual de conexão interativa por um shell nativo ocupado pelo processo externo, sem a TUI interceptar stdin/stdout/ANSI. Ao fechar com `exit`, o programa volta para a mesma TUI e mantém o servidor selecionado/lista original.

---

## Observação do plano

Este documento assume o fluxo recomendado: **mesmo terminal, mas saindo temporariamente do modo alternate screen/raw mode da TUI**. Assim a conexão SSH externa pega o terminal inteiro sem conflito com controle de teclado, ASI ou integrações. Depois que o processo externo encerra, a aplicação reativa a TUI e continua no `ServerList`.

Se o requisito for “nova janela/terminal emulador” (por exemplo `alacritty` + ssh) em vez de bloquear o terminal atual, isso altera este plano na parte do lançador.

---

## Fluxo recomendado

1. Usuário seleciona um servidor na lista e aperta **Enter**.
2. A TUI desativa raw mode e sai da alternate screen de forma segura.
3. O programa abre uma sessão nativa de terminal executando o comando SSH correspondente ao servidor escolhido.
4. O shell externo é responsável por autenticação, PTY, prompt e encerramento.
5. Quando o usuário fecha com `exit`/Ctrl-D, a TUI volta ao mesmo estado anterior e continua mostrando a lista/tela inicial.

Esse fluxo substitui o caminho integrado atual que usa canal SSH + parser ANSI dentro da TUI:

- Entrada/list `Enter` → `app.connect_ssh()` em [`src/main.rs:389`](/home/marcos/Projetos/lazyssh/src/main.rs#L389)
- Abertura de sessão nativa `russh` no app em [`src/tui/app.rs:529`](/home/marcos/Projetos/lazyssh/src/tui/app.rs#L529)
- Envia teclas para o shell interno na TUI em [`src/main.rs:813`](/home/marcos/Projetos/lazyssh/src/main.rs#L813)

---

## Escopo da mudança proposta

### Entra no escopo

- Criar uma função/helper de "native shell session" no projeto.
- Usar o comando SSH nativo sem passar pelo parser/renderizador da TUI.
- Ligar **Enter** na lista para abrir essa sessão nativa.
- Garantir restauração automática da TUI mesmo quando o processo externo encerrar ou falhar.
- Manter sessões/integrações existentes funcionais enquanto a função nova for usada no fluxo normal.

### Fora do escopo agora

- Reconstruir todo um modo interno para shell SSH nativo apenas usando `russh`.
- Redesenhar autenticação, inventário e SFTP só por causa dessa mudança de UX.
- Remover qualquer componente legado não conectado ao novo fluxo de uma única vez; deixar como opção/legado por compatibilidade, se fizer sentido no código atual.

---

## Decisão arquitetural proposta

Preferir **executar no processo externo** antes de tentar forçar o terminal emulador/TUI a conviver com output e keys da shell. A TUI fica somente responsável por *pausar sua própria captura* sem ficar intermediando bytes do SSH.

### Por quê

- Elimina conflito entre raw mode/alternate screen da TUI e controle direto de um shell.
- Evita problemas de parsing ANSI e comportamento estranho em ferramentas integradas.
- Respeita prompts reais do processo externo: senha, `sudo`, MFA, agent etc., sem que a aplicação precise inspecionar ou forçar texto.

### Tradeoff

- A implementação usa o comando SSH disponível no ambiente, então é menos "puro" do que uma conexão totalmente no Rust; porém resolve melhor a intenção prática da solicitação e mantém a TUI fora do caminho durante a sessão interativa.

---

## Implantação por etapas

### Tarefa 1 — Criar helper/estado mínimo para abertura nativa

**Files:** Modify `src/main.rs`, optionally add testable unit helpers in `src/tui/app.rs` or `src/ssh/service.rs`

- Criar uma função que recebe o `Server` selecionado e monta a chamada externa de forma segura.
- Preferir interface via `std::process::Command` sem montar argumento por concatenação vulnerável; usar vetor de argumentos para preservar segurança contra injeção.
- Não chamar essa rota da TUI enquanto ainda estiver no modo direto/integrado atual.

> Observação: como o código atual usa variáveis de servidor, a função precisa descobrir quais dados precisam vir do `Server` e quais podem ser resolvidos pelo processo externo próprio SSH. Exemplos que devem entrar na planilha de investigação antes da implementação: porta implícita/ou explícita, usuário default e chaves configuradas no TOML podem exigir tratamento adicional.

---

### Tarefa 2 — Adicionar transição temporária para modo nativo

**Files:** Modify `src/main.rs`

- Ao acionar a conexão nativa pela lista:
  - chamar `disable_raw_mode()` se ainda estiver ativo;
  - chamar `leave alternate screen`;
  - desativar mouse capture se existir no fluxo usado;
  - executar/espere o comando externo.
- Depois que o comando encerrar, reconstituir exatamente a ordem para voltar:
  - `enable_raw_mode()`;
  - entrar de novo em alternate screen;
  - recriar/renderizar a TUI sem manter estado de shell anterior misturado na tela;
  - voltar para `ServerList` ou o último visualizador desejado.

O princípio é simples: **só uma coisa controla o terminal por vez**, e ela troca controle explicitamente entre TUI ↔ shell externo.

---

### Tarefa 3 — Ligar a ação do usuário ao novo fluxo

**Files:** Modify `src/main.rs`, optionally testable helpers in app layer

- Atualizar o handler de **Enter** na lista para chamar o helper nativo em vez de `app.connect_ssh()`.
- Remover ou desativar temporariamente o trecho da TUI que reencaminha teclas para o shell interno, caso ele passe a ser uma rota não mais usada.
- Garantir que se aparecer algum fallback visual por um instante do outro modo antigo, ele não fique consumindo eventos durante o processo nativo.

---

### Tarefa 4 — Tratar encerramento e volta à TUI

**Files:** Modify `src/main.rs`, optionally add tests around cleanup orchestration

- Quando o shell externo sai com status normal ou não-zero:
  - reativar raw mode/alternate screen;
  - limpar qualquer estado de sessão anterior do app;
  - voltar para a lista original sem tentar reapertar prompts.
- Para erro inesperado no início da abertura:
  - mostrar erro na própria TUI, reentrar nela e manter o fluxo seguro.

---

### Tarefa 5 — Proteger contra regressões

**Files:** Modify `src/main.rs` and/or add tests/helpers as needed

- Reforçar a sequência de cleanup em um caminho testável para o casal:
  - raw mode on/off;
  - alternate screen enter/leave;
  - retorno à lista depois do encerramento.
- Não deixar o antigo parser/integrador de shell como ponto cego não coberto.

---

## Pontos de risco e decisões que preciso observar antes da codificação real

1. O sistema SSH nativo precisa estar disponível no ambiente; se faltar, a TUI deve continuar funcionando para listas/edit/sftp sem depender dele.
2. A forma mais segura é chamar o binário SSH local com argumentos construídos por vetor, mas isso aumenta dependência do cliente externo e exige mapeamento cuidadoso dos campos de configuração atual.
3. O fluxo atual já cria sessão `russh` e retransmite teclas para a shell dentro da TUI; substituir este caminho sem quebrar código restante é mais simples se a nova rota ficar como uma decisão separada por nome (por exemplo, conexão interativa nativa vs integração interna), mesmo que no produto final só um fique ativo.

---

## Validação esperada

- `cargo build`
- Testes existentes: `src/main.rs`, módulos ligados à alteração e quaisquer tests globais relevantes do projeto.
- Verificação manual sugerida: abrir a lista, selecionar servidor válido e confirmar que a sessão aberta não passa pela TUI; depois fechar com `exit` e observar volta automática para o mesmo estado anterior da aplicação.

---

## Resumo executivo

Recebi a solicitação real: **não quero mais a shell integrada dentro da TUI.** Quero um fluxo de **sessão nativa do terminal SSH** que não seja controlado pela TUI, e depois retorne automaticamente quando eu fechar com `exit`. Eu proponho resolver isso fazendo a app temporariamente abandonar o modo raw/alternate da TUI, executar uma sessão SSH externa segura, espalhar retornos e reentrar na mesma TUI sem misturar os dois controles. O próximo passo seria transformar esse plano em implementação mínima usando helper dedicado e ligação de `Enter` para o novo fluxo nativo; se quiser, eu sigo pela parte prática agora depois de ajustar ou validar essa rota proposta.
