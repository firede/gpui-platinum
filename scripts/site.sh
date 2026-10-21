#!/bin/sh
# Builds the web page into a directory (default: site/dist): the page, its
# fonts and its pictures, drawn by the desktop example (desktop --site).
# SITE is the address the page is served at, for the pictures a link to it
# shows.
#
#	./scripts/site.sh
#	SITE=https://example.com ./scripts/site.sh out
#	python3 -m http.server -d site/dist   # then open http://localhost:8000
#
# scripts/deploy.sh builds it for its address and deploys it.
set -eu
cd "$(dirname "$0")/.."

out=${1:-site/dist}
site=${SITE:-http://localhost:8000}
rm -rf "$out"
mkdir -p "$out"
cp -R site/index.html site/site.css site/site.js site/llms.txt site/_headers site/fonts "$out/"

cargo run --quiet --locked --example desktop -- --site "$out"

sed -i.bak -e "s|__SITE__|$site|g" "$out/index.html"
rm "$out/index.html.bak"
echo "$out"
