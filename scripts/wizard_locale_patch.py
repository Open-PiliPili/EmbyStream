#!/usr/bin/env python3
"""
Scan wizard.rs for new user-facing string literals, assign wizard.patch.NNNN keys,
merge into locales/en.json + zh.json, replace literals with tr("key").

Requires: `use crate::i18n::tr` in wizard.rs.

Run from repo root: python3 scripts/wizard_locale_patch.py
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LOC = ROOT / "locales"
SRC_WIZ = ROOT / "src/cli_wizard/wizard.rs"

import importlib.util

_spec = importlib.util.spec_from_file_location("bi", ROOT / "scripts" / "build_i18n.py")
_bi = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_bi)
CLI: dict[str, tuple[str, str]] = _bi.CLI
strip_zh_period = _bi.strip_zh_period


def parse_rust_string(src: str, i: int) -> tuple[str, int]:
    if i >= len(src) or src[i] != '"':
        raise ValueError
    i += 1
    out: list[str] = []
    while i < len(src):
        c = src[i]
        if c == '"':
            return "".join(out), i + 1
        if c == "\\":
            i += 1
            if i >= len(src):
                break
            esc = src[i]
            if esc == "n":
                out.append("\n")
            elif esc in "\\\"'":
                out.append(esc)
            elif esc == "\n":
                i += 1
                while i < len(src) and src[i] in " \t\n\r":
                    i += 1
                continue
            else:
                out.append(esc)
            i += 1
            continue
        out.append(c)
        i += 1
    raise ValueError("unterminated string")


SKIP_LITERALS = frozenset(
    {
        "",
        "{}",
        "{:?}",
        " ",
        "%Y%m%d_%H%M%S",
        "./logs",
        "info",
        "middle",
        "allow",
        "deny",
        "60001",
        "8096",
        "443",
        "path_join",
        "path",
        "stream",
        "key",
        "127.0.0.1",
        "localhost",
        "template.toml",
        "invalid file name",
    }
)


def has_binary_or_ansi(s: str) -> bool:
    return any(ord(c) < 32 and c not in "\n\t\r" for c in s)


def is_match_arm_pattern(src: str, end_quote: int) -> bool:
    j = end_quote
    while j < len(src) and src[j] in " \t\n\r":
        j += 1
    return src.startswith("=>", j)


def skip_literal(s: str) -> bool:
    if re.fullmatch(r"w\.\d{4}", s):
        return True
    if re.match(r"^wizard\.[a-z0-9_.]+$", s):
        return True
    if s in SKIP_LITERALS:
        return True
    if not s:
        return True
    if len(s) <= 1 and s not in ("$", "?"):
        return True
    if re.fullmatch(r"%[YmdHMS_]+", s):
        return True
    if s.startswith("./") and len(s) < 32:
        return True
    if re.fullmatch(r"[\d.]+", s):
        return True
    if s in ("Yes", "No"):
        return True
    if "{" in s and "}" in s:
        return True
    if has_binary_or_ansi(s):
        return True
    return False


def iter_replace_string_literals(text: str, en_to_key: dict[str, str]) -> str:
    out: list[str] = []
    i = 0
    n = len(text)
    while i < n:
        if i + 1 < n and text[i : i + 2] == "//":
            j = text.find("\n", i)
            if j < 0:
                out.append(text[i:])
                break
            out.append(text[i : j + 1])
            i = j + 1
            continue
        if text[i] == '"':
            if i > 0 and text[i - 1] == "b":
                try:
                    s, end = parse_rust_string(text, i)
                except ValueError:
                    out.append(text[i])
                    i += 1
                    continue
                out.append(text[i:end])
                i = end
                continue
            try:
                s, end = parse_rust_string(text, i)
            except ValueError:
                out.append(text[i])
                i += 1
                continue
            if (
                not skip_literal(s)
                and s in en_to_key
                and not is_match_arm_pattern(text, end)
            ):
                k = en_to_key[s]
                out.append(f'tr("{k}")')
                i = end
                continue
            out.append(text[i:end])
            i = end
            continue
        out.append(text[i])
        i += 1
    return "".join(out)


def reverse_wizard_en_to_key(en_map: dict[str, str]) -> dict[str, str]:
    rev: dict[str, str] = {}
    for k, v in sorted(en_map.items()):
        if not k.startswith("wizard."):
            continue
        rev.setdefault(v, k)
    return rev


def next_patch_index(en_map: dict[str, str]) -> int:
    m = 0
    for k in en_map:
        if k.startswith("wizard.patch."):
            try:
                m = max(m, int(k.rsplit(".", 1)[-1]) + 1)
            except ValueError:
                pass
    return m


def main() -> None:
    en_path = LOC / "en.json"
    zh_path = LOC / "zh.json"
    en_map = json.loads(en_path.read_text(encoding="utf-8"))
    zh_map = json.loads(zh_path.read_text(encoding="utf-8"))
    for k, (en, zh) in CLI.items():
        en_map[k] = en
        zh_map[k] = strip_zh_period(zh)

    en_to_key = reverse_wizard_en_to_key(en_map)
    patch_i = next_patch_index(en_map)

    def assign_new(en: str) -> str:
        nonlocal patch_i
        if en in en_to_key:
            return en_to_key[en]
        nk = f"wizard.patch.{patch_i:04d}"
        patch_i += 1
        en_map[nk] = en
        zh_map[nk] = en
        en_to_key[en] = nk
        return nk

    t = SRC_WIZ.read_text(encoding="utf-8")
    i = 0
    while i < len(t):
        if i + 1 < len(t) and t[i : i + 2] == "//":
            i = t.find("\n", i)
            if i < 0:
                break
            i += 1
            continue
        if t[i] == '"':
            if i > 0 and t[i - 1] == "b":
                try:
                    _, end = parse_rust_string(t, i)
                except ValueError:
                    i += 1
                    continue
                i = end
                continue
            try:
                s, end = parse_rust_string(t, i)
            except ValueError:
                i += 1
                continue
            if (
                not skip_literal(s)
                and s not in en_to_key
                and not is_match_arm_pattern(t, end)
            ):
                assign_new(s)
            i = end
            continue
        i += 1

    en_map = dict(sorted(en_map.items()))
    zh_map = dict(sorted(zh_map.items()))
    LOC.mkdir(exist_ok=True)
    en_path.write_text(
        json.dumps(en_map, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    zh_path.write_text(
        json.dumps(zh_map, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    rev = reverse_wizard_en_to_key(en_map)
    (LOC / "_reverse_en_to_key.json").write_text(
        json.dumps(rev, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    (LOC / "_en_to_key.json").write_text(
        json.dumps(rev, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )

    raw = SRC_WIZ.read_text(encoding="utf-8")
    SRC_WIZ.write_text(iter_replace_string_literals(raw, en_to_key), encoding="utf-8")

    print("en.json keys", len(en_map), file=sys.stderr)


if __name__ == "__main__":
    main()
