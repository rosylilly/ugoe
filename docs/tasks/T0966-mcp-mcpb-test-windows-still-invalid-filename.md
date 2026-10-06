---
id: T0966
title: "mcp: T0965 の後も Windows の CI で .mcpb の中継のテストが InvalidFilename で落ちる (wine で再現して直す)"
status: in-progress
milestone: M9
crates: [ugoe-mcp, docs]
depends_on: []
assignee: claude-worker-T0966
---

# T0966: mcp: T0965 の後も Windows の CI で .mcpb の中継のテストが InvalidFilename で落ちる (wine で再現して直す)

## 目的

T0965 で `${__dirname}` に `\\?\` を外したパスを入れるようにしたが、それを含む main (67b0d91d) の CI
(https://github.com/rosylilly/ugoe/actions/runs/37452217684/job/112231214104 、`rust (windows)` の Test) でも同じく落ちた。

```
thread 'the_mcp_bundle_relays_to_a_running_editor' panicked at crates\ugoe-mcp\tests\editor.rs:266:14:
called `Result::unwrap()` on an `Err` value: Os { code: 123, kind: InvalidFilename, ... }
```

T0965 の見立て (コマンドのパスの verbatim と `/` の混在) だけでは足りない。ほかの候補: `current_dir(&tmp.path)` (verbatim のまま)、
`DISCOVERY_ENV` の値、展開した `server/ugoe-mcp.exe` の書き出し方 (`/` を含む名前で `dir.join`)、std の `Command` の Windows でのプログラムの解決。
推測で直さず、Windows で再現してから直す。

## 受け入れ条件

- [ ] Windows で再現した (手元の `wine` と `x86_64-pc-windows-gnu` のターゲットでテストを動かす。動かせなければ、失敗したときにコマンド・作業ディレクトリ・環境変数を出すようにして、CI のログで原因を絞る)
- [ ] 原因をメモに書き、直した。manifest (配布物) の誤りなら manifest を直す
- [ ] 再現した環境で、直した後にテストが通ることを確かめた
- [ ] `just cross-check` と `just ci` が通る

## 参考

- T0965 のメモ、crates/ugoe-mcp/tests/editor.rs (`desktop_dirname`・`expand_dirname`・`Relay::spawn_command`)、crates/ugoe-mcp/src/mcpb.rs

## メモ

- CI の失敗から起票した (2026-10-06)。
