---
id: T0505
title: 深度推定による振り向きリグの提案
status: done
milestone: M6
crates: [ugoe-ai, docs]
depends_on: [T0501, T0612, T0509]
assignee: claude-worker-T0505
---

# T0505: 深度推定による振り向きリグの提案

## 目的

原画から深度マップを推定し、ワープデフォーマの制御点の `depth` と、`depth_rotation` による振り向きのキーフォームを提案する。

## 受け入れ条件

- [x] 深度推定を `Provider` の裏に隠し、録画した応答でテストできる
- [x] 意味ラベルで深度を補正する
- [x] 提案が `validate` を通り、`ugoe render` で振り向きの画像を確認するテストがある
- [x] `just ci` が通る

## 参考

- `docs/ai-integration.md#深度推定による振り向きリグ`
- `docs/spec/keyform-inference.md#深度からの回転`

## メモ

- やったこと
  - `ugoe_ai::depth`: 深度推定は言語モデルの `Provider` では得られないので、別のトレイト `DepthEstimator` の裏に隠した
    (受け入れ条件の「`Provider` の裏に」はこの形で満たす。team-lead の了承済み)。`disclosure` は `Provider` と同じ `Disclosure` を返す。
    実装は HTTP の推論サーバ (`HttpDepthEstimator`、既存の `Transport` を使い録画の再生でテスト) と決めた深度マップ (`FixedDepthEstimator`)。
    サーバとの約束 (本文・応答・値の向き `DepthPolarity`) は `docs/ai-integration.md#深度推定器` に書いた。新しい外部の依存はなし。
  - `ugoe_ai::turn`: `propose_depth_turn` (推定器を呼んで提案を作る) と `plan_turn` (深度マップから設定を決める)。
    制御点を覆う最も手前のメッシュのラベルで補正する (`label_depth`。前髪は 0.8 以上、後ろ髪は −0.3 など)。
    部位 (頭・体) と回転の中心・起伏・`angle_scale` もラベルから決める。式と表は `docs/ai-integration.md#深度の決め方-ugoe_aiturn`。
  - 依存: ugoe-ai に ugoe-core・ugoe-model・ugoe-rigging・ugoe-runtime を足した (どれもワークスペース内で、ツール層が依存してよい)。
    ugoe-runtime は dev-dependency の予定だったが、基準の姿のキャンバス座標 (制御点・メッシュの三角形) を得るのに評価が要るので通常の依存にした。
- 決めたこと・気づいたこと
  - 深度の正規化は、部位のメッシュが覆う点を 48×48 の格子で標本化して分位点 (2%・98%) で行う。メッシュの頂点だけで行うと、
    扇形のメッシュでは内側の点が 1 つしかなく、分位点が輪郭の値になって内側がすべて 1 に張り付いた。
  - キーの足し方の順序: 加算合成の格子のまま AngleY の端のキーを足すと、斜めのセルがその場で `additive` になり、
    `InferKeyforms { depth_rotation }` (`overwrite_derived: false`) では作り直されない。中立のキー → `SetKeyformInference` → 端のキーの順にし、
    足したキーの `authored` のセルを `MarkKeyformDerived` で推定に戻す。もとからあったキーのセルは書き換えない (テストあり)。
  - ラベルは T0503 の `classify::Label` をメッシュごとに受け取る (`BTreeMap<MeshId, Label>`)。分類の結果 (`LayerLabel`) からは
    `mesh_labels` で作る (PSD の取り込みはメッシュの名前をレイヤーの名前にするので、`layer_path` の最後の名前で対応付ける。
    同じ名前のメッシュ・違うラベルの同名レイヤーは決められないので付けない)。左右 (`Side`) は深度の補正に使わない。
  - 範囲外の変更 (team-lead の承認済み。`crates:` に docs を足した): `docs/ai-integration.md` (深度推定器と深度の決め方の節) と
    `docs/spec/keyform-inference.md` (実装待ちの注記)。仕様とコードを同じ PR で合わせるため。
- 確認
  - `tests/depth_turn.rs`: 録画 (`tests/cassettes/depth_face.json`、顔を楕円体・後ろ髪を奥・前髪を額と同じ深さにした 32×32 の合成の深度マップ。
    実際のサーバの録画ではない) を再生して提案を作り、`Batch` で適用して `validate` を通す。ugoe-runtime で AngleX = ±30 の中央の制御点が
    `d·sin30` だけ左右に、AngleY = 30 で上に動くこと、後ろ髪の点が逆へ回り込むこと、覆われない角は平らな板と同じことを数値で確かめた。
  - 目視: `cargo run -p ugoe-ai --example depth_turn -- /tmp/turn.json` と `ugoe render` で AngleX = ±30・15、AngleY = ±30、(30, 30) を描いた。
    AngleX で目鼻口が向いた側へ寄り、後ろ髪が反対側へはみ出して振り向いて見える。AngleY で上下を向く。前髪は顔と一緒に手前側で動く。
    (30, 30) は縦の線が傾いて顔全体が斜めに歪む (仕様どおり。T0612 のメモと同じ)。4×4 の粗いワープなので、目の線が深度の差で少し曲がる。
  - 受け入れ条件の「`ugoe render` で確認するテスト」は、team-lead との取り決めどおり GPU のテストは足さず、自動のテストは ugoe-runtime での
    数値の確認 (`tests/depth_turn.rs`)、`ugoe render` での確認は例 (`examples/depth_turn.rs`) と目視で行った。
- 後続の候補
  - PSD の取り込みがレイヤーの経路 → メッシュ ID の対応を返すようにし、`mesh_labels` の名前による対応付けを置き換える。
  - 実際の深度推定サーバ (Depth Anything V2 など) の録画と、ローカルで動かす推定器 (ONNX など。依存の相談が要る)。
  - エディタからの呼び出し (送る前の確認に `disclosure` を使う) と、PSD の合成画像を原画として渡す経路。
  - メッシュの頂点ごとの深度 (`Mesh::depth`) の提案と、ワープの親がワープの場合 (rigging が未対応)。
  - 補正の値 (起伏 0.3・前髪 0.8 など) は合成の顔で決めた。実際の絵 (`assets/fixtures/psd-real/`) で調整する。
