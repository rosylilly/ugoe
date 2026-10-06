---
id: T0059
title: "e2e: 3 つのアプリを 1 回の cargo でビルドし、e2e の CI は rust (apps) のキャッシュを読むだけにする"
status: in-progress
milestone: M10
crates: [tests/e2e, .github/workflows, docs]
depends_on: []
assignee: claude-worker-T0059
---

# T0059: e2e: 3 つのアプリを 1 回の cargo でビルドし、e2e の CI は rust (apps) のキャッシュを読むだけにする

## 目的

T0057 の調べ (メモの C) で、e2e のビルドを 3 つのアプリをまとめた 1 回の cargo
(`cargo build -p ugoe-editor -p ugoe-viewer -p ugoe-studio -p ugoe-studio-vcam --features tauri/custom-protocol`) にすれば、
rust (apps) の後のコンパイルは依存 17 個 (手元で約 31 秒) で済むことが分かった。そうすれば e2e は自分のキャッシュを持たずに
`group-apps` と `conformance` のキャッシュを読むだけにでき、CI のキャッシュの 1 組が約 1.1GB 減る (約 4.6GB)。

## 受け入れ条件

- [ ] `tests/e2e/run.mjs` が、各アプリのフロントエンドを作って (`pnpm build`、要るものは `pnpm wasm`) から、3 つのアプリを 1 回の cargo でビルドする
- [ ] できた実行ファイルが `tauri build` と同じ振る舞いになることを確かめ、確かめ方をメモに書いた (frontendDist の埋め込み・tauri.conf.json の読み込み・
      `--ci` の扱い。違いがあれば直したか、理由を書いた)
- [ ] `just e2e` の全筋書き (3 つのアプリ) と `just e2e --manual` が通る
- [ ] e2e.yml が e2e の専用のキャッシュを保存せず、rust (apps) (`group-apps`) と `conformance` のキャッシュを読むだけになった。
      docs/testing.md の「CI のキャッシュ」と tests/e2e/README.md を今の形に直した
- [ ] actionlint で増えた指摘が無い
- [ ] 1 組の実際の合計は push の後に `gh api repos/{owner}/{repo}/actions/cache/usage` と `gh cache list` で測る。測れなければ「CI で確かめること」としてメモに書く
- [ ] `just ci` が通る

## 参考

- T0057 のメモ (調べたこと C・起票の候補)、T0051 のメモ (見積もり)
- tests/e2e/run.mjs・tests/e2e/README.md、.github/workflows/e2e.yml・ci.yml、docs/testing.md の「CI のキャッシュ」

## メモ

- T0058 で起票した (元の候補: T0057 (e2e のビルドを 1 回の cargo にする))。
