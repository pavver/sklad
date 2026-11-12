#!/bin/sh
# Builds the website into site/dist: index.html in English and uk.html in Ukrainian.
# Trunk writes every page as index.html, so the Ukrainian one is built apart and renamed.
set -eu
cd "$(dirname "$0")/../site"
rm -rf dist dist-uk
trunk build --release=true index.html
trunk build --release=true --dist dist-uk uk.html
cp -n dist-uk/* dist/ 2>/dev/null || true
cp dist-uk/index.html dist/uk.html
rm -rf dist-uk dist/.trunk-lock
