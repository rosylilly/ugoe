---
id: T0640
title: エディタ: プロジェクトに法線マップの画像を保存する
status: todo
milestone: M4
crates: [ugoe-editor-core, docs]
depends_on: [T0112]
assignee:
---

# T0640: エディタ: プロジェクトに法線マップの画像を保存する

## 目的

T0112 で `TextureRef::normal_map` と書き出しの `NRML` チャンクを足したが、`.ugoeproj` の保存と読み込みは法線マップの画像を扱っていない。
法線マップを持つモデルを保存して開き直すと、書き出しが `MissingTexture` で失敗する。

## 受け入れ条件

- [ ] `Project::save` / `load` が法線マップの画像を保存・読み込みし、往復できる (保存の場所と名前の規則を決めて書く)
- [ ] 法線マップを持つモデルを保存 → 開き直す → 書き出す、が通るテスト
- [ ] 既存のプロジェクトがそのまま読める。`project.rs` の `TODO(T0640)` を消す
- [ ] `just ci` が通る

## 参考

- `docs/tasks/T0112-normal-map-data.md` のメモ
- `docs/spec/file-formats.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
