# 移植性とエンジン組み込み

## 方針

ランタイムは次の 3 段階で組み込めるようにします。深い段階ほど手間がかかりますが、ホスト側の自由度が上がります。

| 段階 | 使うもの | 向いている場面 |
| --- | --- | --- |
| A. 全部入り | `ugoe` (既定の feature。参照レンダラ `ugoe::render` = `ugoe-render` (wgpu) を含む) | Rust 製のアプリ・Bevy 以外の wgpu アプリ・Web (wasm) |
| B. 評価だけ | `ugoe` (+ C ABI) で頂点を取り出し、描画はエンジンで行う。Rust から使うなら `default-features = false` で参照レンダラを外せる ([下](#ugoe-クレートの-feature)) | Unity、Unreal、Godot、Bevy (wgpu のバージョンが合わない場合) |
| C. 再実装 | 仕様書 (`docs/spec/`) だけを使う | Rust を持ち込めない環境 |

**段階 B を第一級に扱います。** ゲームエンジンは自前の描画パイプライン (ソート・バッチ・ポストエフェクト) を持つので、描画はエンジンに任せ、ugoe はエンジンに頂点と状態を渡すのが最も組み込みやすいためです。
そのため `ugoe-runtime` の出力 (`DrawableState`) は、GPU の知識なしで使える素直なデータにしています。描画の式は [spec/rendering.md](spec/rendering.md) が正であり、参照シェーダ (WGSL) は naga で HLSL / GLSL / MSL / SPIR-V に変換できます。

## ターゲット別

| ターゲット | 方式 | 備考 |
| --- | --- | --- |
| Windows / macOS / Linux | 段階 A (Vulkan / Metal / DX12 / GL) | |
| iOS / Android | 段階 A または B | `ugoe-ffi` を staticlib / cdylib としてビルドする |
| Web | `ugoe-wasm` (WebGPU、なければ WebGL2) | npm パッケージとして配布する (M4) |
| Unity | 段階 B: `ugoe-ffi` + C# (P/Invoke) + Unity の Mesh / Material | 描画はエンジンのシェーダで行う。[bindings/unity/README.md](../bindings/unity/README.md) |
| Unreal | 段階 B: `ugoe-ffi` + C++ プラグイン + ProceduralMesh または独自の SceneProxy | |
| Godot | 段階 B: GDExtension (godot-rust で `ugoe` を直接使う。`bindings/godot`) | 描画は Godot の 2D (キャンバスアイテムとシェーダ)。ワークスペースの外に置く ([ADR 0026](adr/0026-godot-binding.md)) |
| Bevy | 段階 A (wgpu のバージョンが一致する場合) または B | Bevy が使う wgpu のバージョンに合わせた ugoe-render のリリースを検討する |

## `ugoe` クレートの feature

| feature | 既定 | 内容 |
| --- | --- | --- |
| `render` | 有効 | 参照レンダラ (`ugoe-render`、wgpu) を `ugoe::render` として再エクスポートする |

段階 B で Rust から `ugoe` を使う組み込み先 (自前のエンジン・godot-rust など) は、`default-features = false` で `render` を外すと
wgpu と naga をビルドしません。`Puppet` と評価・物理・モーション・読み込み (`ugoe::format`) はそのまま使え、無くなるのは `ugoe::render` だけです。
光源の型 (`Lighting`) も `ugoe::render` にあるので、外したときはホストが自分の形で持ちます (`ugoe::lighting` の `uses_lighting`・`bundle_normal_maps` は使える)。

```sh
cargo add ugoe --no-default-features
```

既定で有効にしているのは、`cargo add ugoe` だけで段階 A まで使えるようにするためと、今の利用者 (`ugoe-wasm`・`ugoe-headless`・
`ugoe-ffi`・`bindings/godot`) のビルドを変えないためです。`ugoe-render` は外せても crates.io には公開します ([ADR 0027](adr/0027-crates-io-publishing.md)。
省ける依存も crates.io に無いと `ugoe` を公開できないため)。外したビルドは CI の `rust (runtime)` で `just sdk-no-render-check` が検査します
(clippy とテストと、`cargo tree` に wgpu・naga が無いこと。T0156)。

## C ABI (`ugoe-ffi`) の規約

- 公開する関数はすべて `ugoe_` で始めます。
- オブジェクトは不透明なポインタで渡し、`ugoe_*_create` / `ugoe_*_destroy` で寿命を管理します。
- 関数はエラーコード (`UgoeResult`、`i32`) を返し、詳細なエラーメッセージはスレッドローカルの `ugoe_last_error()` で取得できるようにします。
- パニックは `catch_unwind` で捕まえてエラーコードに変換します。FFI の境界の外にパニックを漏らしません。
- 頂点データは、ホストが確保したバッファにコピーする API (`ugoe_puppet_copy_positions(p, drawable, out_ptr, len)`) と、内部のバッファを借用する API (次の `update` まで有効) の両方を提供します。
- ヘッダファイルは cbindgen で生成して `crates/ugoe-ffi/include/ugoe.h` に置き、CI で最新かどうかを検査します (`just ffi-header` で作り直し、`just ffi-check` で検査と C の見本の実行)。cbindgen は MPL-2.0 なので、ワークスペースの依存には入れず、版を固定した CLI として使います。`just ffi-check` は `just ci` に含めません ([crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md#ヘッダの生成))。
- `ugoe_abi_version()` の値は、ABI を壊す変更をしたら上げます。関数・型・定数・エラーコードを足すだけなら上げません
  (既存の構造体にフィールドを足すのは壊す変更なので、別の関数や構造体を足します。[crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md#abi-の版))。
- 可変長の結果は、ホストのバッファ (`out`・`capacity`) に書いて数を返し、足りなければ `UGOE_ERROR_BUFFER_TOO_SMALL` で数だけを返します。
- 着せ替えなど、失敗の種類を機械が読む必要がある関数は、`ugoe_last_error_kind()` と `ugoe_last_error_field(key)` で wasm と同じ `code` と補足の値を返します。
  読み込みの警告も wasm と同じ `code` です。どちらも値は SDK (`ugoe::DressUpError::code`・`ugoe::LoadWarning`) が決め、バインディングはそれを写すだけにします (T0412)。
- 外から来るファイルの読み込みの上限 (展開するバイト) は、組み込む側が `UgoeReadLimits` で指定できます (指定しない関数は既定値)。
  超えたら `UGOE_ERROR_LIMIT_EXCEEDED` で、種類 `read_limit_exceeded` と補足の値は wasm と同じです (T0152。`docs/spec/file-formats.md#読み込みの上限`)。

## ホストから来る数

ホストが渡す数 (`dt`・パラメータの値・リップシンクの音量・受け手の入力・トラッキングの値・光源・座標など) が NaN・無限大・負・範囲外のときの扱いです (T0010。
M9 のコードレビューで、入口ごとに拒む・無視する・0 にするがばらばらだったものをまとめた。[code-review.md](workflow/code-review.md#m9-の結果))。
公開仕様の対応する規則は [spec/motion.md#host-supplied-numbers](../spec/motion.md#host-supplied-numbers)。

**方針: 前提は下の層で決め、上の層は表し方だけを持つ (T0139)。**

- 何を受け付け、受け付けない値をどう扱うかは、下の層 (`ugoe-runtime` と `ugoe::Puppet`) が決める。下の層は、受け付けない値を「書かない」「0 とみなす」
  「使わない」のどれかにして、パニック・無限ループ・NaN の居座り (一度入ると戻らない状態) を起こさない。Rust から直接使っても安全。
- 上の層 (C ABI・wasm・Godot・Unity) は、下の層が受け付けない値を **エラーとして知らせる** (何も変えずに失敗を返す) か、**下の層の扱いをそのまま出す** かだけを選ぶ。
  上の層が下の層と違う前提 (別のクランプや別の既定値) を持たない。上の層の検査を抜けた値も下の層が安全にするので、検査の漏れが壊れにつながらない。
- 表し方の流儀: C ABI は結果コードで知らせる (`UGOE_ERROR_INVALID_ARGUMENT`)。Unity はそれを例外 (`UgoeException`) にする。Godot は C ABI に揃え、
  `godot_error!` を出して前の値を保つ (戻り値のある関数は false)。wasm は毎フレームやスライダーから呼ばれる口 (`update`・`set_parameter`・
  `set_lip_sync_level`) では下の層の扱いをそのまま出し (例外でホストのループを止めないため)、受け手の口 (`follow`・`set_base_values`) では例外にする
  (理由は [crates/ugoe-wasm/README.md](../crates/ugoe-wasm/README.md#有限でない値と-dt))。
- 新しい入口を足すときは、下の層の扱いをこの表で確かめ、上の層では検査を写さずに結果を表すだけにする。下の層に無い前提が要るなら、まず下の層に足す。

### 下の層の扱い

| 値 | 入口 (Rust) | NaN | ±無限大 | 負・範囲の外 |
| --- | --- | --- | --- | --- |
| `dt` | `Puppet::update`・`Puppet::follow` | 0 とみなす (時間を進めずに各段を当てて評価する) | 0 とみなす | 負は 0 とみなす |
| `dt` (再生器を直接使う) | `ugoe_motion` の `MotionPlayer`・`ExpressionPlayer`・`EyeBlinkPlayer`・`BreathPlayer` の `advance`、`PhysicsSystem::update` | 何も進めない | 何も進めない | 何も進めない |
| パラメータの値 | `ParameterValues::set`・`blend` (ホストの書き込み、モーション・表情・目パチ・呼吸・物理・リップシンクの書き込みもここを通る) | 書かない (前の値のまま) | 値域の端にクランプ | 値域にクランプ |
| 基準値の列 | `Puppet::set_base_values`・`Puppet::follow` の `values` | エラー (何も変えない) | 値域にクランプ | 値域にクランプ。数が違えばエラー |
| リップシンクの音量 | `Puppet::set_lip_sync_level` | 音量なし (`None` と同じ) | 0〜1 にクランプ | 0〜1 にクランプ |
| 受け手のリップシンクの音量と強さ、パーツの不透明度の値と重み | `Puppet::follow` | エラー | エラー (強さが無限大だと `lerp` が `0 × ∞` で NaN を書くため) | 音量は 0〜1 にクランプ。重みは 0〜1 にクランプ |
| パーツの不透明度の上書き | `ModelInstance::set_part_opacity_override` | 書かない (前の上書きのまま) | 書かない | 重みは 0〜1 にクランプ |
| 動的物理の加算量 | `DynamicOffsets` (`ModelInstance::dynamic_offsets_mut`) | 評価で 0 とみなす (書いた値は残る) | 0 とみなす | (意味どおり) |
| トラッキングの入力 | `Puppet::set_tracking` (`TrackingInput`) | 式の値が有限でないパラメータには書かない ([avatar-profile.md](spec/avatar-profile.md#フェイストラッキングとの対応)) | 同じ | 値域にクランプ |
| 当たり判定の点 | `Puppet::hit_test`・`hit_areas_at` | 当たらない | 当たらない (範囲の外) | (意味どおり) |
| 光源 | `ugoe_render` の `Lighting` | その光源を使わない ([rendering.md](spec/rendering.md#ライティング)) | 同じ。`range` の無限大は減衰なし | `range` が正でなければ使わない |
| モーション・表情 (検証していないもの) | `Puppet::start_motion`・`Puppet::set_expression` | パラメータへは書かない。パーツの不透明度のカーブは使わない。リップシンクの強さが有限でなければリップシンクを当てない。長さの NaN は 0 と同じ | パラメータへは値域にクランプ。パーツの不透明度のカーブとリップシンクの強さは NaN と同じ | 値域にクランプ |
| エフェクトの設定 (検証していないもの) | `Puppet::set_effects` | 目パチの `interval` が 0 より大きくなければまばたきしない。呼吸の `cycle` が 0 以下なら `sin` の項は 0 ([motion.md](spec/motion.md#エフェクト)) | 同じ | 同じ |
| 物理の状態と出力 | `PhysicsSystem` | 鎖を静止姿勢に置き直し、出力を書かない ([physics.md](spec/physics.md#有限でない状態)) | 同じ | — |

- ファイルから読むモデル・物理・モーション・表情の数は、読み込みで有限かを検証する (`ugoe-format`、[model.md#検証](spec/model.md#検証))。
  上の「検証していないもの」は、Rust から組み立てた値や、検証しない JSON の口 (`set_effects_json` など) から来たもの。
- `Puppet::update` でモーションのパーツの不透明度のカーブとリップシンクの強さを使わない値は、指揮役が配る記録
  (`part_opacities_applied`・`lip_sync_applied`) にも入れない。受け手の `follow` は有限でないものを拒むので、指揮役の出力はいつも受け手に通る
  (`crates/ugoe/tests/host_numbers.rs`)。

### 入口ごとの表し方

「拒む」は、何も変えずに失敗を返す (C ABI は `UGOE_ERROR_INVALID_ARGUMENT`、Unity は `UgoeException`、Godot は `godot_error!` と false、wasm は例外)。
空欄は下の層と同じ。Unity の `Ugoe.Puppet` は C ABI をそのまま呼ぶので C ABI の列と同じ (違うものだけ書く)。

| 値 | C ABI (`ugoe-ffi`)・Unity | wasm (`WebPuppet`、`@ugoe/runtime`) | Godot (`UgoePuppet` ノード) |
| --- | --- | --- | --- |
| `dt` (`update`) | 負・NaN・無限大を拒む (`ugoe_puppet_update`)。Unity の `UgoePuppet` の毎フレームの進め方は、`Time.deltaTime × speed` が正の有限でなければ 0 で進める | 下の層と同じ (0 とみなす) | `advance(delta)` は負・NaN・無限大を拒む。`speed` の setter も負・有限でない値を拒む |
| パラメータの値 | NaN を拒む (`ugoe_puppet_set_parameter`・`set_parameters_by_id`)。無限大はクランプ | 下の層と同じ (`set_parameter` は書かずに `true`)。`<ugoe-avatar>` の `parameters` 属性は有限でない値の組を無視する | `set_parameter` は NaN を拒む。無限大はクランプ |
| リップシンクの音量 | NaN を拒む。負 (`-∞` を含む) は入力をやめる (`None`)。`+∞` は 1 | 下の層と同じ (NaN は `undefined` と同じ) | `set_lip_sync_level` は NaN を拒む。負は 0 (入力をやめるのは `clear_lip_sync_level`) |
| 基準値の列・受け手 (`follow`) | 下の層の拒否を結果コードにする。加えて `dt` の負・NaN・無限大を拒む。音量が負ならリップシンクを当てない | 下の層の拒否を例外にする。`dt` は 0 とみなす | 下の層の拒否をエラーにする。加えて `delta` の負・NaN・無限大を拒む。音量が負ならリップシンクを当てない |
| トラッキング | 頭の回転とブレンドシェイプが有限でなければ拒む。体と肩は NaN を省略とみなし、無限大を拒む (`UgoeTrackingInput`) | 下の層と同じ (書かない) | `set_tracking_head` は有限でなければ拒む。`set_tracking_body` は C ABI と同じ。ブレンドシェイプは有限でない値を数えずに飛ばす (`set_tracking_blend_shape` は false) |
| 光源 | 向きが 0・有限でない値・`range` が正でない・負の色を拒む (`ugoe_lighting_uniform`) | `set_lighting_json` は同じ条件で例外 | `ambient_light` は色が有限で負でなければ受け付け、そうでなければ拒む。光源は C ABI と同じ条件で、通らないものは警告を出して使わない |
| 当たり判定の点 | 下の層と同じ (当たらない) | 同じ | 同じ |
| 表示 (wasm のビュー・Godot のマスク) | — | `set_view`・`pan_by`・`zoom_at`・`set_background` は有限でなければ何も変えない ([wasm の表](../crates/ugoe-wasm/README.md#有限でない値と-dt)) | `mask_scale` は有限でなければ拒む |
| モーション・表情・エフェクト・物理の JSON | 読み込みの検証に従う (`ugoe_puppet_set_physics_json` は検証して拒む。`set_effects_json` は検証しない) | 同じ (`start_motion_json`・`set_expression_json` は検証して例外) | 同じ |

- C ABI が拒む値を下の層は「書かない」「0 とみなす」で受け付けるのは、上の層の表し方の違いで、前提の違いではない。C ABI は結果コードで知らせる流儀なので、
  ホストの誤りを早く見つけられるよう拒む (T0423)。
- 確かめるテスト: 下の層は `ugoe-runtime` (`nan_writes_are_ignored`・`non_finite_part_opacity_overrides_are_ignored`・`non_finite_offsets_are_not_added`)、
  `ugoe` (`crates/ugoe/tests/host_numbers.rs`・`follow_rejects_non_finite_inputs`)、`ugoe-motion` と `ugoe-physics` の各テスト (T0317・T0320)。
  上の層は `crates/ugoe-ffi/tests/`、`crates/ugoe-wasm/src/animation.rs`、Godot の smoke (`example/tests/smoke.gd`)、Unity の `tests/ReceiverTests.cs`・`FrameStateTests.cs`。

## ランタイム層が守る制約 (移植性のため)

- wasm32-unknown-unknown でビルドできること (`just wasm-check`)。
- `std::fs`、`std::thread`、`std::time` を使わないこと。時間は `dt` として受け取ります。
- グローバルな状態 (static mut、グローバルなロガーの初期化など) を持たないこと。
  例外: 出力に影響しない一意な ID を振るためのアトミックなカウンタ (`ModelInstance::id()`) は認める。初期化が不要で、評価結果の決定性にも影響しないため。
- 重い依存を追加しないこと。追加する場合は ADR を書きます。

> 未決: 組み込み機器向けに `no_std + alloc` に対応するか。`ugoe-core`、`ugoe-model`、`ugoe-runtime` は対応できる設計にしておき、需要があれば対応する。

## 配布

判断の経緯は [ADR 0017](adr/0017-single-runtime-distribution.md)。すべての配布物を 1 つのコア (`ugoe` クレート) から作り、主要な環境に 1 行で導入できるようにします。

| 環境 | 配布物 | タスク |
| --- | --- | --- |
| Rust | crates.io の `ugoe` | T0403 |
| Web | npm の `@ugoe/runtime` と Web Components の `<ugoe-avatar>` | T0404 |
| Unity | UPM パッケージ ([bindings/unity](../bindings/unity/README.md)) | T0405 |
| Godot | GDExtension (`bindings/godot`。ワークスペースの外、[ADR 0026](adr/0026-godot-binding.md)) | T0406 |
| Unreal | プラグイン | T0407 |
| C / その他 | `ugoe-ffi` の共有ライブラリとヘッダ | T0401 |

- **適合テスト**: すべての配布物で、同じモデル・同じパラメータから同じ頂点と画像が得られることを CI で検査します (T0408、`just conformance`)。参照となる出力は `ModelInstance` と `ugoe render` から作ります。いまは C ABI・wasm (npm パッケージ。WebGL2 と WebGPU)・Unity の C# の層 (頂点)・Godot (画像) を比べています。バインディングを足すときの手順は [tests/conformance/README.md](../tests/conformance/README.md#エンジン向けのバインディングを足す)。
- **リリースの自動化**: タグを打つと、すべての配布物をビルドして公開します。バージョン番号は揃えます。手順は [release.md](release.md)。
- crates.io には `ugoe` とその依存のクレートだけを公開します (`ugoe-ffi`・`ugoe-wasm`・ツール層は公開しない。[ADR 0027](adr/0027-crates-io-publishing.md))。タグ `v*` で CI (`crates-release.yml`) が予行し、承認の後に公開します ([release.md#cratesio](release.md#cratesio))。
- ランタイムとライブラリは MIT で配布します ([ADR 0018](adr/0018-licensing-and-1.0-scope.md))。
- Web の `@ugoe/runtime` は `bindings/web/` に置き、MIT で配布します。GPL-3.0-only の `packages/` (アプリが共有する部品) とは分けます ([ADR 0025](adr/0025-web-runtime-package.md))。使い方は [bindings/web/README.md](../bindings/web/README.md)、npm への公開の手順 (タグ `v*` で CI から公開する `web-runtime-release.yml`) は [bindings/web/README.md#リリース](../bindings/web/README.md#リリース)。
- Unity の UPM パッケージは `bindings/unity/` に置き、MIT で配布します。タグ `v*` で CI (`unity.yml`) が各プラットフォームのライブラリを入れた tarball を作って成果物に残し、配る場所に置くのは手で行います ([bindings/unity/README.md#リリース](../bindings/unity/README.md#リリース))。
- Godot の GDExtension は `bindings/godot/` に置き、MIT で配布します。依存する godot-rust だけが MPL-2.0 なので、ルートのワークスペースの外に置きます ([ADR 0026](adr/0026-godot-binding.md))。使い方は [bindings/godot/README.md](../bindings/godot/README.md)。

## 前方互換

運用中のアプリは、ランタイムを更新しなくても新しいファイルを読めます。知らない機能は、仕様が定めるフォールバックで簡易な表示に落ちます ([ADR 0013](adr/0013-forward-compatibility.md)、[spec/file-formats.md#機能の宣言と段階的な劣化](spec/file-formats.md#機能の宣言と段階的な劣化))。

