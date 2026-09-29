---
id: T0209
title: 参照レンダラ: ライティング
status: done
milestone: M4
crates: [ugoe-render]
depends_on: [T0112]
assignee: claude-worker-T0209
---

# T0209: 参照レンダラ: ライティング

## 目的

法線マップと光源 (環境光・平行光源・点光源) によるライティングを実装する。変形に合わせて法線を回す。

## 受け入れ条件

- [x] 光源の API (`Lighting`) をホストから渡せる
- [x] 変形 (回転・反転) に合わせて法線が回ることを確かめる GPU テストがある
- [x] 法線マップのないメッシュは法線 (0, 0, 1) として扱う
- [x] WebGL2 で動く
- [x] 仕様書の「実装待ち」を更新する
- [x] `just ci` が通る

## 参考

- `docs/adr/0015-lighting.md`
- `docs/spec/rendering.md#ライティング`

## メモ

- 公開 API (追加のみ): `Lighting { ambient, lights }`・`Light::{Directional, Point}`・`MAX_LIGHTS` (8)、
  `Renderer::{set_lighting, set_normal_map, remove_normal_map}`。光源を渡さなければ環境光が白で光源なし (受けない場合と同じ色)。
  ホスト (ugoe の Puppet・wasm・ffi・headless・エディタ・ビューア) からはまだ呼んでいない。
- 決めた規約 (rendering.md#ライティング に書いた): 法線マップは G が画像の上向き (OpenGL 形式)。`direction` は光が進む向き。
  点光源の減衰は `clamp(1 − d / range, 0, 1)²`。座標はキャンバス座標 + 奥行きで、ビュー変換は陰影に影響しない。
- 法線を回す方法: フラグメントシェーダで、キャンバス座標と UV の画面での微分からヤコビ行列 `∂P/∂(u, v)` を求め (三角形の中で一定)、
  極分解の回転成分で回す。反転していれば u 軸を反転した写像を使い、法線の x を反転する。
- 描画の統合との関係: 法線マップはテクスチャと同じバインドグループ (group 2 の binding 2) に置くので、描画の状態は増えない。
  ライティングの種類 (受けない・平ら・法線マップ) は要素ごとの値 (`mask_params.z` の空き) で渡すので、受ける要素と受けない要素もまとめて描く。
  光源は group 0 の binding 2 のユニフォームで、ライティングを受ける要素を描くフレームで変わったときだけ転送する (`uniform_bytes` に数える)。
- ライティングなしの出力が変わらないことの確認: 実装の前後で既存の GPU テストが描いた 132 枚の画像を、lavapipe と llvmpipe の両方で
  画素単位で比べて一致した (テストに足した `UGOE_GPU_TEST_DUMP` で書き出した)。
- GPU テストの追加 (10 件): 陰影の式・受けないメッシュとモデル・法線マップの向き (G が上)・回転 / 反転 / 転置での法線の向き・
  ビューの反転・点光源の減衰・ふくらみの法線マップを変形しても光源の側が明るいこと・統合あり / なしの画素の一致 (ライティングあり)・
  光源の転送・法線マップの登録と置き換えと削除。単体テストに、参照シェーダを GLSL ES 3.00 (WebGL2) に変換できることを足した
  (dev-dependency の naga に `glsl-out` を足した)。`just test-gpu` は lavapipe・llvmpipe (GL)・既定のアダプタで 61 件が通った。
- 範囲外の変更: なし (Cargo.lock は ugoe-render の dev-dependency に png を足した分だけ)。
- 後続の候補:
  - ホストへの配線: `ugoe` の Puppet / `ugoe-wasm` / `ugoe-ffi` / `ugoe-headless` がバンドルの法線マップを `set_normal_map` で登録し、
    光源を渡す API を出す。CLI の `ugoe render --light`、エディタ・ビューアのプレビューで光源を動かす UI。
  - 光源をモデルごとに変えたい場合 (今はレンダラごと)。影・スペキュラは範囲外 (ADR 0015 にない)。
