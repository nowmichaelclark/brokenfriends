#!/usr/bin/env bash
# Build the Driftloom extension and copy it where Godot looks for it.
#   rust/build.sh linux    the laptop (Fedora): needs Rust
#   rust/build.sh web      the browser: needs nightly-2026-10-24 Rust and Emscripten 4.0.20
#                          (see rust/README.md)
#   rust/build.sh android  phones: needs the Android NDK (ANDROID_NDK_HOME) and cargo-ndk
set -euo pipefail
cd "$(dirname "$0")/driftloom-godot"
bin=../../game/addons/driftloom/bin
case "${1:-linux}" in
  linux)
    cargo build --release
    mkdir -p "$bin/linux"
    cp target/release/libdriftloom_godot.so "$bin/linux/"
    ;;
  web)
    : "${GODOT4_BIN:?set GODOT4_BIN to a Godot 4.7.1 binary (the web build generates its API from it)}"
    # Godot's web engine has no WebAssembly exception handling, and Rust's
    # Emscripten target unwinds with it, so the browser build aborts on a
    # panic instead of unwinding (std rebuilt with panic_abort).
    CARGO_PROFILE_RELEASE_PANIC=abort \
      cargo +"${NIGHTLY:-nightly-2026-10-24}" build --release -Zbuild-std=std,panic_abort --target wasm32-unknown-emscripten
    mkdir -p "$bin/web"
    cp target/wasm32-unknown-emscripten/release/driftloom_godot.wasm "$bin/web/"
    ;;
  android)
    # 64-bit phones (most since about 2017) and older 32-bit ones; API 24
    # is Android 7, the oldest Godot 4 supports.
    cargo ndk -t arm64-v8a -t armeabi-v7a -P 24 build --release
    mkdir -p "$bin/android/arm64" "$bin/android/arm32"
    cp target/aarch64-linux-android/release/libdriftloom_godot.so "$bin/android/arm64/"
    cp target/armv7-linux-androideabi/release/libdriftloom_godot.so "$bin/android/arm32/"
    ;;
  *)
    echo "usage: rust/build.sh linux|web|android" >&2
    exit 1
    ;;
esac
find "$bin" -type f -printf "built: %s bytes  %p\n"
