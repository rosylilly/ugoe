#!/usr/bin/env python3
"""クレート間の依存方向を検査する。ルールの正は docs/architecture.md#レイヤーと依存ルール。

新しいクレートを追加したら ALLOWED にエントリを追加すること (未登録クレートはエラー)。
"""
import json
import subprocess
import sys

CORE = {"ugoe-core"}
DATA = CORE | {"ugoe-model"}
EVAL = DATA | {"ugoe-runtime"}
RUNTIME_TIER = EVAL | {"ugoe-format", "ugoe-physics", "ugoe-motion", "ugoe-render", "ugoe"}
TOOL_BASE = RUNTIME_TIER | {"ugoe-psd", "ugoe-mesher", "ugoe-editor-core"}

# クレート -> 依存してよい内部クレート
ALLOWED = {
    # --- ランタイム層: ツール層 (PSD・メッシャ・エディタ・AI) に依存してはならない ---
    "ugoe-core": set(),
    "ugoe-model": CORE,
    "ugoe-runtime": DATA,
    "ugoe-format": DATA,
    "ugoe-physics": EVAL,
    "ugoe-motion": EVAL,
    "ugoe-render": EVAL,
    "ugoe": RUNTIME_TIER - {"ugoe"},
    "ugoe-ffi": RUNTIME_TIER,
    "ugoe-wasm": RUNTIME_TIER,
    # --- ツール層 ---
    "ugoe-psd": CORE,
    "ugoe-mesher": CORE,
    "ugoe-editor-core": RUNTIME_TIER | {"ugoe-psd", "ugoe-mesher"},
    "ugoe-ai": TOOL_BASE,
    "ugoe-cli": TOOL_BASE | {"ugoe-ai"},
    "ugoe-editor": TOOL_BASE | {"ugoe-ai"},
}


def main() -> int:
    meta = json.loads(
        subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1"])
    )
    members = {p["name"] for p in meta["packages"]}
    errors = []
    for pkg in meta["packages"]:
        name = pkg["name"]
        if name not in ALLOWED:
            errors.append(f"{name}: scripts/check-layers.py の ALLOWED に未登録")
            continue
        for dep in pkg["dependencies"]:
            if dep["name"] not in members or dep.get("kind") == "dev":
                continue
            if dep["name"] not in ALLOWED[name]:
                errors.append(f"{name} -> {dep['name']}: 許可されていない依存方向")
    for e in errors:
        print(f"error: {e}", file=sys.stderr)
    if not errors:
        print(f"layers ok ({len(members)} crates)")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
