---
id: T0411
title: Godot: CI での検査・ライティングと着せ替えとトラッキング・リソースとしての取り込み
status: done
milestone: M7
crates: [bindings/godot, docs, justfile, scripts, .github/workflows, AGENTS.md]
depends_on: [T0406]
assignee: claude-worker-T0411
---

# T0411: Godot の後続

## 目的

T0406 の後続の候補をまとめる。

## 受け入れ条件

- [x] CI で `node bindings/godot/build.mjs --check` と `--conformance` を回す (Godot と Xvfb を入れる。版を固定する)。範囲外の ci.yml の変更は team-lead に相談する
- [x] ライティング (法線マップ・光源)・着せ替え (.ugoemod の付け外し)・トラッキングの入力をノードに出す
- [x] `.ugoe` を Godot のリソースとして取り込む (ResourceFormatLoader)
- [x] 1 要素あたり 8 つまでのマスクの上限をなくす。アニメーション中に入れ子のマスクが 1 フレーム遅れないかを確かめる
- [x] Godot のアセットライブラリでの配布の準備 (手順だけ。公開は利用者が行う)
- [x] Vulkan のレンダラでの見た目、Windows・macOS でのビルドは、できなければ利用者の確認に残す
- [x] justfile に godot-build / godot-check を足す (Godot が無ければ入れ方を案内して止める。just ci には入れない)。bindings/godot/check-licenses.mjs を scripts/check-licenses.mjs の小さな関数に統合するかを決め、ADR 0026 の検査の方法を揃える
- [x] `just ci` と `node bindings/godot/build.mjs --check` が通る

## 参考

- `docs/tasks/T0406-godot-extension.md` のメモ、`docs/adr/0026-godot-binding.md`

## メモ

### 範囲外の変更 (team-lead の承認済み。crates: に足した)

- `justfile`: `godot-build` / `godot-check` を足した (`just ci` には入れていない)。Godot が無ければ `build.mjs` が入れ方を示して止まる。
- `scripts/check-licenses.mjs`: `--godot` の分岐を足し、`bindings/godot/check-licenses.mjs` を消した (統合した。引数なしの検査は変わらない。
  共通の `runtimeDeps`・`cargoMetadata` に切り出しただけ)。
- `.github/workflows/ci.yml`: ジョブ `godot` (ubuntu-24.04、Godot 4.7.1 の公式ビルドを SHA-512 で確かめる、Xvfb・Mesa は 24.04 のもの、
  `just godot-check`)。
- `AGENTS.md`: ライセンスの検査の場所 (`bindings/godot/check-licenses.mjs` → `node scripts/check-licenses.mjs --godot`) の 1 語だけ直した
  (統合の結果、古いパスを指したままになるため)。

### やったこと

- ノード `UgoePuppet2D` に出した:
  - ライティング: `ambient_light`・`lights` (辞書の配列。シーンに保存)・`add_directional_light`・`add_point_light`・`clear_lights`・
    `uses_lighting`。法線マップはバンドルとモジュールの `NRML` をアルファを掛けずに読む。シェーダは参照レンダラと同じ式
    (ヤコビ行列は `dFdx`/`dFdy`、光源は 8 個まで)。
  - 着せ替え: `modules` (モジュールのリソースの配列。シーンに保存し、読み込むときに付ける)・`attach_module` (同じスロットか名前は差し替え)・
    `detach_module`・`clear_modules`・`get_module_names`。付け外しではレンダラを作り直し、`DressUpChange` で変わっていないテクスチャは
    デコードし直さない。
  - トラッキング: `set_tracking_head` (Vector3 の度)・`set_tracking_body` (辞書)・`set_tracking_blend_shapes` (辞書、MediaPipe の名前も可)・
    `set_tracking_blend_shape`・`clear_tracking`・`is_tracking`・`get_tracking_blend_shape_names`。モデルを読み直しても保つ。
