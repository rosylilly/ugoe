---
id: T0705
title: ビューア: ライティングの光源の操作
status: done
milestone: M4
crates: [apps/viewer, docs]
depends_on: [T0210]
assignee: claude-worker-T0705
---

# T0705: ビューア: ライティングの光源の操作

## 目的

利用者の指摘。wasm には光源の口 (`set_lighting_json`) があるが、ビューアで操作できない。

## 受け入れ条件

- [x] ライティングを使うモデル (`uses_lighting`) を開いたら、光源のパネル (環境光・平行光源・点光源の向き・色・強さ。既定の光源の組) を出す。設定をビューアに残す
- [x] ヘッドレスのブラウザで確かめる。ビューアの README とマニュアルを更新する
- [x] `just ci`・ビューアのビルドが通る

## 参考

- `docs/tasks/T0210-lighting-hosts.md` のメモ

## メモ

### やったこと

- `apps/viewer/src/lib/lighting.ts`: 画面で扱う光源の設定 (`LightingSettings`。平行光源は来る向きの方位・高さ (度)、点光源の位置・手前・範囲は
  キャンバスの大きさに対する割合、色は `#rrggbb` と強さ)、既定の光源の組 3 つ (標準・夕日・夜のランプ)、`set_lighting_json` の JSON への変換
  (`toLightingJson`。平行光源の `direction` は来る向きの逆)、`localStorage` (`ugoe-viewer.lighting`、版 1) への保存と、壊れた値に強い読み込み
  (`sanitizeLighting`: 足りない値は既定、範囲の外は収める。JSON が壊れていたり localStorage が例外を出したりしても既定で始める)。
- `LightingPanel.svelte` (畳める `<details>`)・`LightPad.svelte` (ドラッグとキーボードで向き・位置を選ぶ面)。`Player` に `usesLighting`・`lighting`・
  `setLighting`・`updateLighting`。モデルを開くたびに `uses_lighting()` を見て、キャンバスの大きさから光源を渡し直す (GPU を作り直したときも)。
  パネルは `player.usesLighting` のときだけ出す (使わないモデルでは出さない)。
- 右のパネルの並び: トラッキング → モデルの機能ごとのパネル (そのモデルが使うときだけ出す、畳める `<details>`) → パラメータ。
  T0706 (着せ替え) のパネルは同じ形でライティングの隣に足せる (`App.svelte` のコメント)。
- Vitest (`lighting.test.ts`、13 件): 向きの変換・点光源の位置・既定の組・面との往復・壊れた値・保存の往復・例外を出す localStorage。
- `stub.d.ts` に `canvas_size`・`set_lighting_json`・`uses_lighting`。README (構成・使い方の表・「ライティング (T0705)」の節・右のパネルの並び)、
  `docs/spec/rendering.md#ホストでのライティング` にビューアの 1 項目。

### 確かめたこと

- `just ci`・`just viewer-build --bundles deb` が通る。
- ヘッドレスの Chrome (SwiftShader の WebGL2) でビルドした `dist/` を開いて確かめた。確かめ用のモデルは、見本のモデル (`assets/samples/avatar.ugoe`) の
  24 枚のテクスチャに `ugoe_rigging::bulge_normal_map` (強さ 0.6) で法線マップを付け、`lighting` と全メッシュの `lit` を有効にしたバンドル。
  リポジトリの外 (scratchpad の小さな Rust のプログラム) で作り、コミットしていない。
  - 見本のモデル (ライティングなし): `usesLighting` は偽でパネルは出ない。ライティングありのモデル: パネルが出て「標準」。
  - 描画領域の左右の明るさ (背景を除く平均): 平行光源を左から (方位 180°) で 左 75.6 / 右 61.2、右から (0°) で 左 62.9 / 右 77.2。
    点光源を右下 (80%, 80%) に動かすと 左 46.8 / 右 76.0・上 46.7 / 下 69.1。丸い面を上の縁までドラッグすると方位 90°・高さ 19°。
    既定の組のボタンで見出しの名前が変わり、スライダーを動かすと「カスタム」になる。
  - 読み直しで設定が残る。壊れた値 (色が `nope`、強さが文字列、`x` が 1e99) は既定値と上限 (1.5) になり、壊れた JSON では「標準」で始まる。エラーは出ない。

### やらなかったこと・後続の候補

- 実際に法線マップを付けて作られたモデルでの見た目 (確かめ用のモデルは機械的に全テクスチャをふくらませたもので、顔などに不自然な陰影が出る)。
  Tauri (WebKitGTK) のウィンドウでは動かしていない (`.deb` は作れた)。
- 光源をビューポートの上で直接ドラッグする操作、光源の位置の表示。光源の設定をモデルごとに覚えること (いまはビューアで 1 つ)。
- 見本のモデル (`assets/samples/avatar.ugoe`) に法線マップを付けた版があると、ビューアの「サンプル」ですぐ試せる。
