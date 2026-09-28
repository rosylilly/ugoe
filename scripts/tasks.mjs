#!/usr/bin/env node
// docs/tasks/ のタスクを一覧表示する。`--ready` で着手可能なタスクだけを表示する。

import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "docs", "tasks");
const ORDER = ["in-progress", "review", "blocked", "todo", "done"];

function parse(file) {
  const text = readFileSync(join(ROOT, file), "utf8");
  const m = text.match(/^---\n([\s\S]*?)\n---\n/);
  if (!m) {
    console.error(`${file}: フロントマターがありません`);
    process.exit(1);
  }
  const meta = { file };
  for (const line of m[1].split("\n")) {
    const i = line.indexOf(":");
    const key = (i < 0 ? line : line.slice(0, i)).trim();
    const value = i < 0 ? "" : line.slice(i + 1).trim();
    meta[key] =
      value.startsWith("[") && value.endsWith("]")
        ? value
            .slice(1, -1)
            .split(",")
            .map((v) => v.trim())
            .filter(Boolean)
        : value;
  }
  return meta;
}

let tasks = readdirSync(ROOT)
  .filter((f) => /^T.*\.md$/.test(f))
  .sort()
  .map(parse);
const byId = new Map(tasks.map((t) => [t.id, t]));

const errors = tasks.flatMap((t) =>
  t.depends_on.filter((d) => !byId.has(d)).map((d) => `${t.id}: 未知の依存 ${d}`),
);
for (const e of errors) console.error(`error: ${e}`);

for (const t of tasks) {
  t.ready = t.status === "todo" && t.depends_on.every((d) => byId.get(d)?.status === "done");
}
if (process.argv.includes("--ready")) tasks = tasks.filter((t) => t.ready);

const rank = (s) => (ORDER.includes(s) ? ORDER.indexOf(s) : 99);
tasks.sort((a, b) => rank(a.status) - rank(b.status) || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
for (const t of tasks) {
  const status = t.ready ? "ready" : t.status;
  const deps = t.depends_on.join(",") || "-";
  const who = t.assignee ? ` @${t.assignee}` : "";
  console.log(
    `${status.padEnd(12)} ${t.id}  ${t.milestone.padEnd(3)} ${t.title}  (deps: ${deps})${who}`,
  );
}
process.exitCode = errors.length > 0 ? 1 : 0;
