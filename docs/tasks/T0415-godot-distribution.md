---
id: T0415
title: Godot: Light2D と XRFaceTracker からの入力・マスクの解像度
status: done
milestone: M7
crates: [bindings/godot, docs]
depends_on: [T0411]
assignee: claude-worker-T0415
---

# T0415: Godot の配布と入力の後続

## 目的

T0411 の後続の候補をまとめる。

## 受け入れ条件

- 配布の自動化 (各 OS のビルドとアセットライブラリ向けの zip) は、公開のマイルストーン (Release) の T0421 に移した
- [x] Godot の `Light2D` から光源を取る (ノードのオプション)
- [x] Godot の `XRFaceTracker` からトラッキングの入力への対応表
- [x] アトラスに収まらずにマスクの解像度を下げたとき、元に戻す判断 (戻す条件と、画質の目安)
- [x] 画面のある Godot エディタでの確認 (インスペクタで UgoeModel を選べるか、modules と lights の配列の編集、書き出したゲームに .ugoe が含まれるか) は、できなければ利用者の確認に残す
- [x] `just ci`・`just godot-check` が通る

## 参考

- `docs/tasks/T0411-godot-followups.md` のメモ、`docs/adr/0026-godot-binding.md`

## メモ

### やったこと (すべて `bindings/godot` の中。範囲外の変更はない)

- **Light2D:** `UgoePuppet2D.use_scene_lights` (Lighting のグループ) を true にすると、同じビューポートの `PointLight2D` /
  `DirectionalLight2D` を毎フレーム読み、キャンバス座標の光源に直して `lights` の後ろに足す。`get_scene_lights()` で結果を
  辞書で見られる。ライトの足し外しは木の `node_added` / `node_removed` で気づく (受け取る関数はノードを借りない
  `Callable::from_linked_fn`。自分の中でマスクのビューポートを足したときにも呼ばれるため)。光源が変わらなければシェーダに
  渡し直さない。使う条件・換算 (色 × energy、点光源の `height` と、テクスチャの半径からの `range`、平行光源の +Y と `height`)・
  使わないもの (テクスチャの形・影・`blend_mode`・Z と層の範囲) は README の「Light2D から取る光源」。
  - 確かめたこと: smoke で位置・奥行き・range・色の換算 (ノードの移動と拡大を含む)、後から足したライト・無効・cull mask・外したライト。
    Xvfb の描画で、Light2D の光源と、同じ値を `add_point_light` で渡した光源が画素まで一致 (差 0/255)、光源なしとは 223/255 違う。
- **XRFaceTracker:** `xr_face_tracker` (Tracking のグループ。`XRServer` の名前、毎フレーム読む) と
  `set_tracking_from_xr_face_tracker(tracker)`。Unified Expressions (143 個) → ARKit の 52 個の対応表を `src/xr.rs` と README に
  書いた (左右は入れ替えない。分かれた形は平均か最大、合成の形は書かれていれば大きい方)。`XRFaceTracker` は godot-rust では
  実験的な API (`experimental-godot-api` の機能が要る) なので、機能を足さずに `get_blend_shapes` を名前で呼んでいる。
  - 確かめたこと: Rust のテスト (52 個の名前の順・左右・平均と最大・合成の形・クランプ) と、smoke で `XRFaceTracker.new()` に値を書いて
    渡す・`XRServer.add_tracker` して名前で毎フレーム読む (`MouthOpenY`・`EyeLOpen`・`EyeROpen`・`MouthForm` に届く)。
- **マスクの解像度を戻す判断:** `src/model.rs` の `MaskShrink`。収まらなければ 0.75 倍ずつ下げ、下げている間にマスクが動いたフレームで、
  1 段上げた配置が 1 辺 3584 (4096 の 7/8) に収まるなら 1 段戻す (余裕は境目での上げ下げの繰り返しを防ぐため。戻した直後に下げ直すのは
  外接矩形がおよそ 8/7 倍に広がったときだけ)。`mask_scale` か画面上の大きさが変わったら 1 から測り直す (今までどおり)。
  下げるときも、今の大きさの使い回しを外して今の倍率で収まるかを先に試すようにした。`get_mask_resolution_factor()` で倍率を見られる。
  - 画質の目安: 適合テストのマスクのケースを `mask_scale` を下げて描き、`ugoe render` と比べた (一時的に conformance.gd を変えて測り、
    戻した)。違うのはマスクの縁だけ。

    | 倍率 | 3/255 を超える画素 (masks-many 以外) | masks-many | 最大の差 |
    | --- | --- | --- | --- |
    | 1 | 0% | 0% | 2/255 |
    | 0.75 | 0.1〜0.4% | 3.0% | 14〜41/255 |
    | 0.5625 | 0.2〜0.5% | 2.9% | 24〜92/255 |
    | 0.42 | 0.3〜0.5% | 3.9% | 16〜130/255 |
    | 0.32 | 0.4〜0.6% | 5.9% | 15〜123/255 |
    | 0.24 | 0.5〜0.9% | 7.1% | 43〜138/255 |

    0.75 は縁が 1 画素ほどぼける程度、0.42 以下では縁の段差が見える。README の「描画のしかた」に要約した。
  - 確かめたこと: Rust のテスト (下げる・余裕に収まらないうちは戻さない・1 段ずつ戻す)。見本のモデルはマスクが少なく、
    1 枚ずつは `MaskArea::fit` が上限に収めるので、Godot の上で実際に下がるところは smoke では作れなかった (倍率の取得だけ確かめた)。
- **書き出し:** 見本のプロジェクトを scratchpad に写し、`godot --headless --export-pack` で .pck を作ると、`models/*.ugoe`・`*.ugoemod`・
  読み込み器 (`.gdc`)・`ugoe.gdextension` が含まれていた。書き出したゲームを動かすことはしていない (書き出しのテンプレートは mono しか無い)。

### 確かめたこと

- `just ci` (2 回 rebase した後)。`just godot-check` (Godot 4.7.1、smoke と比べ合わせ 20 ケース、すべて 100%)。
  `cargo clippy --all-targets -D warnings` と `cargo test` (bindings/godot、27 件)。

### 確かめていないこと (利用者の確認に残す)

- 画面のある Godot のエディタでの操作: インスペクタで `model` に `UgoeModel` を選べるか、`modules` と `lights` の配列の編集、
  `use_scene_lights` を入れてエディタの中でライトを動かしたときに陰影が付いてくるか、`xr_face_tracker` の入力欄。
  書き出したゲーム (テンプレートで作った実行ファイル) で `.ugoe` を読めるか (.pck に含まれることだけ確かめた)。
- 実機の `XRFaceTracker` (OpenXR のフェイストラッキングの拡張など)。トラッカーが合成の形だけを書く場合や、値の向き。
- Forward+ / Mobile (Vulkan) での見た目 (T0411 から変わらず)。

### 後続の候補

- `Light2D` のテクスチャの形を減衰に使う (シェーダでライトのテクスチャを読む)。`blend_mode` の SUB / MIX、影。
- 頭の回転を `XRBodyTracker` (頭の関節) や HMD の向きから取る (`XRFaceTracker` に頭の回転が無いため)。
- マスクが多く大きいモデルで Godot の上の下げ・戻しを確かめる見本 (適合テストの計画に `mask_scale` を足すなど。tests/conformance の範囲)。
