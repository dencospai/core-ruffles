#!/usr/bin/env bash
# Link Windows MSVC objects from Linux using Rust's bundled LLVM linker.
set -euo pipefail
: "${RUFFLE_WINDOWS_SDK:?Set RUFFLE_WINDOWS_SDK to the xwin SDK directory}"
: "${RUFFLE_RUST_LLD:?Set RUFFLE_RUST_LLD to the Rust LLVM linker}"
exec "$RUFFLE_RUST_LLD" -flavor link "$@" \
    "/libpath:$RUFFLE_WINDOWS_SDK/crt/lib/x86_64" \
    "/libpath:$RUFFLE_WINDOWS_SDK/sdk/lib/um/x86_64" \
    "/libpath:$RUFFLE_WINDOWS_SDK/sdk/lib/ucrt/x86_64"
