#!/bin/sh
# Builds the web page and deploys it to Cloudflare, as wrangler.jsonc says.
# Needs npx, and wrangler logged in (npx wrangler login) to the account of
# the domain, or a token in CLOUDFLARE_API_TOKEN, as
# .github/workflows/site.yml gives it as a release is published.
#
#	./scripts/deploy.sh
set -eu
cd "$(dirname "$0")/.."

./scripts/site.sh site/dist
npx --yes wrangler@4 deploy
