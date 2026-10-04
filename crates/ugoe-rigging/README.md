# ugoe-rigging

**層:** T1 リギング (依存してよいクレートは `scripts/check-layers.mjs` を参照)

キーフォームの推定を行う。推定の結果はふつうのキーフォームとして格子に焼き込み、ランタイムは推定を知らない。

- 加算合成 (`infer_additive`、`infer_additive_target`)
- ミラー (`mirror_keyforms`、設定は `MirrorOptions`。書き込まずに試す `check_mirror`)。親が異なる対は、ランタイムの
  ワールド変換 (`ParentWorld`、逆はニュートン法) でキャンバス座標を経由して反転する
- 深度からの回転 (`infer_depth_rotation`)。`yaw` / `pitch` 以外の軸と手で直したセルは加算合成で足し合わせる
- 法線マップの生成 (`normal.rs`): アルファの形からふくらみの高さを作り、勾配を法線にする (`bulge_normal_map`、設定は `BulgeOptions`)。
  部位のまとまりのテクスチャを基準の姿のキャンバスに並べて 1 つの形としてふくらませる `group_bulge_normal_maps` (`PlacedTexture`、`normal/group.rs`) と、
  形に加えず向きだけを受けるテクスチャ (顔の上の眼鏡) も扱う `group_bulge_normal_maps_oriented` もある。
  まとまりの格子の升目の数は `MAX_GROUP_CELLS` までで、超えれば確保の前に `NormalMapError::GridTooLarge` を返す。
  高さの場 (`HeightField`) は `ugoe-ai` の深度推定からの法線マップでも使う ([docs/ai-integration.md#法線マップの生成](../../docs/ai-integration.md#法線マップの生成))

目視確認用に、深度からの回転とミラーで振り向きの顔を作る例がある
(`cargo run -p ugoe-rigging --example depth_face -- <out.json>` のあと `ugoe render <out.json> --param AngleX=30 -o <png>`)。

## 守ること

- 決定的であること (同じモデルと格子からは同じ結果)
- `authored` のセルを上書きしない。書き込むのは `derived` のセルだけ (ミラーの `overwrite_authored` を明示したときを除く)
- 書いたセルの由来に推定の方法 (`additive` / `depth_rotation` / `mirror`) を記録し、ミラーは元を `mirror_source` に記録する。
  `InferScope::Own` では別の方法で作ったセルを書き換えず、推定の元にする (T0614)
- モデルの型は `ugoe-model` に置き、このクレートはアルゴリズムだけを持つ
- 編集は `ugoe_editor_core::Command` から呼ぶ。このクレートは Undo を知らない

## 仕様

- [docs/spec/keyform-inference.md](../../docs/spec/keyform-inference.md)
- [docs/adr/0010-keyform-inference.md](../../docs/adr/0010-keyform-inference.md)

## 関連タスク

- T0610 (`docs/tasks/T0610-*.md`)
- T0611 (エディタのコマンド)、T0612 (ミラーと深度からの回転)
- T0506 (法線マップの生成)
