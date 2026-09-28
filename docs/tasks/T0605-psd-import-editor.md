---
id: T0605
title: エディタ: PSD の取り込みと自動メッシュ
status: in-progress
milestone: M2
crates: [ugoe-editor-core, apps/editor]
depends_on: [T0501, T0502, T0601, T0604]
assignee: claude-worker-T0605
---

# T0605: エディタ: PSD の取り込みと自動メッシュ

## 目的

PSD を読み込んで、レイヤーの構造からパーツ・テクスチャ・メッシュ (自動メッシュ) を作る。M2 の完了条件の入口になる。

## 受け入れ条件

- [ ] `Command::ImportPsd` (または取り込み結果を表す `Batch`) が、グループをパーツに、ラスタレイヤーをテクスチャとメッシュにする。レイヤーの位置と描画順 (上のレイヤーほど手前) を保つ
- [ ] メッシュは `ugoe-mesher` で自動生成する。メッシュの再生成 (`RegenerateMesh { mesh, options }`) もコマンドとしてある
- [ ] テクスチャの画像はプロジェクト (`textures/`) に保存され、保存・読み込みで失われない
- [ ] エディタのメニューから PSD を選んで取り込め、ビューポートに表示される
- [ ] テスト用の小さな PSD (`assets/fixtures/`) を取り込むテストがある
- [ ] `just ci` が通る

## 参考

- `docs/ai-integration.md#psd-取り込み`、`docs/ai-integration.md#自動メッシュ`
- `docs/spec/file-formats.md#プロジェクト-ugoeproj`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
