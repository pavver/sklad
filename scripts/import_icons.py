#!/usr/bin/env python3
"""Imports the icons listed in assets/icons/icons.toml into assets/icons/.

Downloads Material Symbols Sharp SVGs from github.com/google/material-design-icons
and source logos from Simple Icons (pinned to SIMPLE_ICONS_VERSION),
keeps only the viewBox and path data, and verifies that every SVG file in
assets/icons/ is declared in the manifest (Material or custom).

Usage: python3 tools/import_material_symbols.py [--check]
"""

import re
import sys
import tomllib
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "assets" / "icons" / "icons.toml"
ICONS = ROOT / "assets" / "icons"
SOURCE = (
    "https://raw.githubusercontent.com/google/material-design-icons/master/"
    "symbols/web/{name}/materialsymbolssharp/{name}{fill}_{opsz}px.svg"
)
SIMPLE_ICONS_VERSION = "13.21.0"
SIMPLE_SOURCE = "https://cdn.jsdelivr.net/npm/simple-icons@{version}/icons/{name}.svg"
OPTICAL_SIZES = {20, 24, 40, 48}
NAME_RE = re.compile(r"^[a-z0-9_]+$")


def normalize(svg: str, url: str) -> str:
    view_box = re.search(r'viewBox="([^"]+)"', svg)
    paths = re.findall(r'<path[^>]*\sd="([^"]+)"', svg)
    if not view_box or not paths:
        raise ValueError(f"unexpected SVG at {url}")
    body = "".join(f'<path d="{d}"/>' for d in paths)
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{view_box.group(1)}">{body}</svg>\n'


def download(url: str) -> str:
    with urllib.request.urlopen(url, timeout=30) as response:
        return normalize(response.read().decode("utf-8"), url)


def fetch(name: str, fill: bool, opsz: int) -> str:
    return download(SOURCE.format(name=name, fill="_fill1" if fill else "", opsz=opsz))


def fetch_simple(name: str) -> str:
    return download(SIMPLE_SOURCE.format(version=SIMPLE_ICONS_VERSION, name=name))


def main() -> int:
    check = "--check" in sys.argv[1:]
    icons = tomllib.loads(MANIFEST.read_text("utf-8"))["icons"]
    errors = []

    for file in sorted(ICONS.glob("*.svg")):
        if file.stem not in icons:
            errors.append(f"{file.name} is not declared in assets/icons/icons.toml")

    for key, spec in icons.items():
        target = ICONS / f"{key}.svg"
        if not NAME_RE.match(key):
            errors.append(f"invalid icon key {key!r}")
            continue
        if spec.get("custom"):
            if not target.exists():
                errors.append(f"custom icon {key} has no file {target.name}")
            continue
        simple = spec.get("simple")
        material = spec.get("material")
        opsz = spec.get("opsz", 20)
        if simple:
            if not re.match(r"^[a-z0-9]+$", simple):
                errors.append(f"invalid entry for {key}")
                continue
            svg = fetch_simple(simple)
            origin = f"Simple Icons {simple}"
        elif material and NAME_RE.match(material) and opsz in OPTICAL_SIZES:
            svg = fetch(material, bool(spec.get("fill", False)), opsz)
            origin = f"Material Symbols Sharp {material}"
        else:
            errors.append(f"invalid entry for {key}")
            continue
        current = target.read_text("utf-8") if target.exists() else None
        if current == svg:
            continue
        if check:
            errors.append(f"{target.name} differs from {origin}")
        else:
            target.write_text(svg, "utf-8")
            print(f"wrote {target.relative_to(ROOT)}")

    for error in errors:
        print(f"error: {error}", file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
