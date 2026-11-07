#!/bin/bash
set -euxo pipefail
# Builds the vendored Hermes for host tests. The default matches the
# jsi-tests build.rs link line (vendor/hermes/build). Pass a build type
# to vary it, e.g. `build-hermes.sh Debug` to reproduce a debug-slice
# behavior difference.
BUILD_TYPE="${1:-Release}"
cd hermes
# newer hermes (RN 0.88+) refuses to configure without an explicit build type
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE="$BUILD_TYPE"
cmake --build ./build
