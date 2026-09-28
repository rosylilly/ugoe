---
id: T0109
title: 前方互換: 機能の宣言と未知の値の読み込み
status: in-progress
milestone: M2
crates: [ugoe-model, ugoe-format, ugoe-runtime, ugoe-render]
depends_on: []
assignee: claude-worker-T0109
---

# T0109: 前方互換: 機能の宣言と未知の値の読み込み

## 目的

ADR 0013 の仕組み (機能 ID の宣言、`FEAT` チャンク、拡張可能な enum の `Unknown`、フォールバック) を実装する。以降の機能追加はすべてこの仕組みに乗るので、早めに入れる。

## 受け入れ条件

- [ ] 機能 ID の一覧とフォールバックを定数として持ち、`docs/spec/file-formats.md` の表と一致させる (テストで検査する)
- [ ] `FEAT` チャンクの読み書き。知らない必須機能は `UnsupportedFeatures` エラー、知らない任意機能は警告の一覧を返す
- [ ] 合成モード・デフォーマの種類・マスクの結合・カーブの対象など、拡張されうる enum に `Unknown` を追加し、未知のタグを読んでもエラーにならない
- [ ] ランタイムとレンダラが `Unknown` にフォールバック (恒等変換・通常の合成・和集合など) を適用する
- [ ] 書き出し時に、使っている機能を自動で宣言する (1.0 の基準の機能は宣言しない。ADR 0018)
- [ ] 1.0 以降の機能を想定した試験用の機能 ID で、任意機能のフォールバックと必須機能の拒否を確かめる
- [ ] 未知のタグ・未知の機能を含むバンドルを読むテストがある
- [ ] `just ci` が通る

## 参考

- `docs/adr/0013-forward-compatibility.md`
- `docs/spec/file-formats.md#機能の宣言と段階的な劣化`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
