#!/bin/sh
# Builds the web page into a directory (default: site/dist): the page, its
# fonts and the pictures in docs, with its links to the repository's files
# made links to them on GitHub.
#
#	./scripts/site.sh
#	python3 -m http.server -d site/dist   # then open http://localhost:8000
#
# scripts/deploy.sh builds it and deploys it.
set -eu
cd "$(dirname "$0")/.."

out=${1:-site/dist}
repo=https://github.com/firede/gpui-platinum/blob/main
rm -rf "$out"
mkdir -p "$out/docs"
cp -R site/index.html site/site.css site/site.js site/llms.txt site/icon.svg site/_headers site/fonts "$out/"
cp docs/*.png "$out/docs/"
for file in "$out/index.html" "$out/site.js"; do
    sed -E -i.bak \
        -e "s#\.\./([A-Za-z./-]*\.(md|rs)|LICENSE)#$repo/\1#g" \
        -e "s#\.\./docs/#docs/#g" \
        "$file"
    rm "$file.bak"
done
echo "$out"
