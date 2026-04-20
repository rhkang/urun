#!/usr/bin/env sh
set -eu

if ! command -v cargo >/dev/null 2>&1; then
    printf 'error: cargo not found\n' >&2
    printf 'Install Rust first: https://rustup.rs\n' >&2
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

printf 'Building urun (release)...\n'
cargo build --release

DEST="$HOME/.local/bin"
mkdir -p "$DEST"
install -m 755 target/release/urun "$DEST/urun"
printf 'Installed: %s/urun\n' "$DEST"

case ":${PATH:-}:" in
    *":$DEST:"*)
        ;;
    *)
        printf '\n'
        printf 'warning: %s is not in your PATH.\n' "$DEST"
        printf 'Add this line to your shell rc (~/.bashrc, ~/.zshrc, ~/.profile, etc.):\n\n'
        printf '    export PATH="$HOME/.local/bin:$PATH"\n\n'
        printf 'Then restart your shell or run: source ~/.bashrc (or your rc file).\n'
        ;;
esac
