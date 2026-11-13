#!/bin/sh
# Downloads the assets matching a pattern from the latest releases other than the one being built,
# so a client whose package index predates the newest release still finds the file it names:
#   scripts/previous_releases.sh '*.deb' repository
# It needs GH_TOKEN, GITHUB_REPOSITORY and GITHUB_REF_NAME, as GitHub Actions sets them.
# KEEP says how many earlier releases to take; five by default.
set -eu
pattern=$1
folder=$2
keep=${KEEP:-5}
mkdir -p "$folder"
downloads=$(mktemp -d)
gh api "repos/$GITHUB_REPOSITORY/releases?per_page=$((keep + 1))" \
  --jq '.[] | select(.draft | not) | .tag_name' |
  grep -vxF "$GITHUB_REF_NAME" |
  head -n "$keep" |
  while read -r tag; do
    gh release download "$tag" --repo "$GITHUB_REPOSITORY" --pattern "$pattern" --dir "$downloads"
  done
# A file built in this run wins over the one of the same name in a release.
for file in "$downloads"/*; do
  [ -e "$file" ] || continue
  [ -e "$folder/${file##*/}" ] || mv "$file" "$folder/"
done
rm -rf "$downloads"
