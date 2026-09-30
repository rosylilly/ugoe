---
id: T0707
title: 見本: 法線マップ付きの見本のアバターと、ビューアのサンプルで試せるようにする
status: done
milestone: M4
crates: [ugoe-cli, ugoe-rigging, assets, apps/viewer, docs]
depends_on: [T0506, T0705]
assignee: claude-worker-T0707
---

# T0707: 見本: 法線マップ付きの見本のアバターと、ビューアのサンプルで試せるようにする

## 目的

利用者の要望 (T0705 の後続の候補)。ライティングを使う見本のモデルがなく、ビューアのライティングのパネルをすぐ試せない。
見本のアバターに法線マップを付けた版を用意し、ビューアの「サンプル」から開けるようにする。

## 受け入れ条件

- [x] 見本のジェネレータ (`ugoe sample avatar`) で、法線マップ付きの版 (例: `assets/samples/avatar-lit.ugoe`) を作れる。法線マップは T0506 の方法で作り、顔や体が不自然にならないよう、パーツごとの強さ (意味ラベルごとの既定の強さなど) と境目の段差を調整する。既存の見本 (`avatar.ugoe`) の出力は変えない
- [x] コミットしたファイルと生成の結果が一致することをテストで確かめる。ファイルの大きさを抑える
- [x] ビューアの「サンプル」で、法線マップ付きの版を選べる (または切り替えられる)
- [x] 描いた絵を目で確かめ (左・右・上からの光)、メモに画像のパスを書く
- [x] `just ci`・`just test-gpu`・ビューアのビルドが通る

## 参考

- `docs/tasks/T0705-viewer-lighting.md`・`docs/tasks/T0506-normal-map-gen.md` のメモ

## メモ

### やったこと

- `ugoe sample avatar --lit -o assets/samples/avatar-lit.ugoe` (`crates/ugoe-cli/src/sample/lit.rs`、`sample.rs` には `pub mod lit;` の 1 行だけ):
  `build_avatar` の出力に、すべてのテクスチャの法線マップ (`SetNormalMap`) と `SetLighting` を 1 つの Batch で足す。リグ・物理・メタデータ・テクスチャは
  `avatar.ugoe` と同じ (**`avatar.ugoe` の出力は変えていない**)。ugoe-cli に ugoe-rigging (ツール層、ワークスペースの内部クレート) の依存を足した。
  - 法線マップは T0506 の方法 (`ugoe_rigging::bulge_height`) だが、テクスチャごとではなく部位のまとまりごとにふくらませる。まとまりの形
    (メンバーのアルファを基準の姿のキャンバスに並べた和) を 1 つのふくらみにして、その高さを各テクスチャに写してから法線にする。
    - 顔 + 目・眉・鼻・口 (強さ 0.35、縁 48px だけ丸める): 目・口が顔の上で平らに浮かない。
    - 首 + 体 + 腕 (強さ 0.3、形全体。形には顔も加える): 首の付け根・肩の上の腕に段差が出ない。顔を加えるのは、首が形の上端になって
      上を向き、あごの下が顔より明るく見えたため。腕に縁の丸みを足す案も試したが、腕の輪郭 (PSD で隠れている部分が粗い) がギザギザの線で見えたのでやめた。
    - 前髪・後ろ髪 (0.5)、耳 (0.5)、耳飾り (0.6) はそれぞれ単独。
  - 部位のキャンバスでの位置は、PSD を取り込み直したメッシュ (親がないので頂点がキャンバス座標) から読む (基準の姿では口などが変形しているため)。
  - 形の外の画素に隣の法線を 4 画素広げる (縮小・描画の補間で正面向きの法線が輪郭に混ざり、肩の上の腕の輪郭に細い線が見えたため)。
  - 法線マップはテクスチャの半分の解像度 (法線マップは UV の範囲に引き伸ばして使われる)。ファイルは 1.57MB (`avatar.ugoe` 1.37MB + 法線マップ約 200KB)。
