#!/usr/bin/env bash
KEYLIGHT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ -d "$KEYLIGHT_ROOT/.build-tools/cargo" ]]; then
 export CARGO_HOME="$KEYLIGHT_ROOT/.build-tools/cargo"
 export RUSTUP_HOME="$KEYLIGHT_ROOT/.build-tools/rustup"
 export PATH="$CARGO_HOME/bin:$KEYLIGHT_ROOT/.build-tools/sysroot/usr/bin:$PATH"
 export PKG_CONFIG_PATH="$KEYLIGHT_ROOT/.build-tools/sysroot/usr/lib/x86_64-linux-gnu/pkgconfig:$KEYLIGHT_ROOT/.build-tools/sysroot/usr/share/pkgconfig:${PKG_CONFIG_PATH:-}"
 export LIBRARY_PATH="$KEYLIGHT_ROOT/.build-tools/sysroot/usr/lib/x86_64-linux-gnu:${LIBRARY_PATH:-}"
fi
