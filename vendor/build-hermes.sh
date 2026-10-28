#!/bin/bash
set -euxo pipefail
cd hermes
# newer hermes (RN 0.88+) refuses to configure without an explicit build type
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build ./build
