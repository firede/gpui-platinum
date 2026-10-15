#!/bin/sh
set -eu

cd "$(dirname "$0")/.."
cargo build --example controls "$@"
app="target/debug/Platinum Controls.app"
mkdir -p "$app/Contents/MacOS"
cp target/debug/examples/controls "$app/Contents/MacOS/controls"
cat > "$app/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
    <key>CFBundleName</key><string>Platinum Controls</string>
    <key>CFBundleDisplayName</key><string>Platinum Controls</string>
    <key>CFBundleIdentifier</key><string>dev.firede.gpui-platinum.controls</string>
    <key>CFBundleExecutable</key><string>controls</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleVersion</key><string>0.1.0</string>
    <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
printf '%s\n' "$PWD/$app"
