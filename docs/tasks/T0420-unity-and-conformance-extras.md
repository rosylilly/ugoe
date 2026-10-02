---
id: T0420
title: Unity と適合テスト: Linear の正確な一致・アトラス化した見本・Godot の頂点の比較
status: done
milestone: M7
crates: [bindings/unity, bindings/godot, tests/conformance, ugoe-cli, assets, apps/editor, apps/viewer, docs]
depends_on: [T0419, T0418]
assignee: claude-worker-T0420
---

# T0420: Unity と適合テストの残り

## 目的

T0419・T0418 の後続の候補をまとめる。

## 受け入れ条件

- [x] Unity の Linear の色空間で、中間の RenderTexture を使って参照と正確に合わせる (今は Gamma を要求)
- [x] アトラス化した見本を同梱する (バッチングが効く見本。法線マップ付きのテクスチャのアトラス化も検討)。マスクの内容のアトラス
- UPM の tarball を GitHub Release に添付するのは、公開のマイルストーン (Release) の T0421 に移した
- [x] Godot から頂点を読み戻して比べる。Godot に JSON のモデルを読ませる
- [x] ビューアとエディタのスクリーンショットに wasm の read_pixels を使う
- [x] `just ci`・`just conformance`・`just unity-check`・`just godot-check` が通る

## 参考

- `docs/tasks/T0419-unity-followups.md`・`docs/tasks/T0418-conformance-more-backends.md` のメモ

## メモ

### やったこと

- **Unity の Linear の色空間 (bindings/unity):** `UgoePuppet.linearCompositing` (既定 `Intermediate`、`Direct` で前の描き方)。
  Intermediate では、まとめたメッシュを sRGB の変換をしない RenderTexture に sRGB の値のまま描き (シェーダのキーワード
  `UGOE_GAMMA_TARGET`、`Graphics.ExecuteCommandBuffer` なのでパイプラインに依存しない)、子の四角形 (`UgoeIntermediate`・
  `UgoeComposite.shader`) で画面に写す。写すときは IEC 61966-2-1 の式で線形に直す (UnityCG の `GammaToLinearSpace` は近似なので
  使わない。Direct の経路も同じ式にした)。テクスチャはモデルの外接矩形を写し、原点を画素の格子にそろえる (倍率 1 で `ugoe render` の
  画素と重なる)。モデルの中の合成は参照と一致し、シーンとの合成 (輪郭の半透明) だけが線形。警告は Direct のときだけ。
- **マスクの内容のアトラス:** 内容を入れ子の深さごとに 1 枚の RenderTexture (ページ) に棚詰めし (`MaskAtlasLayout`、
  `Runtime/Core/Offscreen.cs`)、読む位置の写像と通常マスクの係数を頂点属性 (UV3.zw・UV4) で渡す。`DrawBatch.MaskContent` は
  `MaskPage` に変えた (同じページの違う内容を読む要素もまとめる)。入れ子は浅いページを読んで深いページに描くので読み書きがぶつからない。
- **アトラスにした見本:** `ugoe sample avatar --atlas` (`--lit` とは同時に使えない) を足し、`assets/samples/avatar-atlas.ugoe`
  (新規、1,409,306 バイト。24 枚 → 994x1698 の 1 枚) をコミットした。`committed_atlas_sample_is_up_to_date` (ugoe-cli) が
  `avatar.ugoe` をアトラスにしたものと同じかを確かめる。Unity の `build.mjs` の見本に入れた。描画の呼び出しは
  avatar-atlas 24 → 1、dress-up をアトラスにすると 14 → 1 (マスクのページでさらに 2 → 1 に減った。`tests/BatchTests.cs`)。
  ほかの見本のファイルは変わっていない (avatar.ugoe も同じ PSD から作り直して同じ結果)。
- **法線マップのアトラス化は検討だけ:** 法線マップを色のテクスチャと同じ配置 (解像度の比で縮めた配置) で詰める必要があり、
  ugoe-editor-core の `atlas.rs` の変更になる (このタスクの crates: の外)。後続の候補に残す。
