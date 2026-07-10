# LazySSH — Guia de Releases

## Como criar uma release

### 1. Build release

```bash
cargo build --release
```

O binário fica em `target/release/lazyssh`.

### 2. Criar arquivo compactado

```bash
# Linux x86_64
cp target/release/lazyssh /tmp/lazyssh
cd /tmp && tar czf lazyssh-v0.1.0-linux-x86_64.tar.gz lazyssh

# macOS ARM (se compilando no Mac)
cp target/release/lazyssh /tmp/lazyssh
cd /tmp && tar czf lazyssh-v0.1.0-macos-aarch64.tar.gz lazyssh
```

### 3. Criar release no GitHub

```bash
gh release create v0.1.0 /tmp/lazyssh-v0.1.0-linux-x86_64.tar.gz \
  --title "v0.1.0" \
  --notes "Descrição da release"
```

### 4. Cross-compile (opcional)

Para gerar binários para outras plataformas sem compilar nelas:

```bash
# Instalar targets
rustup target add x86_64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-gnu
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin

# Compilar
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
```

### Estrutura de uma release

```
v0.1.0
├── lazyssh-v0.1.0-linux-x86_64.tar.gz     # Linux x86_64
├── lazyssh-v0.1.0-linux-aarch64.tar.gz    # Linux ARM64 (se disponível)
├── lazyssh-v0.1.0-macos-x86_64.tar.gz     # macOS Intel (se disponível)
└── lazyssh-v0.1.0-macos-aarch64.tar.gz    # macOS Apple Silicon (se disponível)
```

### Checklist

- [ ] `cargo test` passa
- [ ] `cargo build --release` compila sem warnings
- [ ] Binário testado na plataforma alvo
- [ ] Release notes atualizadas
- [ ] Script `install.sh` atualizado (se necessário)
