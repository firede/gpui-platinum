#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

cargo build --locked --example gallery --example desktop "$@"
examples="${CARGO_TARGET_DIR:-target}/debug/examples"
for page in 1 2 3 4 5; do
    suffix="-$page"
    if [ "$page" = 1 ]; then
        suffix=""
    fi
    "$examples/gallery" --page "$page" --png "docs/gallery$suffix@2x.png" --scale 2
done
"$examples/desktop" --lang en --png docs/desktop@2x.png --scale 2
"$examples/desktop" --lang zh-Hans --png docs/desktop-zh-hans@2x.png --scale 2
