#!/usr/bin/env python3
"""Update release versions in Cargo and AppStream files."""

from datetime import date
from pathlib import Path
import re
import sys

version = sys.argv[1]
if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
    raise SystemExit("version must be MAJOR.MINOR.PATCH")

cargo = Path("Cargo.toml")
cargo.write_text(
    re.sub(
        r'^version = "[^"]+"',
        f'version = "{version}"',
        cargo.read_text(encoding="utf-8"),
        count=1,
        flags=re.MULTILINE,
    ),
    encoding="utf-8",
)
metadata = Path("resources/io.github.cosmic_utils.compass.metainfo.xml")
text = metadata.read_text(encoding="utf-8")
if f'release version="{version}"' not in text:
    entry = (
        f'    <release version="{version}" date="{date.today().isoformat()}">'
        "<description><p>Bug fixes and improvements.</p></description></release>"
        + chr(10)
    )
    text = text.replace("  <releases>" + chr(10), "  <releases>" + chr(10) + entry)
metadata.write_text(text, encoding="utf-8")
