#!/usr/bin/env sh
set -eu

if ! command -v cargo >/dev/null 2>&1; then
    printf 'error: cargo not found\n' >&2
    printf 'Install Rust first: https://rustup.rs\n' >&2
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

TARGETS="x86_64-apple-darwin aarch64-apple-darwin"

installed="$(rustup target list --installed 2>/dev/null || true)"
for t in $TARGETS; do
    if ! printf '%s\n' "$installed" | grep -qx "$t"; then
        printf 'Installing Rust target: %s\n' "$t"
        rustup target add "$t"
    fi
done

mkdir -p dist

for t in $TARGETS; do
    printf '\nBuilding urun for %s (release)...\n' "$t"
    cargo build --release --target "$t"
    install -m 755 "target/$t/release/urun" "dist/urun-$t"
    printf '  -> dist/urun-%s\n' "$t"
done

printf '\nDone. Artifacts:\n'
ls -1 dist/urun-*-apple-darwin 2>/dev/null || true
