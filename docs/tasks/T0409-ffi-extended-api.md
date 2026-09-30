---
id: T0409
title: C ABI: 着せ替え・ライティング・当たり判定・トラッキング・読み込みの警告を足す
status: in-progress
milestone: M7
crates: [ugoe-ffi, docs]
depends_on: [T0401]
assignee: claude-worker-T0409
---

# T0409: C ABI の拡張

## 目的

T0401 の後続の候補。C ABI は読み込み・パラメータ・update・描画可能要素・テクスチャ・モーションと表情までで、
Unity などのホストが使いたい機能 (着せ替え・ライティング・当たり判定・トラッキング・読み込みの警告) がまだない。

## 受け入れ条件

- [ ] 着せ替え: `.ugoemod` の読み込み、付け外し、スロットの一覧、失敗の構造化されたエラー (wasm の `DressUpError` と同じ code)
- [ ] ライティング: 法線マップのバイト列の借用、光源の設定 (ランタイムの `Lighting` に合わせる)
- [ ] 当たり判定 (`hit_test` と標準の当たり判定)、トラッキングの入力、読み込みの警告 (wasm の `load_warnings_json` と同じ code)、ID によるパラメータの一括設定、表情の解除
- [ ] ABI を壊す変更なら `ugoe_abi_version()` を上げる。ヘッダを作り直し、C の見本を広げる
- [ ] `just ci`・`just wasm-check`・`just ffi-check` が通る。Windows と macOS での C の見本のビルドは、できなければ利用者の確認に残す

## 参考

- `docs/tasks/T0401-ffi.md` のメモ、`crates/ugoe-ffi/README.md`、`docs/portability.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
