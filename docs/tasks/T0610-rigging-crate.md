---
id: T0610
title: ugoe-rigging クレートと加算合成
status: done
milestone: M2
crates: [ugoe-rigging (新規), scripts/check-layers.mjs, commitlint.config.mjs, docs/architecture.md]
depends_on: [T0108]
assignee: claude-worker-T0610
---

# T0610: ugoe-rigging クレートと加算合成

## 目的

ツール層のクレート `ugoe-rigging` を作り、キーフォームの加算合成 (交互作用の項を含む) を実装する。

## 受け入れ条件

- [x] クレートを追加し、依存方向の表 (`scripts/check-layers.mjs`) とコミットの scope (`rigging`) に登録する
- [x] `docs/spec/keyform-inference.md#加算合成` のすべての式にテストがある (1 軸の補間・外挿、交互作用の項の伝播、クランプ)
- [x] `authored` のセルを上書きしない
- [x] 中立のキーがない軸はエラーになる
- [x] `just ci` が通る

## 参考

- `docs/adr/0010-keyform-inference.md`
- `docs/spec/keyform-inference.md`

## メモ

- API: `infer_additive(&Model, &mut KeyformGrid<T>)` と、要素の ID から格子を引く `infer_additive_target(&mut Model, &KeyformTarget)`。どちらも `InferReport { written }` (書き込んだセルの添字) を返す。格子の `inference` の設定は見ない (方法の選択は T0611 のコマンドが行う)。
- キーフォームの型は `KeyformValues` トレイト (数値のフィールドを `f32` の列に平らにする・書き戻す・クランプする) で扱う。メッシュ・パーツ・ワープ・回転・グルーに実装した。
- 仕様で曖昧だった点を `docs/spec/keyform-inference.md` に書いた: 中立のキーの許容誤差 (1e-5)、基準のセルは `authored` 必須、片側外挿で基準のセルも数えること、推定した 1 軸セルをクランプしてから `Δ_k` に使うこと、グルーの強さも 0〜1 にクランプすること、頂点数の不一致はエラー。
- `origins` が空の格子はすべて `authored` なので何も書かない。
- `AGENTS.md` のリポジトリ構成とツール層の一覧、`docs/glossary.md` の「(予定)」も直した。
- 後続: T0611 (コマンド) はエラーのとき格子を変えないことに依存してよい。T0612 のミラー・深度からの回転は同じクレートに追加する。