- **ugoe-rigging のバグ修正 (範囲外。team-lead が承認し、crates: に ugoe-rigging を追加した)**: `inflate` のマルチグリッドで、粗い格子の画素を「4 画素のうち 2 つ以上が形の中」
  としていたため、細い切れ込みの多い形 (髪など) で粗い格子の形が広がり、V サイクルが発散・振動していた。見本の PSD で SOR の参照解と比べると
  前髪 43.4 → 1.2、後ろ髪 101 → 397、袖 78 → 51 (顔だけ合っていた)。「4 画素すべてが形の中」に変えると全部位で参照解と合う (最大の誤差 0.1px 以下)。
  テスト `ragged_shapes_match_the_reference_solution` (櫛・格子・切れ込みの円を参照解と比べる。前の規則では失敗する) と `docs/ai-integration.md` の記述。
  エディタの「Generate」と `ugoe_ai::normal` の結果も変わる (髪などが正しくふくらむ)。
  - 修正は inflate の粗い格子の規則だけ (関数の署名は変えない。world.rs は触らない)。テストは debug で約 1.3 秒 (参照解の SOR は 1500 回)。
    前の規則に戻すと櫛の形で失敗する (高さの差 29px) ことを確かめた。
  - 性能: V サイクルの回数 (8) は変わらず、粗い格子の形の中の画素はむしろ減る。`ugoe sample avatar --lit` (debug、24 枚の法線マップ) は
    新 9.2〜11.4 秒 / 旧 9.3〜12.5 秒で、差は計測のぶれの範囲。
  - 出力が変わるもの: コミットされた法線マップは今回の `avatar-lit.ugoe` だけ (`avatar.ugoe` と compat の資料は法線マップを持たず、`just ci` の
    compat テストも通った)。ugoe-ai (`tests/normal.rs`・GPU の `normal_render.rs`)・editor-core (`lighting_tests.rs`)・mcp のテストは期待値を変えずに通った。
- テスト (`sample::lit::tests`): アバターとの違いはライティングだけ・法線マップの大きさ、まとまりの中で向きがつながる (目・鼻・口と顔、首・腕と体で 3° 未満)・
  顔の中ほどが正面・頬の輪郭が外向き・髪がふくらむ、コミットしたファイルが最新 (モデルは `avatar.ugoe` + 法線マップの名前とライティング、
  法線マップは画素ごとに差 2 以内。浮動小数点の環境差を許すためバイト列では比べない)。
- ビューア: `samples.ts` の `SAMPLES` に「アバター (ライティング)」(`avatar-lit.ugoe`) を足しただけ (開く流れは変えていない)。`samples.test.ts` に 1 行。
- ドキュメント: `assets/samples/README.md` (表・「avatar-lit.ugoe」の節・`avatar-lit.jpg`)、`docs/manual/lighting.md` (「見本で試す」)、
  `crates/ugoe-cli/README.md` (`--lit`)、`apps/viewer/README.md` (メニューの説明・同梱の大きさ約 3MB・確かめ方)。

### 確かめたこと

- `just ci` (main (T0711) に rebase し、`just wasm` をやり直した後も)・`just test-gpu`・`just viewer-build --bundles deb`。
- `ugoe render` (llvmpipe) で左・右・上から光を当てて目で確かめた (環境光 0.55、平行光源 0.6、高さ 45°。ビューアの「標準」に近い):
  /tmp/claude-1000/-home-rosylilly-src-github-com-rosylilly-ugoe/32f91561-94f7-4861-9c03-8fe8c9ff7192/scratchpad/T0707/final-lrt.png (全身)・final-face.png・final-body.png (顔・体の拡大、左・右・上)。途中の版は T0707/v1〜v10 (v1〜v2 はマルチグリッドのバグで髪が平ら、
  v3〜v6 は肩の上の腕の輪郭に線)。まとめは `assets/samples/avatar-lit.jpg`。
- ヘッドレスの Chrome (SwiftShader の WebGL2、`vite preview` のビルド済みの `dist/`) で「サンプル」→「アバター (ライティング)」を開き、
  `usesLighting` が真でライティングのパネルが出ること、方位を変えると陰影が変わること、「アバター」に戻すとパネルが消えることを確かめた:
  /tmp/claude-1000/-home-rosylilly-src-github-com-rosylilly-ugoe/32f91561-94f7-4861-9c03-8fe8c9ff7192/scratchpad/T0707/shots/01-menu.png・02-lit-standard.png・03-left.png・03-right.png・03-top.png (lrt.png にまとめ)。
  読み込み時に 404 が 1 件出る (T0710 と同じもの。見本のファイルではない)。

### 確かめていないこと・後続の候補

- Tauri (WebKitGTK) のウィンドウでは動かしていない (`.deb` は作れた)。
- 顔は淡い肌の色なので、光が当たらない側は灰色がかって見える (陰影の掛け方としては正しい)。好みで顔の強さ・縁の幅を変える余地がある。
- 目のハイライトなど光らせたくない部分の `lit` は切っていない (目は顔と同じ向きを受けるので不自然さは小さい)。
- 部位のまとまりでふくらませる方法は、`ugoe_ai::normal` / エディタの生成にも使える (選んだメッシュをまとめて 1 つの形にする設定)。意味ラベルごとの既定の強さ (T0506 の後続の候補) と合わせて検討するとよい。
- 見本のテクスチャを `avatar.ugoe` と共有できないので、同梱の見本は約 3MB になった。
