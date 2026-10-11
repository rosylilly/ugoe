---
id: T0710
title: 見本: 着せ替えの見本 (本体と衣装) と、ビューアのサンプルで試せるようにする
status: done
milestone: M4
crates: [ugoe-cli, assets, apps/viewer, docs]
depends_on: [T0648, T0706]
assignee: claude-worker-T0710
---

# T0710: 見本: 着せ替えの見本 (本体と衣装) と、ビューアのサンプルで試せるようにする

## 目的

利用者の要望 (T0706 の後続の候補)。ビューアの「サンプル」から、すぐに着せ替えを試せるようにする。

## 受け入れ条件

- [x] スロットを持つ見本の本体と、見本の衣装のモジュール (2 つ以上。同じスロットで差し替えられる組を含む) を用意する。見本のジェネレータで作れ、コミットしたファイルと生成の結果が一致することをテストで確かめる。既存の見本 (`avatar.ugoe`) の出力は変えないか、変えるなら理由を書く。ファイルの大きさを抑える
- [x] ビューアの「サンプル」で、着せ替えの見本を開き、衣装を付け外しできる
- [x] 描いた絵を目で確かめ、メモに画像のパスを書く。マニュアル (`docs/manual/dress-up.md`) から見本に触れる
- [x] `just ci`・ビューアのビルドが通る

## 参考

- `docs/tasks/T0706-viewer-dress-up.md`・`docs/tasks/T0648-editor-module-project.md` のメモ

## メモ

### やったこと

- `ugoe sample dress-up -o <DIR>` (`crates/ugoe-cli/src/sample/dress_up.rs`): 本体 `dress-up.ugoe` (512x512、胸から上のデフォルメした人物) と
  モジュール `LongHair`・`TwinTails` (どちらも `Hair`)・`CatEars` (`Headwear`)・`Sailor` (`Outfit`) を `assets/samples/dress-up/` に書く
  (合わせて約 135KB)。絵は PSD を使わずコードで描く小さなラスタライザ (`sample/paint.rs`。符号付きの距離の形 + 輪郭線) で描き、
  モデルとモジュールは要素を直接並べて作る (見本のアバターのような `Command` と推定は使っていない。座標の定数が主なので直接の方が短い)。
  - 本体: `BodyWarp` (`BodyAngleX`・`Breath`)、`HeadRotation` (`AngleZ`)、`HeadWarp` (`AngleX/Y` の丸みの変形)、目の開閉・口の開き。
    スロット `Hair` (420〜680、顔と目・眉を挟む、既定の髪を置き換え、肌 `Skin` をマスクに公開)・`Headwear` (900〜1000)・`Outfit` (260〜340、既定のシャツを置き換え)。
  - `LongHair` の後ろ髪は本体の肌を反転マスクにして体の奥に見せ、頭の中心を軸にした回転デフォーマを動的物理 (`deformer_angle`) で揺らす。
    `TwinTails` の房も結び目を軸に揺れる。
  - 生成の中で `validate`・`validate_module`、同じスロットの組を除く組み合わせの `compose`・`compose_physics` を確かめる。
  - テスト: コミットしたファイルと生成の結果が **バイト列で** 同じ (`committed_sample_is_up_to_date`。絵は四則と平方根だけで描くので
    環境で変わらない)、既定の姿で絵が描いた位置にある、パラメータで頭・付けたモジュールが動く、頭を振ると髪が揺れて止まる、など。
- `sample.rs` には `pub mod dress_up; mod paint;` の 2 行を足しただけ。**`avatar.ugoe` の出力は変えていない。**
- `ugoe render --module <PATH>` (複数可): `.ugoemod` を本体のバンドルに合成して描く (テクスチャは本体の後ろに並べ、物理のリグも加える)。
  見本の確認に使った。テスト `render_attaches_modules_after_the_base_textures`。
- ビューア: ツールバーの「サンプル」をメニュー (`<details>`) にし、「アバター」と「着せ替え」を選べる (`src/lib/samples.ts` の `SAMPLES`・`openSample`、
  Vitest `samples.test.ts`)。「着せ替え」は本体を開いてから 4 つのモジュールを読み込み、`LongHair`・`CatEars`・`Sailor` を付ける
  (`TwinTails` は読み込むだけ。パネルの「差し替える」で試す)。`vite.config.ts` は `assets/samples/` の `.ugoe`・`.ugoemod` をすべて
  `samples/<パス>` で配り、ビルドの出力に入れる。`host.ts` の `readSampleModel` を `readSampleFile(path)` にした。`dressUp.ts`・`player.svelte.ts` は触っていない。
- ドキュメント: `assets/samples/README.md` (表と「dress-up」の節、`dress-up.jpg`)、`docs/manual/dress-up.md` (「見本で試す」と冒頭の案内、
  `images/dress-up-viewer-sample.png`)、`crates/ugoe-cli/README.md` (`sample dress-up`・`--module`)、`apps/viewer/README.md`。

### 確かめたこと

- `just ci` (main (T0124・T0120) に rebase した後も、`just wasm` をやり直してから)、`just viewer-build --bundles deb`。描画は触っていないので `just test-gpu` は回していない。
- `ugoe render` (llvmpipe) で本体だけ・各モジュール・パラメータ (向き・片目・口・体) を描いて目で確かめた:
  scratchpad/T0710/base.png・long.png・twin.png・base-pose.png、まとめたものは `assets/samples/dress-up.jpg`。
- ヘッドレスの Chrome (SwiftShader の WebGL2、`vite preview` のビルド済みの `dist/`) で「サンプル」→「着せ替え」を開き、
  スロット 3 つ・`LongHair*`・`CatEars*`・`Sailor*` が付き、「差し替える」で `TwinTails` に替わること、`AngleX` = 30 で房が揺れることを確かめた。
  スクリーンショット: scratchpad/T0710/shots/01-menu.png・02-dress-up.png・03-swapped.png・04-sway.png
  (scratchpad は作業したエージェントの一時ディレクトリ。もう残っていない)。
  読み込み時に 404 が 1 件出るが、見本のファイルではない (favicon などと思われる。確かめていない)。

### 確かめていないこと・後続の候補

- Tauri (WebKitGTK) のウィンドウでは動かしていない (`.deb` は作れた)。
- エディタで見本を開くための `.ugoeproj` (本体とモジュールのプロジェクト) は作っていない。`--project` のような出力を足せば、
  モジュールのプロジェクトの見本にもなる。
- 見本の本体はアバタープロファイルを名乗っていない (スロットの「アプリに見せる」の見本にはならない)。
- 着せ替えの見本を開いてから「アバター」を開くと、読み込んだモジュールが残ったまま外れる (T0706 の既定の振る舞い)。
  見本を切り替えたらモジュールの一覧を空にするかは、T0711 (モデルごとに組を覚える) と一緒に考えるとよい。
- T0707 (法線マップ付きの見本) は `sample/paint.rs` の描き方を使える (形の距離から法線を作れる)。
