---
id: T0506
title: 法線マップの生成
status: done
milestone: M4
crates: [ugoe-rigging, ugoe-ai, ugoe-editor-core, docs]
depends_on: [T0112, T0610, T0504, T0640]
assignee: claude-worker-T0506
---

# T0506: 法線マップの生成

## 目的

アルファの形から法線マップを作る決定的な方法 (`ugoe-rigging`) と、深度推定から作る方法 (`ugoe-ai`) を実装する。

## 受け入れ条件

- [x] 決定的な方法のテスト (円形のアルファから放射状の法線ができるなど) がある
- [x] パーツごとにふくらみの強さを調整できる
- [x] 結果を提案としてエディタに出せる
- [x] `just ci` が通る

## 参考

- `docs/ai-integration.md#法線マップの生成`

## メモ

- 法線マップを設定・差し替え・外す `Command` はまだない (T0640 のメモ)。生成した法線マップを提案として出すには、この Command (と `AddTexture` との関係) を先に足す必要がある。範囲に `ugoe-editor-core` を足すこと。

- 範囲の追加 (team-lead の承認済み): `ugoe-editor-core` (法線マップの Command が要るため) と `docs` (仕様とコードを同じ PR で合わせるため)。
- やったこと
  - editor-core: `Command` の末尾に `SetNormalMap { texture, normal_map, png }` と `SetLighting { enabled }` (OP_COUNT 92、見本は `codegen::lighting_samples`)。
    `RemoveTexture` は、ほかから指されない法線マップの画像も表から除く (Undo で戻ることをテスト)。規則は `docs/editor.md#法線マップ`。
  - ugoe-rigging `normal.rs`: `bulge_normal_map` (`BulgeOptions { strength, radius, smoothing, alpha_threshold }`)・`HeightField`・`encode_normal` / `decode_normal`。
    形全体のふくらみは `∇²u = −4` をマルチグリッドで解いて `√u` を高さにする (円から半球、帯から半円の筒)。
    最初は輪郭からの距離変換を高さにしていたが、形の芯に折れ目ができ、光を当てると放射状の筋や稜線が見えたので変えた。
    `radius` を指定すると距離変換で縁だけを丸める。
  - ugoe-ai `normal.rs`: `propose_bulge_normal_maps` と `propose_depth_normal_maps` / `plan_depth_normal_maps`。`NormalMapOptions { default, per_texture, textures, replace_existing }` で
    テクスチャ (パーツ) ごとに設定を変えられる。深度からは、テクスチャの画素を覆うメッシュの三角形から基準の姿のキャンバス座標を求めて深度を引き、全体で正規化し、
    縁の丸み (`rim`) を足す。式と規則は `docs/ai-integration.md#法線マップの生成`。`turn.rs` の `percentiles` と `PERCENTILE` を `pub(crate)` にして共有した。
  - `docs/spec/rendering.md` の「実装待ち」の注記を外した。
- 確認
  - `ugoe-rigging/tests/normal.rs`: 円から放射状の法線 (半球の法線 x = r/R に近い)、`strength` に比例、`radius` で縁だけ丸まる、決定的。単体テストで円の `u ≈ R² − r²`。
  - `ugoe-ai/tests/normal.rs`: 実際の PSD (胸から上) で全テクスチャに法線マップを付けて `validate` を通り、1 回の Undo で画像ごと戻る。テクスチャごとの強さ、作り直しの有無、
    深度 (半球から放射状、UV の左右反転に従う、テクスチャの解像度によらない、平らな深度に縁の丸み)。
  - GPU テスト (`ugoe-ai` の feature `gpu-tests`、dev-dependency に ugoe-render・wgpu・pollster): 円に左上から光を当てると左上が明るく右下が暗い、PSD で法線マップが陰影を変え、
    光源がなければライティングを受けない場合と同じ。`just test-gpu` が通った (dzn)。
  - 目視 (`UGOE_NORMAL_RENDER_DUMP` で書き出し、Read で確認): scratchpad の `T0506/disc-lit.png` (なめらかな球)・`bustup-flat.png` (法線マップなし)・
    `bustup-bulge.png` (アルファから)・`bustup-depth.png` (人物を覆う楕円体の深度から)。筋や折れ目はない。
- 確かめていないこと・分かったこと
  - エディタの画面からの呼び出し (T0637 が提案のプレビューと採用を扱う)。実際の深度推定サーバの出力での見た目 (合成の深度マップでだけ確かめた)。
  - 既定の強さ 1 (断面が円) は、顔や体のような大きなパーツには強い (縁が真横を向いて暗くなる)。パーツは PSD のレイヤーごとに別々にふくらむので、重なって隠れた部分の形も効き、
    境目 (首と肩など) に段差が見える。
- 後続の候補
  - 意味ラベルごとの既定の強さ (顔・体は弱く、髪は強く、目・口は平らに、など) を `propose_bulge_normal_maps` に。
  - `Mesh::lit` を変える Command (`SetMeshProperties` にない)。目のハイライトなどを外すため。
  - エディタで生成の設定 (テクスチャごとの強さ) を選んで提案を出す画面。
