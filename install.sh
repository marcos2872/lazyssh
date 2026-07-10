#!/bin/bash
set -e

# LazySSH Installer
# Instala o LazySSH binário no sistema
# Suporta: Linux (bash, zsh, fish) e macOS (bash, zsh, fish)

REPO="marcos2872/lazyssh"
VERSION="${1:-v0.1.0}"
INSTALL_DIR="$HOME/.local/bin"
BINARY_NAME="lazyssh"

# Cores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[lazyssh]${NC} $1"; }
warn()  { echo -e "${YELLOW}[lazyssh]${NC} $1"; }
error() { echo -e "${RED}[lazyssh]${NC} $1"; exit 1; }

# Detectar plataforma
detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)
            case "$arch" in
                x86_64|amd64)  PLATFORM="linux-x86_64" ;;
                aarch64|arm64) PLATFORM="linux-aarch64" ;;
                *) error "Arquitetura Linux não suportada: $arch" ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64)  PLATFORM="macos-x86_64" ;;
                arm64)   PLATFORM="macos-aarch64" ;;
                *) error "Arquitetura macOS não suportada: $arch" ;;
            esac
            ;;
        *) error "Sistema operacional não suportado: $os" ;;
    esac

    info "Plataforma detectada: $PLATFORM"
}

# Verificar dependências
check_deps() {
    local missing=()

    command -v curl >/dev/null 2>&1 || missing+=("curl")
    command -v tar >/dev/null 2>&1 || missing+=("tar")

    if [ ${#missing[@]} -gt 0 ]; then
        error "Dependências faltando: ${missing[*]}"
    fi
}

# Baixar e instalar
install_binary() {
    local url="https://github.com/${REPO}/releases/download/${VERSION}/lazyssh-${VERSION}-${PLATFORM}.tar.gz"
    local tmp_dir
    tmp_dir="$(mktemp -d)"

    info "Baixando $url ..."
    curl -fsSL "$url" -o "$tmp_dir/lazyssh.tar.gz" || error "Falha ao baixar. Verifique se a release $VERSION existe."

    info "Extraindo..."
    tar xzf "$tmp_dir/lazyssh.tar.gz" -C "$tmp_dir"

    # Criar diretório de instalação
    mkdir -p "$INSTALL_DIR"

    # Copiar binário
    cp "$tmp_dir/lazyssh" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"

    # Limpar
    rm -rf "$tmp_dir"

    info "Binário instalado em: $INSTALL_DIR/$BINARY_NAME"
}

# Configurar PATH
setup_path() {
    local shell_name
    shell_name="$(basename "$SHELL" 2>/dev/null || echo "bash")"

    # Verificar se já está no PATH
    if echo "$PATH" | tr ':' '\n' | grep -q "^$INSTALL_DIR$"; then
        info "Diretório já está no PATH"
        return
    fi

    # Detectar arquivo de configuração do shell
    local rc_file=""
    case "$shell_name" in
        bash)
            if [ -f "$HOME/.bashrc" ]; then
                rc_file="$HOME/.bashrc"
            elif [ -f "$HOME/.bash_profile" ]; then
                rc_file="$HOME/.bash_profile"
            elif [ -f "$HOME/.profile" ]; then
                rc_file="$HOME/.profile"
            fi
            ;;
        zsh)
            rc_file="$HOME/.zshrc"
            ;;
        fish)
            rc_file="$HOME/.config/fish/config.fish"
            ;;
        *)
            if [ -f "$HOME/.profile" ]; then
                rc_file="$HOME/.profile"
            fi
            ;;
    esac

    if [ -n "$rc_file" ]; then
        # Verificar se já tem a linha
        if ! grep -q "export PATH.*$INSTALL_DIR" "$rc_file" 2>/dev/null; then
            if [ "$shell_name" = "fish" ]; then
                echo "set -gx PATH $INSTALL_DIR \$PATH" >> "$rc_file"
            else
                echo "" >> "$rc_file"
                echo "# LazySSH" >> "$rc_file"
                echo "export PATH=\"\$HOME/.local/bin:\$PATH\"" >> "$rc_file"
            fi
            info "PATH adicionado em $rc_file"
            warn "Execute: source $rc_file ou reinicie o terminal"
        else
            info "PATH já configurado em $rc_file"
        fi
    else
        warn "Não foi possível detectar o arquivo de shell config"
        warn "Adicione manualmente: export PATH=\"\$HOME/.local/bin:\$PATH\""
    fi
}

# Verificar se sshpass está instalado
check_sshpass() {
    if ! command -v sshpass >/dev/null 2>&1; then
        warn "sshpass não encontrado. É necessário para autenticação por senha."
        warn "Instale com: sudo apt install sshpass (Linux) ou brew install hudochenkov/sshpass/sshpass (macOS)"
    fi
}

# Verificar keyring
check_keyring() {
    info "Para armazenamento seguro de senhas, instale o keyring do seu desktop:"
    info "  Linux (GNOME): já vem com o sistema"
    info "  Linux (KDE): sudo apt install libsecret-1-dev"
    info "  macOS: já vem com o sistema (Keychain)"
}

# Main
main() {
    echo ""
    echo "  LazySSH Installer"
    echo "  ================="
    echo ""

    detect_platform
    check_deps
    install_binary
    setup_path
    check_sshpass
    check_keyring

    echo ""
    info "Instalação concluída!"
    info "Execute: lazyssh"
    echo ""
}

main "$@"
