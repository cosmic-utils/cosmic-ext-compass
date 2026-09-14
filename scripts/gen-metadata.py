#!/usr/bin/env python3
"""Generate localized desktop and AppStream fields."""

from __future__ import annotations

import re
from pathlib import Path
from xml.sax.saxutils import escape

ROOT = Path(__file__).resolve().parent.parent
I18N = ROOT / "i18n"
DESKTOP = ROOT / "resources/org.cosmic_utils.compass.desktop"
METAINFO = ROOT / "resources/org.cosmic_utils.compass.metainfo.xml"
FTL = "compass.ftl"
SOURCE = "en"
NL = chr(10)


def translations() -> dict[str, dict[str, str]]:
    values: dict[str, dict[str, str]] = {}
    for path in sorted(I18N.glob(f"*/{FTL}")):
        for line in path.read_text(encoding="utf-8").splitlines():
            match = re.match(r"^([a-z0-9-]+) = (.*)$", line)
            if match:
                values.setdefault(match.group(1), {})[path.parent.name] = match.group(2)
    return values


def desktop_text(values: dict[str, dict[str, str]]) -> str:
    mapping = {
        "Name": "compass",
        "Comment": "desktop-comment",
        "Keywords": "desktop-keywords",
    }
    source = [
        line
        for line in DESKTOP.read_text(encoding="utf-8").splitlines()
        if not re.match(r"^[A-Za-z]+\[[^]]+\]=", line)
    ]
    output: list[str] = []
    for line in source:
        field = line.partition("=")[0]
        key = mapping.get(field)
        if key is None:
            output.append(line)
            continue
        output.append(f"{field}={values[key][SOURCE]}")
        for language, value in sorted(values[key].items()):
            if language != SOURCE:
                output.append(f"{field}[{language.replace('-', '_')}]={value}")
    return NL.join(output) + NL


def element_group(tag: str, values: dict[str, str], indent: str) -> str:
    lines = [f"{indent}<{tag}>{escape(values[SOURCE])}</{tag}>"]
    lines.extend(
        f'{indent}<{tag} xml:lang="{language}">{escape(value)}</{tag}>'
        for language, value in sorted(values.items())
        if language != SOURCE
    )
    return NL.join(lines)


def replace_group(text: str, tag: str, values: dict[str, str]) -> str:
    pattern = re.compile(
        rf"^(\s*)<{tag}(?![^>]*xml:lang)[^>]*>.*?</{tag}>"
        rf"(?:\n\s*<{tag} xml:lang=.*?</{tag}>)*",
        re.MULTILINE,
    )
    match = pattern.search(text)
    if not match:
        raise SystemExit(f"missing <{tag}> in {METAINFO.name}")
    replacement = element_group(tag, values, match.group(1))
    return text[: match.start()] + replacement + text[match.end() :]


def main() -> None:
    values = translations()
    required = [
        "compass",
        "desktop-comment",
        "desktop-keywords",
        "metainfo-summary",
        "metainfo-description",
    ]
    missing = [key for key in required if SOURCE not in values.get(key, {})]
    if missing:
        raise SystemExit(f"missing English Fluent keys: {', '.join(missing)}")
    DESKTOP.write_text(desktop_text(values), encoding="utf-8")
    text = METAINFO.read_text(encoding="utf-8")
    for tag, key in [
        ("name", "compass"),
        ("summary", "metainfo-summary"),
        ("p", "metainfo-description"),
    ]:
        text = replace_group(text, tag, values[key])
    METAINFO.write_text(text, encoding="utf-8")
    print("generated localized desktop and AppStream metadata")


if __name__ == "__main__":
    main()
