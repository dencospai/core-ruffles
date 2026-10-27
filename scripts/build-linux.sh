#!/usr/bin/env bash
# Native Linux x64 build. Requires Rust, a C toolchain and Java 17.
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_dir"
target=x86_64-unknown-linux-gnu
if [[ "$(uname -s)" != Linux || "$(uname -m)" != x86_64 ]]; then
    echo 'This script builds natively on Linux x86_64.' >&2
    exit 1
fi
command -v java >/dev/null
cargo build --release --locked --target "$target"
artifact_dir="${CARGO_TARGET_DIR:-$project_dir/target}/$target/release"
mkdir -p dist/linux-x64
# Use the usual libretro filename, without Cargo's lib prefix.
cp "$artifact_dir/libruffle_libretro.so" dist/linux-x64/ruffle_libretro.so
cp ruffle_libretro.info README.md VALIDATION.md THIRD_PARTY.md LICENSE-MIT LICENSE-APACHE dist/linux-x64/
cp tests/fixtures/smoke.swf tests/fixtures/cursor.swf dist/linux-x64/
echo 'Core built: dist/linux-x64/ruffle_libretro.so'
