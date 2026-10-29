#!/bin/bash
# CocoaPods records vendored_libraries into the link settings at
# `pod install` time — and silently DROPS the entry if the archive is
# missing or has no valid architecture (it probes with lipo). So a real
# arm64 placeholder must exist before Xcode ever runs the RustTurboBuild
# script phase; the script phase overwrites it with the actual build.
set -euo pipefail
cd "$(dirname "$0")/.."
if [ ! -f librust-app.a ]; then
  echo "RustTurbo: installing placeholder librust-app.a (replaced by the RustTurboBuild script phase)"
  cp scripts/placeholder.a librust-app.a
fi
