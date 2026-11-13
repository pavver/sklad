#!/bin/sh
# Builds the website into site/dist: index.html in English, and uk.html, fr.html and ru.html.
# Trunk writes every page as index.html, so each translation is built apart and renamed.
set -eu
cd "$(dirname "$0")/../site"
rm -rf dist dist-page
trunk build --release=true index.html
for page in uk fr ru; do
  trunk build --release=true --dist dist-page "$page.html"
  cp -n dist-page/* dist/ 2>/dev/null || true
  cp dist-page/index.html "dist/$page.html"
  rm -rf dist-page
done
rm -rf dist/.trunk-lock
