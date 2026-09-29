---
id: T0111
title: 着せ替え: スロットとモジュールの合成
status: done
milestone: M4
crates: [ugoe-model, ugoe-runtime, ugoe-editor-core, apps/editor]
depends_on: [T0109]
assignee: claude-worker-T0111
---

# T0111: 着せ替え: スロットとモジュールの合成

## 目的

`Model::slots` とモジュールのデータを定義し、本体とモジュールの組を合成して 1 つのモデルを作る。

## 受け入れ条件

- [x] スロットとモジュールの型を定義し、`validate` が参照を検査する
- [x] `compose(base, modules)` が ID の付け替え・取り付け・描画順の写像・`replaces` を行い、結果が `validate` を通る
- [x] 組み替えでパラメータの値を引き継ぐ
- [x] 中規模のモデルで合成とコンパイルにかかる時間を計測し、結果をメモに書く
- [x] フィクスチャとテストがある
- [x] `just ci` が通る

## 参考

- `docs/adr/0014-composable-modules.md`
- `docs/spec/modules.md`

## メモ

- 型: `ugoe_model::module` に `Slot` (`id` `anchor` `draw_order: DrawOrderRange { min, max }` `replaces` `parameters`
  `mask_sources`)、`Module` (`schema_version` `name` `slot` `requires` `parts` `deformers` `meshes` `glues` `textures`
  `physics: ModulePhysics { rigs, colliders }`)、`ModuleRequirements` (`avatar_level` `slots` `parameters`)。
  `Model::slots` は `serde(default)` で空なら書き出さないので `SCHEMA_VERSION` は据え置き。
  仕様書の `[min, max]` と `(slot, Module)` の組は、`{ min, max }` と `Module::slot` に改めた (modules.md に反映)。
- 検証: `validate` がスロット (ID の一意性・参照先・描画順の範囲) を検査する (`validate/module.rs`)。
  `validate::validate_module` はモジュールの要素を代わりのモデルに入れて `validate` / `validate_physics` を流用する
  (参照するパラメータは種類 `Unknown` の代わりを置き、本体のメッシュへのマスクは除く)。`unknown::in_module` を追加 (T0113 用)。
  アバタープロファイルの準拠の検査で `avatar.slots` の存在を調べる (T0110 の `TODO(T0111)` を解消)。
- 合成: `ugoe_runtime::compose` / `compose_physics`。モジュールの要素は本体の後ろに加えるので、本体のインデックスは変わらない。
  `replaces` はパーツ自身の不透明度を 0 にすると、ポーズ (`set_part_opacity_override`) やモーションの `part_opacity` が
  上書きして見えてしまうので、不透明度 0 の親パーツ `#replaced/<ID>` を挟む方式にした (モジュール名は `#` で始まれない)。
  最上位の要素の座標は `anchor` の局所座標系、パラメータは標準パラメータか `Slot::parameters` のものだけ使える。
- パラメータの引き継ぎ: `ParameterValues::carry_over` (同じ ID の値を新しい値域にクランプして写す)。
- 計測 (`cargo run --release -p ugoe-runtime --example compose_timing`、WSL2、30 回の中央値):
  本体 メッシュ 200 枚・頂点 12,800・ワープ 40 個 (格子は 2 軸 × キー 3) + モジュール 4 つ (計メッシュ 64 枚・ワープ 16 個)。
  合成 (検証込み) 0.31 ms、コンパイル (検証込み) 0.39 ms (うちモデルの複製 0.18 ms)、合成 + コンパイル + 最初の update 0.93 ms、
  毎フレームの update 0.28 ms。ADR 0014 の目標 (数 ms 以内) を満たす。
- `crates:` に `ugoe-editor-core` と `apps/editor` を足した (team-lead 承認済み)。`Model` にフィールドを足したので、
  `ugoe-editor-core/src/project.rs` の `Model` の構造体リテラルに `slots` を 1 行足さないとビルドが通らず、
  型の生成物 (`apps/editor/src/lib/generated/types.ts`) も `just gen` で作り直す必要があったため。
- ファイル形式: `slots` は `MODL` の `Model` に入るが、機能 `modules` は `docs/spec/file-formats.md` と
  `ugoe_format::features::FEATURES` に基準の機能として登録済みで宣言しないので、`ugoe-format` の変更は不要。
  `MODL` は名前付きの MessagePack (`to_vec_named`) なので、省略できるフィールドの追加で既存のファイルも読める。
- 後続の候補:
  - `ugoe` の `Puppet` に着せ替えの API (モジュールの付け外しで再合成・パラメータとポーズ状態の引き継ぎ・物理の再構築) を足す。
  - モジュールの最上位のパーツは最上位のままにしている。本体のパーツ (体・頭) の不透明度に追従させたいなら、スロットに `part` を足す。
  - モジュールの物理が本体の衝突体 (頭など) と当たれるよう、スロットで衝突体を公開する (`mask_sources` と同じ形)。
  - モジュールの要素をポーズのグループに入れる手段はない (袖の差分など)。
