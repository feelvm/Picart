#!/bin/sh
# Xcode pre-build phase: builds libeditor_core.a for the selected SDK target
# and copies it into $SRCROOT/build for linking.
#
# One-time Mac setup (see README):
#   curl .../rustup (install rustup), then:
#   rustup target add aarch64-apple-ios aarch64-apple-ios-sim
set -e

# Xcode's build environment doesn't inherit the login shell PATH.
if ! command -v cargo >/dev/null 2>&1; then
  [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
  export PATH="$PATH:$HOME/.cargo/bin"
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found — install rustup and the iOS targets (see README)" >&2
  exit 1
fi

REPO_ROOT="$(cd "$SRCROOT/.." && pwd)"
OUT_DIR="$SRCROOT/build"
mkdir -p "$OUT_DIR"

case "$PLATFORM_NAME" in
  iphoneos)
    TARGET="aarch64-apple-ios" ;;
  iphonesimulator)
    # Apple Silicon simulators. On an Intel Mac, add x86_64-apple-ios and lipo the two .a files.
    TARGET="aarch64-apple-ios-sim" ;;
  *)
    echo "error: unsupported platform '$PLATFORM_NAME'" >&2
    exit 1 ;;
esac

cargo build --manifest-path "$REPO_ROOT/Cargo.toml" --release --target "$TARGET" -p picsart-editor-core
cp "$REPO_ROOT/target/$TARGET/release/libeditor_core.a" "$OUT_DIR/libeditor_core.a"
echo "build-rust.sh: libeditor_core.a ready for $TARGET"
