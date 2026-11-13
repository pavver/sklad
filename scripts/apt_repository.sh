#!/bin/sh
# Turns a folder of .deb files into a flat apt repository signed with the default gpg key:
#   deb [signed-by=/usr/share/keyrings/sklad.gpg] https://pavver.github.io/sklad/debian ./
set -eu
cd "$1"
apt-ftparchive packages . > Packages
gzip -9kf Packages
apt-ftparchive \
  -o APT::FTPArchive::Release::Origin=sklad \
  -o APT::FTPArchive::Release::Label=sklad \
  -o APT::FTPArchive::Release::Suite=stable \
  -o APT::FTPArchive::Release::Architectures=amd64 \
  release . > Release
gpg --batch --yes --clearsign -o InRelease Release
gpg --batch --yes --armor --detach-sign -o Release.gpg Release
