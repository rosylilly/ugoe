---
id: T0640
title: エディタ: プロジェクトに法線マップの画像を保存する
status: done
milestone: M4
crates: [ugoe-editor-core, docs]
depends_on: [T0112]
assignee: claude-worker-T0640
---

# T0640: エディタ: プロジェクトに法線マップの画像を保存する

## 目的

T0112 で `TextureRef::normal_map` と書き出しの `NRML` チャンクを足したが、`.ugoeproj` の保存と読み込みは法線マップの画像を扱っていない。
法線マップを持つモデルを保存して開き直すと、書き出しが `MissingTexture` で失敗する。

## 受け入れ条件

- [x] `Project::save` / `load` が法線マップの画像を保存・読み込みし、往復できる (保存の場所と名前の規則を決めて書く)
- [x] 法線マップを持つモデルを保存 → 開き直す → 書き出す、が通るテスト
- [x] 既存のプロジェクトがそのまま読める。`project.rs` の `TODO(T0640)` を消す
- [x] `just ci` が通る

## 参考

- `docs/tasks/T0112-normal-map-data.md` のメモ
- `docs/spec/file-formats.md`

## メモ

- **保存の規則:** 法線マップの画像はテクスチャと同じ `textures/` に `TextureRef::normal_map` の名前で置く
  (`textures/<normal_map>.png`)。`Project::textures` にも同じ名前で入れる (T0112 の `export_bundle`・`atlas.rs` と同じ)。
  名前の規則と大文字小文字の重複はテクスチャの名前と合わせて検査し、違反があれば何も書かない。複数のテクスチャが
  同じ法線マップを指してもよく、画像は 1 枚だけ置く。`normal_map` が指す画像は `remove_stale` で消さない。
  読み込みでは `normal_map` のすべてに画像が必要 (ないと `MissingTexture`)。形式のバージョンは変えない
  (法線マップのない既存のプロジェクトは同じファイルになる)。`docs/spec/file-formats.md#名前と読み込み` と `model.md#ライティング` に書いた。
- `Command::AddTexture` は、既存の法線マップの名前と (大文字小文字を区別せず) 同じ名前を拒否するようにした
  (画像の表とファイルが衝突するため)。
- テスト (`project_tests.rs`): 往復・保存し直しで消えない・外すと消える・共有は 1 枚・名前の検査・読み込みで画像がない・
  保存 → 開き直す → 書き出す (アトラス化あり・なし) で `NRML` に入る・`AddTexture` の衝突。
- **確かめていないこと:** エディタの画面からの操作 (法線マップを設定する `Command` や UI はまだない)。
- **後続の候補:** 法線マップの画像を設定・差し替え・外す `Command` と画面 (いまは model.json を直接書くか API からしか付けられない)。
  テクスチャを消したときに、ほかから指されない法線マップの画像を表から除く。
