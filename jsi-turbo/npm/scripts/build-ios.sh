#!/bin/bash
# Invoked by Xcode (RustTurboBuild script phase, before_compile).
# Maps the Xcode build settings onto a Rust target and runs the
# discover+aggregate+build hook for this app.
set -euo pipefail

# Xcode's PATH can be minimal; add the usual homes for node and cargo
export PATH="$PATH:/opt/homebrew/bin:/usr/local/bin:$HOME/.cargo/bin"

POD_ROOT="${PODS_TARGET_SRCROOT:?PODS_TARGET_SRCROOT not set}"
# <app>/node_modules/react-native-rust-turbo -> <app>
APP_ROOT="$(cd "$POD_ROOT/../.." && pwd)"

case "${PLATFORM_NAME:-}" in
  iphonesimulator) RUST_TARGET="aarch64-apple-ios-sim" ;;
  iphoneos)        RUST_TARGET="aarch64-apple-ios" ;;
  *)
    echo "RustTurboBuild: unsupported platform '${PLATFORM_NAME:-}'"
    exit 1
    ;;
esac

case "${CONFIGURATION:-}" in
  Release) PROFILE="--release" ;;
  *)       PROFILE="" ;;
esac

LIB="$(node "$POD_ROOT/scripts/build.js" \
  --app "$APP_ROOT" --target "$RUST_TARGET" $PROFILE | tail -n1)"

cp "$LIB" "$POD_ROOT/librust-app.a"
echo "RustTurboBuild: installed $(basename "$LIB") ($RUST_TARGET, ${CONFIGURATION:-Debug})"