- リソース: `.ugoe` → `UgoeModel`、`.ugoemod` → `UgoeModule` (`data` にバイト列)。読み込み器 (`ResourceFormatLoader`) とリソースは
  **GDScript** で書いた (`example/addons/ugoe/`)。最初は Rust で書いたが、Godot は読み込み器をワーカースレッド (エディタの走査など)
  から呼び、godot-rust はメインスレッド以外からの呼び出しでパニックした (`experimental-threads` は godot-rust 自身が健全でないとしている)。
  ノードの `model_path` をやめて `model` (リソース) にした (`load_model(path)` と `load_model_from_buffer` は残した)。
  拡張を `example/addons/ugoe/` に移した (アセットライブラリで配る形)。ADR 0026 の決定 5。
- マスクの上限をなくした: カバレッジを 1 枚のアトラスに棚詰めで並べ、要素は浮動小数点のテクスチャの表 (`MaskTable`) から読むマスクと
  アトラスでの場所を `texelFetch` で読んでループする (サンプラは 1 つ)。入れ子は段ごとのアトラスにし、段 k − 1 のビューポートを
  段 k のビューポートの子にした (Godot は子のビューポートを先に描く)。マスク用メッシュごとの `SubViewport` はなくなった。
  カバレッジの画素の格子をキャンバスの格子に揃えた (揃えないと、着せ替えの見本でマスクの縁が最大 18/255 ずれた)。
- 1 フレーム遅れの確認: `moving-masks` (入れ子のマスクをパラメータで 80px 動かす) と `avatar-moving` で、値を変えた直後のフレームを
  `ugoe render` と比べる。どちらも 100% (最大 2/255)。
- 比べ合わせ (`build.mjs --conformance`) に足した: `many-masks` (1 要素が 12 個のマスク)・`moving-masks`・`avatar-moving`・
  `avatar-lit` (環境光・平行光源・点光源)・`dress-up` (3 つのモジュール)。参照の画像は `ugoe render --time 0` にした
  (Godot 側は必ず `Puppet::update(0)` を通すので、物理を含めて揃う。`--time` なしだと着せ替えのモジュールの物理の分だけずれた)。
  10 モデルすべて 100% の画素で 3/255 以内 (最大 2/255)。
- 配布の手順を `bindings/godot/README.md#配布-godot-のアセットライブラリ` に書いた (公開は利用者)。見本のシーンはモデルの切り替え・
  モジュールのチェックボックス・マウスに付いてくる点光源を足した。

### 確かめたこと

- `just ci` (rebase 後)。`cargo test` / `clippy -D warnings` / `fmt` (bindings/godot)。
- この環境の Godot 4.7.1 で `just godot-check`: smoke 54 項目 (リソースの読み込み・シーンへの保存と復元 (モデル・パラメータ・モジュール・
  光源)・トラッキングがパラメータに届く・着せ替えの付け外しと差し替え・壊れたファイル) と比べ合わせ 10 モデル。
- CI のジョブを手元でなぞった: 公式の zip を落として SHA-512 を確かめ、`DISPLAY` なし・`WGPU_BACKEND=gl`・`LIBGL_ALWAYS_SOFTWARE=1`
  で `just godot-check` が通る (xvfb-run の経路)。GitHub Actions の上では実行していない。
- 見本のシーンを `--write-movie` で撮った (着せ替え 3 つ・ライティングのモデル・既定)。

### 確かめていないこと (利用者の確認に残す)

- 画面のある Godot のエディタでの操作: インスペクタで `model` に `UgoeModel` を選べるか (ヒントにスクリプトのクラス名を使っている)、
  `modules` と `lights` の配列の編集、ファイルシステムのドックでの `.ugoe` の表示、書き出したゲームで `.ugoe` が含まれるか。
- Forward+ / Mobile (Vulkan) での見た目。Windows・macOS でのビルド (`.gdextension` の設定はある)。

### 後続の候補

- 配布の自動化: 各 OS のビルド (macOS は universal と署名) と `addons/ugoe` の zip をリリースに載せる (ADR 0017)。アイコンと
  スクリーンショットを用意してアセットライブラリに出す。
- 光源を Godot の `Light2D` から取る (ノードの変形を考えてキャンバス座標に直す)。
- Godot の XR のフェイストラッキング (`XRFaceTracker`、Unified Expressions) から `set_tracking_*` への対応表。
- アトラスが 4096 に収まらずマスクの解像度を下げたとき、`mask_scale` が変わるまで戻さない。要素が多いモデルで問題になれば、段ごとに
  アトラスを分けるか、下げた解像度を戻す判断を足す。
