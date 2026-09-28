---
id: T0616
title: M2 の受け入れテスト (PSD から書き出しまでの通し)
status: in-progress
milestone: M2
crates: [ugoe-editor-core (tests/ のみ), ugoe-cli (tests/ のみ)]
depends_on: [T0605, T0606, T0607, T0608, T0611, T0612, T0613, T0614]
assignee: claude-worker-T0616
---

# T0616: M2 の受け入れテスト (PSD から書き出しまでの通し)

## 目的

ロードマップの M2 の完了条件を、GUI を使わずにコマンドだけで通して確かめる自動テストを作る。

> PSD を読み込み、自動メッシュを作り、デフォーマとパラメータを手で設定して、`.ugoeproj` に保存し、`.ugoe` として書き出せる。
> 端の形だけを作れば、中間・斜め・左右反転のキーフォームが推定される。

## 受け入れ条件

- [ ] `ugoe-editor-core` の結合テストが、エディタと同じ `Command` だけを使って次を順に行う: PSD (`assets/fixtures/layers.psd`) の取り込み → ワープデフォーマと回転デフォーマの作成 → パラメータ `AngleX` / `AngleY` の作成 → キーの追加 → 端のセル (1 軸セル) の編集 → 加算合成による斜めのセルの推定 → 左右の対のミラー → 深度からの回転 → `.ugoeproj` への保存と読み込み → `.ugoe` の書き出し
- [ ] 各段階で、`validate` が通ることと、推定したセルが仕様 (`docs/spec/keyform-inference.md`) の式どおりであることを確かめる
- [ ] 書き出した `.ugoe` をランタイム (`ugoe::Puppet` または `ModelInstance`) で読み込み、中間・斜めのパラメータ値で評価できる
- [ ] GPU があるとき (`gpu-tests`) は `ugoe render` で斜めの姿勢を PNG に描き、期待する画素を確かめる
- [ ] 全体を 1 回の Undo で戻せない操作が無いこと (各段階の Undo / Redo) を確かめる
- [ ] `just ci` が通る

## 参考

- `docs/roadmap.md#m2-エディタ-mvp`
- `docs/editor.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
