#!/usr/bin/env python3
"""docs/tasks/ のタスクを一覧表示する。`--ready` で着手可能なタスクだけを表示する。"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent / "docs" / "tasks"
ORDER = ["in-progress", "review", "blocked", "todo", "done"]


def parse(path: pathlib.Path) -> dict:
    text = path.read_text(encoding="utf-8")
    m = re.match(r"^---\n(.*?)\n---\n", text, re.S)
    if not m:
        raise SystemExit(f"{path.name}: フロントマターがありません")
    meta = {}
    for line in m.group(1).splitlines():
        key, _, value = line.partition(":")
        value = value.strip()
        if value.startswith("[") and value.endswith("]"):
            meta[key.strip()] = [v.strip() for v in value[1:-1].split(",") if v.strip()]
        else:
            meta[key.strip()] = value
    meta["file"] = path.name
    return meta


def main() -> int:
    tasks = [parse(p) for p in sorted(ROOT.glob("T*.md"))]
    by_id = {t["id"]: t for t in tasks}
    errors = [
        f"{t['id']}: 未知の依存 {d}" for t in tasks for d in t["depends_on"] if d not in by_id
    ]
    for e in errors:
        print(f"error: {e}", file=sys.stderr)
    for t in tasks:
        t["ready"] = t["status"] == "todo" and all(
            by_id.get(d, {}).get("status") == "done" for d in t["depends_on"]
        )
    if "--ready" in sys.argv:
        tasks = [t for t in tasks if t["ready"]]
    tasks.sort(key=lambda t: (ORDER.index(t["status"]) if t["status"] in ORDER else 99, t["id"]))
    for t in tasks:
        status = "ready" if t["ready"] else t["status"]
        deps = ",".join(t["depends_on"]) or "-"
        who = f" @{t['assignee']}" if t.get("assignee") else ""
        print(f"{status:<12} {t['id']}  {t['milestone']:<3} {t['title']}  (deps: {deps}){who}")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
