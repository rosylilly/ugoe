---
id: T0062
title: "web-check (pre-push と just ci) の前に、アプリの wasm の生成物が古いかを確かめ、古ければ作り直すか分かるエラーで止める"
status: done
milestone: M10
crates: [scripts, justfile, lefthook.yml, docs]
depends_on: []
assignee: claude-worker-T0062
---

# T0062: web-check (pre-push と just ci) の前に、アプリの wasm の生成物が古いかを確かめ、古ければ作り直すか分かるエラーで止める

## 目的

エディタ・ビューア・スタジオの wasm の生成物 (`apps/editor/src/lib/viewport/generated`・`apps/viewer/src/lib/wasm/generated`・
`apps/studio/src/lib/wasm/generated`) は git の管理外で、`just wasm <out>` で作る。ugoe-wasm の API が変わった後も作業ツリーに古い生成物が残るので、
pull や merge の後の pre-push (lefthook の web-check = 各パッケージの `svelte-check`) や `just ci` の `wasm-types` が、
コードの誤りでなく生成物の古さで落ちる (2026-10-06〜07 に何度も起きた。例: T1660 の `rotation_local_keyform` が無いと svelte-check が言う)。
利用者 (人) にも原因が分かりにくい。

## 受け入れ条件

- [x] 生成物が今の ugoe-wasm (と wasm-bindgen の版) から作られたものかを確かめる仕組みを足した。例: `just wasm` が生成物の横に入力 (crates/ugoe-wasm と依存するクレートのソース・Cargo.lock・wasm-bindgen の版) の指紋を書き、確かめる側が今の指紋と比べる。安く (ビルドせずに) 確かめられること
- [x] lefthook の pre-push の web-check と、`just ci` の web-check (または wasm-types) の前で確かめ、古ければ作り直す。作り直しが重すぎるなら「`just wasm …` で作り直す」と分かるエラーで止める。どちらにしたかと理由をメモに書く
- [x] 生成物が無い場合 (作ったことが無い) と、mbx のある機械・無い機械の両方で動く
- [x] 古い生成物を置いて (指紋を書き換えるなど) 確かめる側が見つけることを、スクリプトのテストで確かめた
- [x] docs/testing.md (と必要なら CONTRIBUTING・apps の README) に書いた
- [x] `just ci` が通る

## 参考

- justfile の `wasm`・`wasm-types`・`web-check`、lefthook.yml の pre-push、scripts/check-wasm-types.mjs

## メモ

- 利用者の依頼で起票した (2026-10-07)。
- やったこと (2026-10-07):
  - `scripts/wasm.mjs` の `build` (`just wasm`) が生成物の隣に `ugoe_wasm.fingerprint` (入力の SHA-256) を書く。入力は ugoe-wasm と
    dev 以外で依存するワークスペースのクレート (Cargo.toml の依存をたどる) の `Cargo.toml`・`build.rs`・`src/` 以下と、ルートの
    `Cargo.toml`・`Cargo.lock` (wasm-bindgen の版を含む)・`rust-toolchain.toml`。テスト・例・README は入れない。求めるのは 114 ファイルで約 5ms。
  - `check` (古ければ一覧と終了コード 1) と `fresh` (古いものだけを 1 回の cargo build で作り直す) を足し、`just wasm-fresh` を `web-check` の
    前提と lefthook の pre-push の web-check (`just wasm-fresh && pnpm ...`) の頭に入れた。指紋の無い生成物 (今ある手元の生成物) は古いとみなす。
    生成物の無いアプリは飛ばす。
  - **作り直す方にした。** 理由: 古いなら作り直さない限り svelte-check は通らず、利用者が次にすることは結局 `just wasm` なので。
    手元の計測で、作り直しは mbx の置き場に当たれば約 3〜6 秒 (cargo build の release + wasm-bindgen を 3 つ)、pre-push の clippy・全テストより軽い。
    古くなければ `just wasm-fresh` は 0.1 秒ほど。wasm-bindgen-cli が無い・版が違うときは、入れ方と、代わりに古い生成物を消す
    `rm -r` (型検査はスタブを読む) を出して止まる。
  - mbx: `fresh` は cargo の前に `exportMbxPath` を呼ぶので、pre-push のように PATH に shim が無いシェルから呼ばれても mbx を通る
    (just を通っても通らなくても同じ)。確かめた: mbx の shim が PATH に無い Bash から `pnpm exec lefthook run pre-push --job web-check` を、
    エディタの指紋を書き換え・ビューアに指紋の無い生成物を置いて流し、2 つが作り直されて svelte-check まで通った (18 秒)。
    `MBX_DISABLE=1 just wasm-fresh` (mbx の無い機械と同じ道) も作り直せた (1 回目は docs/workflow/build-cache.md の「戻した成果物は読み取り専用」の
    既知の失敗で serde_core が書けずに落ち、2 回目は通った。この判定の変更とは関係しない)。
  - テストは `scripts/wasm.test.mjs` (`just script-check`)。偽のワークスペースで、指紋の書き換え・指紋の無い生成物・ソースとシェーダと
    Cargo.toml と Cargo.lock と rust-toolchain.toml の変更を古いと見つけ、テスト・README・dev の依存だけのクレートの変更と生成物の無いアプリを飛ばす。
  - 文書: docs/testing.md に「wasm の生成物の古さ」、AGENTS.md の pre-push の行、apps/editor/README.md。
  - `just ci` が通った (rebase の後。web-check で `the generated ugoe-wasm bindings are up to date`)。
- やらなかったこと: CI (GitHub Actions) の web ジョブは毎回 `just wasm` で作るので変えていない。Windows の lefthook (Git Bash の sh で
  `just wasm-fresh && ...`) は確かめていない。
- 起票の候補: なし。
