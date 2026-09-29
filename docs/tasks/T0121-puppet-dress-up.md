---
id: T0121
title: 着せ替え: Puppet の API と合成の残り
status: done
milestone: M4
crates: [ugoe, ugoe-runtime, ugoe-model, ugoe-motion, ugoe-format, apps/editor, docs]
depends_on: [T0111, T0113]
assignee: claude-worker-T0121
---

# T0121: 着せ替え: Puppet の API と合成の残り

## 目的

T0111 で作った合成 (`compose`) を、ランタイム SDK から使えるようにし、T0111 で残した合成の制限を解く。

## 受け入れ条件

- [x] `Puppet` にモジュールの付け外しの API があり、組み替えでパラメータの値と物理の状態の扱いが決まっている (`ParameterValues::carry_over` を使う)
- [x] モジュールの最上位のパーツを、本体のパーツ (スロットで指定) にぶら下げられる
- [x] 本体の衝突体をスロットで公開し、モジュールの物理がそれに当たる
- [x] モジュールの要素をポーズのグループに入れられる
- [x] `docs/spec/modules.md` を更新し、`just ci`・`just wasm-check` が通る

## 参考

- `docs/tasks/T0111-modules-compose.md` のメモ
- `docs/adr/0014-composable-modules.md`

## メモ

- 型: `Slot` に `part` (モジュールの最上位のパーツの親)・`colliders` (公開する本体の衝突体)・`pose_parts`
  (追従させてよい本体のポーズの要素)、`Module` に `pose: ModulePose { groups, links }` を足した。どれも serde default で
  空なら書き出さないので `SCHEMA_VERSION` は据え置き。model.md・modules.md に反映。
- 検証: `validate` がスロットの `part`・`pose_parts` を、`validate_physics` がスロットの `colliders` を検査する。
  `validate_module` は本体の衝突体・本体のポーズの要素への参照を除いて検査する (合成時に検査)。
- 合成: 最上位のパーツはスロットの `part` の子になる。リグの衝突体の参照は、モジュールのものなら付け替え、
  そうでなければスロットの `colliders` にある本体の衝突体としてそのまま残す (`ComposeError::UnknownCollider`)。
  ポーズは `links` を本体の要素の `links` に加え (`ComposeError::PoseNotExposed`)、`groups` を本体のグループの後ろに加える。
- Puppet: `crates/ugoe/src/dress_up.rs` に `attach_module` / `detach_module` / `set_modules` / `modules` / `base_model` /
  `texture_sources` と `DressUpChange` / `DressUpError` / `TextureSource`。T0210 と衝突しないよう、`lib.rs` は
  `mod`・`pub use`・`wardrobe` フィールドの追加だけにした。最初にモジュールを付けたときのモデルと物理を本体として覚える。
  - パラメータは値と基準値 (`base` / `output`) を `ParameterValues::carry_over` で引き継ぐ。
  - ポーズは作り直して `PosePlayer::carry_over` で同じ並びのグループの状態を引き継ぐ。
  - 物理は合成した設定で作り直す (リセット。次の update でその時の入力の静止姿勢から)。本体に物理が無くてもモジュールに
    リグがあれば、重力 (0, 1)・60 Hz の設定に加える。
  - テクスチャ: `DressUpChange::textures` に設定し直すべき番号を返す (出どころが変わったもの・増えたもの・同じ名前の別の
    `Arc` のモジュールに差し替えたもの)。利用者は `texture_sources()` の出どころの画像を `Renderer::set_texture` /
    `set_normal_map` で設定する。
- `crates:` に `ugoe-motion` と `apps/editor` を足した (team-lead 承認済み)。`PosePlayer::carry_over` を足すためと、
  `Slot` のフィールドの追加で型の生成物 (`apps/editor/src/lib/generated/types.ts`) を `just gen` で作り直すため。
  `Slot` / `Module` の構造体リテラルの追従で `ugoe-runtime/examples/compose_timing.rs` と `ugoe-model` のテストを直した。
- `ugoe-format` の `module_file::tests::rejects_invalid_module` を 1 か所直した。モジュールにない衝突体への参照が
  本体の衝突体への参照として許されるようになり、`validate_module` が見つけなくなったため (`frame` の誤りで試す)。
- 確かめたこと: `just ci`・`just wasm-check`・`just test-gpu`。
- 後続の候補:
  - 組み替えで物理の状態 (粒子) を引き継ぐ (同じ ID のリグだけ引き継ぐなど)。今はリセットで、本体の髪の揺れも止まる。
  - モジュールを付けている間に `set_physics` で渡した設定は、次の組み替えで本体の設定から作り直される。本体の物理を
    差し替える API (`set_base_physics` など) があるとよい。
  - wasm / FFI / ビューア・エディタからの付け外し (`.ugoemod` を読んでテクスチャを登録するところまで) は未接続。
