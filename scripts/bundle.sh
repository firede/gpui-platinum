#!/bin/sh
set -eu

cd "$(dirname "$0")/.."
example="controls"
name="Platinum Controls"
if [ "${1-}" = "--desktop" ]; then
    example="desktop"
    name="Platinum Desk"
    shift
elif [ "${1-}" = "--gallery" ]; then
    example="gallery"
    name="Platinum Gallery"
    shift
fi
cargo build --example "$example" "$@"
app="target/debug/$name.app"
mkdir -p "$app/Contents/MacOS"
cp "target/debug/examples/$example" "$app/Contents/MacOS/$example"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
    <key>CFBundleName</key><string>$name</string>
    <key>CFBundleDisplayName</key><string>$name</string>
    <key>CFBundleIdentifier</key><string>dev.firede.gpui-platinum.$example</string>
    <key>CFBundleExecutable</key><string>$example</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleVersion</key><string>0.1.0</string>
    <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
printf '%s\n' "$PWD/$app"
