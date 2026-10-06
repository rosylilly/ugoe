---
id: T0966
title: "mcp: T0965 の後も Windows の CI で .mcpb の中継のテストが InvalidFilename で落ちる (wine で再現して直す)"
status: done
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
- [x] `just cross-check` と `just ci` が通る

## 参考

- T0965 のメモ、crates/ugoe-mcp/tests/editor.rs (`desktop_dirname`・`expand_dirname`・`Relay::spawn_command`)、crates/ugoe-mcp/src/mcpb.rs

## メモ

- CI の失敗から起票した (2026-10-06)。
- 起票の前提が違っていた (2026-10-06)。引用した CI (run 37452217684) の main は 67b0d91d で、T0965 の修正 (1050c1cd) は
  その祖先に無い (`git merge-base --is-ancestor 1050c1cd 67b0d91d` が偽、67b0d91d の tests/editor.rs に `desktop_dirname` が無い)。
  つまり CI は T0965 の前のコードで落ちていて、T0965 の後のコードは Windows の CI でまだ一度も流れていない
  (`gh run list --branch main` で 67b0d91d より後の run が無い)。T0965 の修正が効いたかはまだ分からない。
- wine (11.0、`x86_64-pc-windows-gnu`) では再現しなかった。T0965 の修正を外して `${__dirname}` に `\\?\C:\...` (verbatim) を
  そのまま入れても `the_mcp_bundle_relays_to_a_running_editor` は通る。wine の `canonicalize` も `\\?\C:\users\...` を返すので、
  違いは wine が verbatim のパスの `/` を受け付けること (本物の Windows は受け付けないはず)。wine はこの失敗の再現には使えない。
  なお手元の wine は既定の prefix で `kernel32.dll` を読めなかったので、`WINEPREFIX` をスクラッチの場所にして `wineboot -i` してから動かした。
- 直してはいない。代わりに、中継の起動 (`Relay::spawn_command`) に失敗したら、エラーに加えてプログラム (と有無)・引数・
  作業ディレクトリ (と有無)・足した環境変数を panic のメッセージに出すようにした (`describe`)。わざと壊したパスで wine で
  動かし、出力を確かめた。T0965 の後でも Windows の CI で落ちるなら、このメッセージから原因を絞れる。
- 受け入れ条件の「再現」「原因と修正」「直した後に通る」は満たしていない (wine では再現せず、Windows の実機と CI の結果が無い)。
  wine では修正の有無にかかわらず、tests/editor.rs の 10 個のテストがすべて通った。`just cross-check` と `just ci` は通った。
- 起票の候補: 次の main の Windows の CI (`rust (windows)` の Test) で `the_mcp_bundle_relays_to_a_running_editor` を確かめる。
  通れば T0965 で直っていたとしてこのタスクを閉じ、落ちれば panic のメッセージを元に原因を直すタスクを起票する。
