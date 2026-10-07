---
id: T0074
title: "fuzz/Cargo.lock の依存の版を、ルートの Cargo.lock とそろえるかを決め、そろえるならずれを検査する"
status: done
milestone: M11
crates: [fuzz, scripts, docs]
depends_on: [T0073]
assignee: claude-worker-T0074
---

# T0074: fuzz/Cargo.lock の依存の版を、ルートの Cargo.lock とそろえるかを決め、そろえるならずれを検査する

## 目的

ワークスペースの外の `fuzz/Cargo.lock` の版が、ルートの `Cargo.lock` と違うものがある (例: glam 0.33.12 と 0.33.11。T0072 の候補)。
fuzz が回すコードと出荷するコードで依存の版が違うと、fuzz が見つけた (あるいは見つけない) 不具合が出荷物と食い違うおそれがある。
`bindings/godot/Cargo.lock` も同じ形。

## 受け入れ条件

- [x] そろえるか (ルートの版に合わせる・ずれを検査で止める) を、作業者が決めて理由をメモと docs/testing.md に書いた
- [x] そろえるなら、共通の依存の版がルートと同じになるよう lock を直し、ずれを見つける検査 (`just groups` か `just fuzz-lint` など) を足した。検査がずれを見つけることを一時的な変更で確かめた
- [x] `just fuzz-lint` と `just ci` が通る

## 参考

- T0072 のメモ、fuzz/Cargo.lock、bindings/godot/Cargo.lock、scripts/check-licenses.mjs

## メモ

- T0072 のワーカーの報告から起票した (2026-10-07)。check-licenses.mjs を触る T0073 の後にした。

### 作業の記録 (claude-worker-T0074)

- **決定: そろえる。** 外の lock (`fuzz/Cargo.lock`・`bindings/godot/Cargo.lock`) で、ルートの `Cargo.lock` にもある依存
  (semver で互換な同じ系列) はルートと同じ版にする。理由: fuzz が回すコード・Godot に出すコードと、出荷するコード
  (ルートでビルドするアプリ・C ABI・wasm) で依存の版が違うと、fuzz の結果が出荷物と食い違うおそれがある。外の lock は
  ルートで依存を上げても自動では追従せず、足した依存は `cargo update --workspace` が最新の版で入れるので、検査が無いとずれ続ける。
  合わせる手間は `cargo update -p … --precise …` だけで小さい。外のクレートだけの依存 (libfuzzer-sys・godot-rust) と、
  ルートと互換でない系列 (godot-rust の glam 0.32) は比べない。docs/testing.md#fuzz に書いた。
- **lock を直した:** `fuzz/Cargo.lock` の cc 1.6.0→1.5.1・glam 0.33.12→0.33.11・libc 0.2.190→0.2.189・twox-hash 2.1.5→2.1.4
  (`cargo update --manifest-path fuzz/Cargo.toml -p <名前> --precise <ルートの版>`)。`bindings/godot/Cargo.lock` はずれが無かった
  (glam 0.32.1 は godot-rust の別の系列) ので変えていない。godot の lock を変えていないので `just godot-check` は流していない。
- **検査:** `scripts/lib/lock-sync.mjs` (lock を読んで比べる。cargo を使わない) と `scripts/check-lock-sync.mjs` (CLI。ずれた依存と
  ルートの版に合わせる `cargo update … -p <名前>@<版> --precise <ルートの版>` を出して終了コード 1)。`just groups` (`just ci` と CI の
  `rust-checks`。ルートで依存を上げたときも気づく) と `just fuzz-lint`・`fuzz-check` (`fuzz/run.mjs`。fuzz だけ) が流す。
  単体テストは `scripts/lock-sync.test.mjs` (`just groups` が流す)。
- **ずれを見つけることの確認 (一時的な変更は戻した。コミットしていない):** (1) 直す前の `fuzz/Cargo.lock` に戻すと 4 つのずれと
  直し方を出して終了コード 1。(2) `bindings/godot/Cargo.lock` の glam 0.33.11 を 0.33.99 に書き換えると godot のずれを出した。
  (3) ルートの `Cargo.lock` の glam を 0.33.98 に書き換える (ルートで上げた場合) と、fuzz・godot の両方のずれを出した。
- **範囲の外の小さな変更:** `justfile` (`groups` に検査とテストを足した)、`bindings/godot/README.md` (版をそろえる旨を 1 項目)。
- `just fuzz-lint` と `just ci` は通った (main は着手から進んでおらず、rebase は不要だった)。

### 起票の候補

- 無し。
