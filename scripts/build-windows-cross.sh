#!/usr/bin/env bash
# Requires Rust's Windows MSVC target, Java and an SDK prepared with xwin.
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_dir"
export RUFFLE_WINDOWS_SDK="${RUFFLE_WINDOWS_SDK:-$project_dir/.tools/windows-sdk}"
rust_sysroot="$(rustc --print sysroot)"
rust_host="$(rustc -vV | sed -n 's/^host: //p')"
export RUFFLE_RUST_LLD="$rust_sysroot/lib/rustlib/$rust_host/bin/rust-lld"
for required in "$RUFFLE_RUST_LLD" \
    "$RUFFLE_WINDOWS_SDK/crt/lib/x86_64/libcmt.lib" \
    "$RUFFLE_WINDOWS_SDK/sdk/lib/um/x86_64/kernel32.lib" \
    "$RUFFLE_WINDOWS_SDK/sdk/lib/ucrt/x86_64/ucrt.lib"; do
    if [[ ! -f "$required" ]]; then
        echo "Missing build dependency: $required" >&2
        exit 1
    fi
done
command -v java >/dev/null
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="$project_dir/scripts/windows-linker.sh"
# Statically link the C runtime so the core does not require an extra VC runtime DLL.
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS='-C target-feature=+crt-static'
cargo build --release --locked --target x86_64-pc-windows-msvc
mkdir -p dist/windows-x64
cp target/x86_64-pc-windows-msvc/release/ruffle_libretro.dll dist/windows-x64/
cp ruffle_libretro.info README.md LICENSE-MIT LICENSE-APACHE dist/windows-x64/
echo 'Core built: dist/windows-x64/ruffle_libretro.dll'