- **Godot (サブエージェント):** `bindings/godot/src/node/readback.rs` (`#[godot_api(secondary)]`。node.rs は `mod readback;` の 1 行) に
  `load_model_from_json`・`get_drawable_ids`・`get_drawable_positions`・`get_drawable_state`・`get_render_order`。conformance.gd が
  頂点のフレームを書き、run.mjs は Godot も `--images` (評価の結果と画像) で比べる。飛ばすケースは 0。smoke.gd に 8 項目。
- **ビューアとエディタのスクリーンショット (サブエージェント):** ビューアのツールバーに「画像を保存」、エディタのビューポートの右上に
  「Screenshot」。どちらも `WebPuppet.read_pixels()` で取り、2D canvas で PNG にして、両方のバックエンドに足した `write_png_file`
  (本文が PNG、パスはヘッダー `x-ugoe-path`。拡張子と PNG の署名を確かめる) で書く。ギズモ・光源の印・HUD は写らない。
  ipc.ts・wasm のスタブ・docs/editor.md・docs/manual/screen.md・apps/viewer/README.md を更新。

### 確かめたこと

- main (T0416・T0422・T0415 の後) に rebase し、`just wasm` 2 つの後に `just ci`・`just unity-check` (dotnet test 39 件・型検査)・
  `just godot-check` (53/53)・`just conformance` (c・wasm・wasm-webgpu・unity・godot がそれぞれ 53/53) が通る。
- Unity の C# の層: ページの配置が重ならないこと・収まらなければ倍率を下げること・写像、中間のテクスチャの画素の格子、
  不透明な画素が Linear の書き込み先で 256 値すべて元に戻ること、半透明の重なりで中間のテクスチャは参照と一致し直接描くとずれること
  (`tests/OffscreenTests.cs`)。
- Godot: 画像は 100% の画素が 3/255 以内。頂点を 0.05px ずらすと FAIL になる。
- スクリーンショット: ヘッドレス Chrome (WebGL2) と使い捨てのブリッジで、保存した PNG と画面を比べた。スクリーンショットは
  scratchpad の `T0420/out/` (viewer-webgl2-page.png・editor-webgl2-gizmo-page.png・dialog-*.png など)。

### 確かめていないこと (利用者の確認に残す)

- **Unity の上では動かしていない** (この環境に Unity が無い)。Linear のプロジェクトで中間のテクスチャが Gamma のプロジェクトと
  (輪郭を除いて) 同じ画素になること、写す四角形とマスクのページの上下の向き (各グラフィックス API)、`SetViewport` でページの領域に
  描けること、シェーダのコンパイル (TEXCOORD5 までの補間・`multi_compile_local`)、Built-in と URP。確かめ方は bindings/unity/README.md。
- スクリーンショットの WebGPU (この環境の SwiftShader ではアプリが変更と無関係にデバイスを失い続ける)、本物の Tauri のウィンドウと
  OS の保存ダイアログ、devicePixelRatio が 1 以外。
- CI の上 (unity.yml・conformance・godot のジョブ)。

### 範囲外の変更

- なし (crates: の範囲内。ugoe-cli・assets・bindings/unity・bindings/godot・tests/conformance・apps・docs)。

### 後続の候補

- 法線マップのあるテクスチャのアトラス化 (ugoe-editor-core。avatar-lit でも描画がまとまる)。
- Unity の中間のテクスチャを画面の画素にそろえる (カメラごとの描画。バイリニアの標本化し直しをなくす)。
- `compare --images-only` は使う実行器が無くなった。残すか消すか。Godot は読み込みの直後にも step(0) を呼ぶ (評価が 1 回多い)。
- ヘッドレスの SwiftShader の WebGPU でビューアとエディタのデバイスが失われ続ける問題の調査。保存の選択肢 (透明・倍率)。
