#!/usr/bin/env python3
"""Refresh CLI / shared keys in locales/en.json and locales/zh.json.

Wizard copy lives under `wizard.*` semantic keys (see scripts/wizard_w_to_semantic_map.json
and scripts/apply_wizard_semantic_migration.py). This script only overlays the fixed CLI table
and regenerates English→key reverse maps for patch tooling.
"""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LOC = ROOT / "locales"


def strip_zh_period(s: str) -> str:
    s = s.rstrip()
    return s[:-1].rstrip() if s.endswith("。") else s


def parse_rust_string(src: str, i: int) -> tuple[str, int]:
    if i >= len(src) or src[i] != '"':
        raise ValueError("expected '\"'")
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
            elif esc == "r":
                out.append("\r")
            elif esc == "t":
                out.append("\t")
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


def extract_wz_calls(text: str) -> list[tuple[str, str, int, int]]:
    """Legacy `wz(en, zh)` extractor for scripts/replace_wz_with_tr.py."""
    results: list[tuple[str, str, int, int]] = []
    i = 0
    while True:
        j = text.find("wz(", i)
        if j < 0:
            break
        start = j
        pos = j + 3
        while pos < len(text) and text[pos] in " \t\n\r":
            pos += 1
        try:
            en, pos = parse_rust_string(text, pos)
        except ValueError:
            i = j + 3
            continue
        while pos < len(text) and text[pos] in " \t\n\r,":
            pos += 1
        try:
            zh, pos = parse_rust_string(text, pos)
        except ValueError:
            i = j + 3
            continue
        while pos < len(text) and text[pos] in " \t\n\r,":
            pos += 1
        if pos >= len(text) or text[pos] != ")":
            i = j + 3
            continue
        end = pos + 1
        results.append((en, zh, start, end))
        i = end
    return results


# Same keys in en + zh; runtime lookup falls back to en if zh missing.
CLI: dict[str, tuple[str, str]] = {
    "cli.about": (
        "Emby streaming proxy: run frontend, backend, or dual gateways from TOML config.",
        "Emby 流媒体代理：按 TOML 配置运行前端、后端或双网关",
    ),
    "cli.arg.lang": (
        "UI language: en (default) or zh (Simplified Chinese); affects config wizard and --help.",
        "界面语言：en（默认，英文）或 zh（简体中文）；影响配置向导与 --help",
    ),
    "cli.sub.help.about": (
        "Print this message or the help of the given subcommand(s)",
        "打印帮助信息（也可在各子命令后使用）",
    ),
    "cli.run.about": (
        "Start HTTP gateways (default when no subcommand: use `run` explicitly).",
        "启动 HTTP(S) 网关（须显式使用 run 子命令；无子命令时进程会直接退出）",
    ),
    "cli.run.arg.config": ("Path to config.toml.", "配置文件路径（config.toml）"),
    "cli.run.arg.ssl_cert_file": (
        "Override TLS cert path (PEM) from config.",
        "覆盖配置中的 TLS 证书路径（PEM）",
    ),
    "cli.run.arg.ssl_key_file": (
        "Override TLS private key path (PEM) from config.",
        "覆盖配置中的 TLS 私钥路径（PEM）",
    ),
    "cli.config.about": (
        "Interactive TOML configuration wizard (prompt language follows `--lang`).",
        "交互式编辑/生成 TOML 配置（提示语随 --lang 切换）",
    ),
    "cli.config.show.about": (
        "List valid TOML configs here and print one (mask secrets unless you confirm).",
        "列出当前目录下合法配置并查看其一（默认遮蔽密钥）",
    ),
    "cli.config.template.about": (
        "Interactive: pick stream_mode and write a starter TOML (via temp file, then atomically).",
        "交互生成入门 TOML（先写临时文件再原子替换）",
    ),
    "error.wizard_prefix": ("config wizard", "配置向导"),
    "wizard.confirm.permanent_delete": (
        "Permanently delete {path}?",
        "永久删除 {path}？",
    ),
    "wizard.yes": ("Yes", "是"),
    "wizard.no": ("No", "否"),
    "wizard.label.default_prefix": ("Default: ", "默认："),
    "wizard.label.example_prefix": ("Example: ", "示例："),
    "wizard.display.empty": ("(empty)", "（空）"),
    "wizard.display.auto_generated": ("(auto-generated)", "（已自动生成）"),
    "wizard.display.secret_masked": ("· · ·", "· · ·"),
    "wizard.error.dual_port": (
        "Dual mode: frontend listen_port {port} cannot equal backend listen_port. Change one of them.",
        "双模式：前端 listen_port {port} 不能与后端 listen_port 相同，请修改其一",
    ),
    "wizard.prompt.enable_anti": (
        "Enable {ctx}? (reject requests when Host ≠ trusted host)",
        "启用 {ctx}？（Host 与信任主机不一致时拒绝）",
    ),
    "regex.error.invalid": (
        "Invalid regex: {detail}. Try again or fix the pattern.",
        "正则无效：{detail}；请重试或修改表达式",
    ),
    "regex.output.matches": ("matches: {value}", "匹配：{value}"),
    "regex.output.group": ("group {idx}: {value}", "捕获组 {idx}：{value}"),
    "wizard.tip.field_input": (
        "Tip: Press Enter to keep the default from the field line (Default: …), or type a new value and press Enter.",
        "提示：直接按 Enter 采用上一行中的默认值；或输入新值后按 Enter",
    ),
    "wizard.tip.select_file_list": (
        "Tip: ↑ / ↓ and Enter to choose · Esc or q to go back",
        "提示：↑ / ↓ 选择，Enter 确认 · Esc 或 q 返回",
    ),
    "wizard.banner.title": (
        "EmbyStream configuration",
        "EmbyStream 配置向导",
    ),
    "wizard.banner.subtitle": (
        "Interactive TOML wizard — prompt language follows `--lang`.",
        "交互式 TOML 配置向导，提示语随 --lang 切换",
    ),
}


def reverse_wizard_en_to_key(en_map: dict[str, str]) -> dict[str, str]:
    """First occurrence wins (stable key order by sorting keys)."""
    rev: dict[str, str] = {}
    for k, v in sorted(en_map.items()):
        if not k.startswith("wizard."):
            continue
        rev.setdefault(v, k)
    return rev


def main() -> None:
    en_path = LOC / "en.json"
    zh_path = LOC / "zh.json"
    en_map = json.loads(en_path.read_text(encoding="utf-8"))
    zh_map = json.loads(zh_path.read_text(encoding="utf-8"))
    for k, (en, zh) in CLI.items():
        en_map[k] = en
        zh_map[k] = strip_zh_period(zh)
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
    print("CLI keys", len(CLI), "wizard reverse entries", len(rev), "total keys", len(en_map))


if __name__ == "__main__":
    main()
