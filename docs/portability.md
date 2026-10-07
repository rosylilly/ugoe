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
| ゲーム機・組み込み機器 (Rust の `std` が無い環境) | 段階 B: `ugoe-ffi` の `std` の無い形 (`--no-default-features --profile nostd` の staticlib) | アロケータはホストの C ライブラリの `malloc`・`free` など、パニックは `abort`。ヘッダは同じ。PNG と BCn の KTX2 のデコードは無い。検査は `aarch64-unknown-none` でのリンクまでで、実機は確かめていない ([crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md#std-の無い環境)、[ADR 0031](adr/0031-no-std-runtime-and-host-neutral-c-abi.md#std-の無い環境への-c-abi)) |

## `ugoe` クレートの feature

| feature | 既定 | 内容 |
| --- | --- | --- |
| `render` | 有効 | 参照レンダラ (`ugoe-render`、wgpu) を `ugoe::render` として再エクスポートする。`std` を含む |
| `std` | 有効 | 下位のクレート (`ugoe-core`〜`ugoe-motion`・`ugoe-format`) と依存の `std` を有効にする。外しても公開 API は変わらない |

段階 B で Rust から `ugoe` を使う組み込み先 (自前のエンジン・godot-rust など) は、`default-features = false` で `render` を外すと
wgpu と naga をビルドしません。`Puppet` と評価・物理・モーション・読み込み (`ugoe::format`) はそのまま使え、無くなるのは `ugoe::render` だけです。
`default-features = false` は `std` も外し、`ugoe` は `no_std` + `alloc` でビルドされます
([ランタイム層が守る制約](#ランタイム層が守る制約-移植性のため))。`Puppet` の読み込み・評価・物理・モーション・トラッキング・着せ替えは
`std` の無い環境 (ゲーム機・組み込み機器) でもそのまま使えます。数学関数は `std` の有無にかかわらず `libm` なので、出力も変わりません。
`std` のある環境で `std` を足す必要はありません (足しても公開 API と出力は同じ)。`std` を外すと、`ugoe::format::image` で読める画像は
圧縮していない KTX2 だけになります (PNG とブロック圧縮の KTX2 は `ugoe-format` の feature `png` (`std` を含む)。
[crates/ugoe-format/README.md](../crates/ugoe-format/README.md))。
光源の型 (`Lighting`・`Light`・`MAX_LIGHTS`) は `ugoe::lighting` にあり、外しても使えます (`ugoe::render` にも同じ型があります。
`uses_lighting`・`bundle_normal_maps` も同じモジュール。T0166)。

```sh
cargo add ugoe --no-default-features
```

既定で有効にしているのは、`cargo add ugoe` だけで段階 A まで使えるようにするためです。リポジトリの中では、ルートの
`[workspace.dependencies]` の `ugoe` で既定の feature を切り、参照レンダラを使うクレート (`ugoe-wasm`・`ugoe-cli`・スタジオ) だけが
`features = ["render"]` を書きます。`ugoe-ffi` と `bindings/godot` は `render` を使わないので、C ABI の共有ライブラリと GDExtension に
wgpu は入りません (T0166)。`ugoe-render` は外せても crates.io には公開します ([ADR 0027](adr/0027-crates-io-publishing.md)。
省ける依存も crates.io に無いと `ugoe` を公開できないため)。外したビルドは CI の `rust (runtime)` で `just sdk-no-render-check` が検査します
(clippy とテストと、`ugoe` と `ugoe-ffi` の `cargo tree` に wgpu・naga が無いこと。T0156・T0166)。

## C ABI (`ugoe-ffi`) の規約

- 公開する関数はすべて `ugoe_` で始めます。
- オブジェクトは不透明なポインタで渡し、`ugoe_*_create` / `ugoe_*_destroy` で寿命を管理します。
- 失敗しうる関数はエラーコード (`UgoeResult`、`i32`) を返し、最後の引数に呼び出し側の `UgoeError*` を取ります。失敗の中身
  (メッセージ・種類・補足の値) はそこに書き、`ugoe_error_message(e)`・`ugoe_error_kind(e)`・`ugoe_error_field(e, key)` などで読みます。
  NULL を渡せば書きません。成功した呼び出しは中身を変えません ([crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md#失敗の受け取り方-t0489))。
- C ABI はグローバルな状態もスレッドローカルな状態も持ちません ([ADR 0031](adr/0031-no-std-runtime-and-host-neutral-c-abi.md#c-abi-のエラーはスレッドローカルに置かない))。
  スレッドについての約束は次のとおりです。
  - `UgoeError` はスレッドに結び付かないので、どのスレッドで作り、書かせ、読んでもかまいません。呼び出しの間で OS のスレッドが変わるホスト
    (Go の goroutine・C# の async の続き・スレッドプール) でも、失敗した呼び出しに渡した `UgoeError` から正しい中身を読めます。
    ただし 1 つの `UgoeError` を同時に複数のスレッドから使ってはなりません (呼び出しごとか、スレッドごとに 1 つ持ちます)。
  - `UgoePuppet`・`UgoeModule`・`UgoeImage` も同じで、どのスレッドから使ってもよいが、1 つを同時に複数のスレッドから使ってはなりません
    (別々のオブジェクトなら並行してかまいません)。
  - 読み出しの関数が返す文字列は、その `UgoeError` に次の失敗を書くか、破棄するまで有効です。
- C ABI はパニックを起こさないように書きます ([パニックしない](#パニックしない))。そのうえで後ろの守りとして、`std` がある形ではパニックを
  `catch_unwind` で捕まえてエラーコード (`UGOE_ERROR_PANIC`) に変換し、メッセージを `UgoeError` に書きます。FFI の境界の外にパニックを漏らしません。
  `std` の無い形では巻き戻せないので、パニックはホストの `abort` を呼びます。
- `std` の有無で関数の集合とヘッダを変えません。`ugoe-ffi` は既定で有効な feature `std` を持ち、外すと `#![no_std]` (`core` + `alloc`) の
  静的ライブラリになります (ゲーム機・組み込み機器向け)。
  - アロケータ (`#[global_allocator]`) はホストの C ライブラリの `malloc`・`calloc`・`realloc`・`free` (と大きな整列の `aligned_alloc`) を、
    パニックの処理 (`#[panic_handler]`) は `abort` を呼びます。シンボルはホストのリンクで解決し、SDK ごとの知識はクレートに持ちません。
  - `panic = "abort"` はワークスペースの既定のプロファイルを変えず、専用のプロファイル `nostd` (ルートの `Cargo.toml`) で決めます。
  - `std` が要る機能 (PNG とブロック圧縮の KTX2 のデコード。`ugoe_image_decode`) は、関数は残したまま `"unsupported_texture"` で失敗します。
    ヘッダの関数の説明と [crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md#std-の無い環境) に書きます。
  - `just nostd-check` が `aarch64-unknown-none` の静的ライブラリを C の見本とリンクし、未解決のシンボルがホストの C ライブラリのもの
    (`malloc`・`calloc`・`realloc`・`aligned_alloc`・`free`・`abort`) だけであることを確かめます。`just ffi-check` はホスト向けの
    `std` の無い形を C ライブラリとリンクして実行します。実機での確認は組み込み先に任せます (T0491)。
- 毎フレームの描画の状態 (すべての描画可能要素の頂点の位置・表示・不透明度・色・合成モード・マスクの効き具合・描画順の値・変更のフラグと、描画順) は、
  1 回の呼び出しでまとめて渡します。境界を越える呼び出しが重いホスト (cgo・マネージドな言語) でも、呼び出しの数が描画可能要素の数によりません
  ([ADR 0031](adr/0031-no-std-runtime-and-host-neutral-c-abi.md#毎フレームの描画の状態はまとめて渡す))。
  - 頂点の位置は、描画可能要素ごとの範囲 (先頭と数) を持つ 1 つの連続した配列です。変更のフラグも一緒に渡すので、ホストは変わっていない要素の位置を写さずに済みます。
  - 内部のバッファを借用する API (`ugoe_puppet_frame(p, &frame, err)`。次の `update`・着せ替え・破棄まで有効) と、ホストが確保したバッファに
    コピーする API (`ugoe_puppet_copy_frame(p, &buffers, changed_positions_only, &out, err)`。足りなければ `UGOE_ERROR_BUFFER_TOO_SMALL` で要る数を返す)
    の両方を提供します。コピーは、変更のフラグが立った要素の位置だけを写すこともできます。
  - 読み込みと着せ替えの後に 1 回読む情報 (UV・インデックス・テクスチャ・マスクの参照など) は、描画可能要素ごとの関数で読みます
    ([crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md#毎フレームの描画の状態-t0490))。
- ヘッダファイルは cbindgen で生成して `crates/ugoe-ffi/include/ugoe.h` に置き、CI で最新かどうかを検査します (`just ffi-header` で作り直し、`just ffi-check` で検査と C の見本の実行)。cbindgen は MPL-2.0 なので、ワークスペースの依存には入れず、版を固定した CLI として使います。`just ffi-check` は `just ci` に含めません ([crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md#ヘッダの生成))。
- `ugoe_abi_version()` の値は、ABI を壊す変更をしたら上げます。関数・型・定数・エラーコードを足すだけなら上げません
  (既存の構造体にフィールドを足すのは壊す変更なので、別の関数や構造体を足します。[crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md#abi-の版))。
- 可変長の結果は、ホストのバッファ (`out`・`capacity`) に書いて数を返し、足りなければ `UGOE_ERROR_BUFFER_TOO_SMALL` で数だけを返します。
- 着せ替えなど、失敗の種類を機械が読む必要がある関数は、`UgoeError` の `ugoe_error_kind(e)` と `ugoe_error_field(e, key)` で wasm と同じ `code` と補足の値を返します。
  補足の値は `ugoe_error_field_count(e)`・`ugoe_error_field_key_at(e, i)`・`ugoe_error_field_value_at(e, i)` で列挙もでき、バインディングはキーの一覧を写さずに済みます (T0464)。
  読み込みの警告も wasm と同じ `code` です。どちらも値は SDK (`ugoe::DressUpError::code`・`ugoe::LoadWarning`) が決め、バインディングはそれを写すだけにします (T0412)。
- 外から来るファイルの読み込みの上限 (展開するバイト) は、組み込む側が `UgoeReadLimits` で指定できます (指定しない関数は既定値)。
  超えたら `UGOE_ERROR_LIMIT_EXCEEDED` で、種類 `read_limit_exceeded` と補足の値は wasm と同じです (T0152。`docs/spec/file-formats.md#読み込みの上限`)。
- wasm のモデルの読み込み (`load_json` / `load_bundle`) の失敗は、すべて `name` が `"LoadError"` で `code` と補足の値を持ちます (T0434。
  一覧は [crates/ugoe-wasm/README.md](../crates/ugoe-wasm/README.md#読み込みの失敗-t0434))。ファイルとモデルの失敗 (`invalid_file`・
  `unsupported_version`・`invalid_model`・`compile` など) の種類は SDK の `ugoe::LoadError` が決め、C ABI の `ugoe_puppet_create_from_*` も
  `ugoe_error_kind(e)` で同じ値を返します (T0457。[crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md#モデルの読み込みの失敗-t0457))。
  利用者に見せる文は組み込み先が `code` から作ります ([ADR 0029](adr/0029-shared-ui-and-i18n.md))。

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
| 基準値の列 | `Puppet::set_base_values`・`Puppet::follow` の `values` | エラー (`ParameterValuesError::NaN`。何も変えない) | 値域にクランプ | 値域にクランプ。数が違えばエラー |
| リップシンクの音量 | `Puppet::set_lip_sync_level` | 音量なし (`None` と同じ) | 0〜1 にクランプ | 0〜1 にクランプ |
| 受け手のリップシンクの音量と強さ、パーツの不透明度の値と重み | `Puppet::follow` | エラー (`ParameterValuesError::NonFiniteLipSync`・`NonFinitePartOpacity`。何も変えない) | NaN と同じエラー (強さが無限大だと `lerp` が `0 × ∞` で NaN を書くため) | 音量は 0〜1 にクランプ。重みは 0〜1 にクランプ |
| パーツの不透明度の上書き | `ModelInstance::set_part_opacity_override` | 書かない (前の上書きのまま) | 書かない | 重みは 0〜1 にクランプ。パーツの番号が範囲の外なら書かない |
| 動的物理の加算量 | `DynamicOffsets` (`ModelInstance::dynamic_offsets_mut`) | 評価で 0 とみなす (書いた値は残る) | 0 とみなす | (意味どおり) |
| トラッキングの入力 | `Puppet::set_tracking` (`TrackingInput`) | 式の値が有限でないパラメータには書かない ([avatar-profile.md](spec/avatar-profile.md#フェイストラッキングとの対応)) | 同じ | 値域にクランプ |
| 当たり判定の点 | `Puppet::hit_test`・`hit_areas_at` | 当たらない | 当たらない (範囲の外) | (意味どおり) |
| 光源 | `ugoe::lighting` の `Lighting` | その光源を使わない ([rendering.md](spec/rendering.md#ライティング)) | 同じ。`range` の無限大は減衰なし | `range` が正でなければ使わない |
| モーション・表情 (検証していないもの) | `Puppet::start_motion`・`Puppet::set_expression` | パラメータへは書かない。パーツの不透明度のカーブは使わない。リップシンクの強さが有限でなければリップシンクを当てない。長さの NaN は 0 と同じ | パラメータへは値域にクランプ。パーツの不透明度のカーブとリップシンクの強さは NaN と同じ | 値域にクランプ |
| エフェクトの設定 (検証していないもの) | `Puppet::set_effects` (検証するなら `Puppet::try_set_effects`) | 目パチの `interval` が 0 より大きくなければまばたきしない。呼吸の `cycle` が 0 以下なら `sin` の項は 0 ([motion.md](spec/motion.md#エフェクト)) | 同じ | 同じ |
| 物理の状態と出力 | `PhysicsSystem` | 鎖を静止姿勢に置き直し、出力を書かない ([physics.md](spec/physics.md#有限でない状態)) | 同じ | — |

- ファイルから読むモデル・物理・モーション・表情の数は、読み込みで有限かを検証する (`ugoe-format`、[model.md#検証](spec/model.md#検証))。
  上の「検証していないもの」は、Rust から組み立てた値 (`Puppet::start_motion`・`Puppet::set_effects` に直接渡したもの) から来たもの。
  エフェクトの設定は `Puppet::try_set_effects` で読み込みと同じに検証でき、C ABI・wasm・Godot の `set_effects_json` はこれを使う (T0461)。
- `Puppet::update` でモーションのパーツの不透明度のカーブとリップシンクの強さを使わない値は、指揮役が配る記録
  (`part_opacities_applied`・`lip_sync_applied`) にも入れない。受け手の `follow` は有限でないものを拒むので、指揮役の出力はいつも受け手に通る
  (`crates/ugoe/tests/host_numbers.rs`)。
- `ParameterValuesError` のバリアントは上の層で区別しない。C ABI は `UGOE_ERROR_INVALID_ARGUMENT` とエラーの文 (`Display`)、
  wasm は文だけの例外 (`code` を持たない) で表す。バリアントの名前は文にも結果コードにも出ない (T0165 の改名で上の層の値は変わっていない)。

### 入口ごとの表し方

「拒む」は、何も変えずに失敗を返す (C ABI は `UGOE_ERROR_INVALID_ARGUMENT`、Unity は `UgoeException`、Godot は `godot_error!` と false、wasm は例外)。
空欄は下の層と同じ。Unity の `Ugoe.Puppet` は C ABI をそのまま呼ぶので C ABI の列と同じ (違うものだけ書く)。

| 値 | C ABI (`ugoe-ffi`)・Unity | wasm (`WebPuppet`、`@ugoe/runtime`) | Godot (`UgoePuppet` ノード) |
| --- | --- | --- | --- |
| `dt` (`update`) | 負・NaN・無限大を拒む (`ugoe_puppet_update`)。Unity の `UgoePuppet` の毎フレームの進め方は、`Time.deltaTime × speed` が正の有限でなければ 0 で進める | 下の層と同じ (0 とみなす) | `advance(delta)` は負・NaN・無限大を拒む。`speed` の setter も負・有限でない値を拒む |
| パラメータの値 | NaN を拒む (`ugoe_puppet_set_parameter`・`set_parameters_by_id`)。無限大はクランプ | 下の層と同じ (`set_parameter` は書かずに `true`)。`<ugoe-avatar>` の `parameters` 属性は有限でない値の組を無視する | `set_parameter` は NaN を拒む。無限大はクランプ |
| リップシンクの音量 | NaN を拒む。負 (`-∞` を含む) は入力をやめる (`None`)。`+∞` は 1 | 下の層と同じ (NaN は `undefined` と同じ) | `set_lip_sync_level` は C ABI と同じ (NaN を拒み、負は入力をやめる)。`clear_lip_sync_level` でもやめられる |
| 基準値の列・受け手 (`follow`) | 下の層の拒否を結果コードにする。加えて `dt` の負・NaN・無限大を拒む。音量が負ならリップシンクを当てない | 下の層の拒否を例外にする。`dt` は 0 とみなす | 下の層の拒否をエラーにする。加えて `delta` の負・NaN・無限大を拒む。音量が負ならリップシンクを当てない |
| トラッキング | 頭の回転とブレンドシェイプが有限でなければ拒む。体と肩は NaN を省略とみなし、無限大を拒む (`UgoeTrackingInput`) | 下の層と同じ (書かない) | `set_tracking_head` は有限でなければ拒む。`set_tracking_body` は C ABI と同じ。ブレンドシェイプは有限でない値を数えずに飛ばす (`set_tracking_blend_shape` は false) |
| 光源 | 向きが 0・有限でない値・`range` が正でない・負の色を拒む (`ugoe_lighting_uniform`) | `set_lighting_json` は同じ条件で例外 | `ambient_light` は色が有限で負でなければ受け付け、そうでなければ拒む。光源は C ABI と同じ条件で、通らないものは警告を出して使わない |
| 当たり判定の点 | 下の層と同じ (当たらない) | 同じ | 同じ |
| 表示 (wasm のビュー・Godot のマスク) | — | `set_view`・`pan_by`・`zoom_at`・`set_background` は有限でなければ何も変えない ([wasm の表](../crates/ugoe-wasm/README.md#有限でない値と-dt)) | `mask_scale` は有限でなければ拒む |
| モーション・表情・エフェクト・物理の JSON | 読み込みの検証に従う (`ugoe_puppet_set_physics_json`・`ugoe_puppet_set_base_physics_json` は検証して `UGOE_ERROR_INVALID_MODEL` で拒み、`ugoe_error_kind` は `invalid_physics`。`ugoe_puppet_set_effects_json` も同じく拒み、種類は `invalid_model`。補足の値はどちらも `errors` (検査のエラーの数)・`error_codes` (その種類)。T0461・T0469) | 同じ (`start_motion_json`・`set_expression_json` は検証して例外。`set_physics_json`・`set_base_physics_json` は `code` が `invalid_physics`、`set_effects_json` は `invalid_model` で、`errors`・`error_codes` を持つ例外。`set_base_physics_json` で付けているモジュールと合成できなければ、付け外しの失敗と同じ `DressUpError` で、`code` と補足の値は C ABI の `UGOE_ERROR_DRESS_UP` と同じ種類。T0476) | 同じ (`set_effects_json`・`set_physics_json`・`set_base_physics_json` は C ABI と同じ文のエラーを出し、結果の辞書 `{ "ok": false, "code", "error", ... }` を返す。種類と `errors`・`error_codes` は C ABI と同じ。T0470) |

- C ABI が拒む値を下の層は「書かない」「0 とみなす」で受け付けるのは、上の層の表し方の違いで、前提の違いではない。C ABI は結果コードで知らせる流儀なので、
  ホストの誤りを早く見つけられるよう拒む (T0423)。
- 確かめるテスト: 下の層は `ugoe-runtime` (`nan_writes_are_ignored`・`non_finite_part_opacity_overrides_are_ignored`・`non_finite_offsets_are_not_added`)、
  `ugoe` (`crates/ugoe/tests/host_numbers.rs`・`follow_rejects_non_finite_inputs`)、`ugoe-motion` と `ugoe-physics` の各テスト (T0317・T0320)。
  上の層は `crates/ugoe-ffi/tests/`、`crates/ugoe-wasm/src/animation.rs`、Godot の smoke (`example/tests/smoke.gd`)、Unity の `tests/ReceiverTests.cs`・`FrameStateTests.cs`。

## ランタイム層が守る制約 (移植性のため)

- wasm32-unknown-unknown でビルドできること (`just wasm-check`)。
- `std` 無し (`no_std` + `alloc`) でビルドできること (`just nostd-check`。[ADR 0031](adr/0031-no-std-runtime-and-host-neutral-c-abi.md))。
  対応したクレートは次のように書きます。
  - クレートの先頭に `#![no_std]` と `extern crate alloc;` を書き、`std::` ではなく `core::` と `alloc::` から使う
    (`String`・`Vec`・`format!`・`vec!` なども `use alloc::…;` で明示する)。コード生成の feature (`ts`・`schema`) を有効にしたときだけは
    `#![cfg_attr(not(any(feature = "ts", feature = "schema")), no_std)]` で std の prelude を使う (ts-rs と schemars の derive が
    prelude の `String` などを使うため。ツール層だけが有効にする)。テストだけ std の prelude で書くなら
    `#![cfg_attr(not(test), no_std)]` にする (`ugoe-format`)。
  - 既定で有効な feature `std` を持ち、依存の `std` (`serde/std` など) はそこから有効にする。`std` を外しても公開 API は変えない
    (エラー型は `core::error::Error` を実装する。thiserror 2 は `std` 無しでもそうなる)。
  - `std::io` (`Read`・`Write`) は使わない。バイト列のスライスを読み、`Vec<u8>` に書く (zstd の展開は `ruzstd::io::Read`。
    `std` があれば `std::io::Read` そのもの)。
  - `HashMap`・`HashSet` は使わない (`std` にしかない)。`BTreeMap`・`BTreeSet` か、決まったハッシュ関数の `hashbrown` に置き換える。
  - 数学関数 (`sin`・`atan2`・`powf`・`floor`・`rem_euclid` など) は `ugoe_core::math` から呼ぶ。中身は `std` の有無にかかわらず常に
    `libm` (純粋な Rust の実装) で、`glam` も feature `libm` で同じ実装を使う。`f32::sin` などは `std` にしか無く、OS の C の数学ライブラリで
    最後の桁が変わりうるため。`std` の有無でも OS でも、同じ入力から同じビット列の出力が得られる。`abs`・`signum`・`to_radians`・
    `min`・`max`・`clamp` などは `core` にあり、結果も IEEE で決まるので、そのまま使う。
    足りない関数は `ugoe_core::math` に `libm` を呼ぶ関数を足す。`std` の `f32`・`f64` の数学関数を直接呼ぶと、clippy の `disallowed_methods`
    が警告する (対象は `scripts/nostd-check.mjs` の `MATH_CRATES` で、各クレートの `clippy.toml` に書く。`just groups` が揃っているかを検査する)。
  - ワークスペースの依存 (`[workspace.dependencies]`) は既定の feature を切ってある (`glam`・`ruzstd`・`serde`・`serde_json`・`thiserror`・
    `ugoe-core`・`ugoe-model`・`ugoe-format`・`ugoe-runtime`・`ugoe-physics`・`ugoe-motion`・`ugoe`)。メンバーの `default-features = false` はワークスペースの側で切らないと効かないため。使う側で
    `features = ["std"]` を書く (`ruzstd` は `["hash", "std"]`)。
  - 検査の対象は `scripts/nostd-check.mjs` の `NOSTD_CRATES`。`aarch64-unknown-none` (`std` が無く、`core` と `alloc` がビルド済みで
    配られる tier 2 のターゲット) で、クレートを 1 つずつ `--no-default-features` で clippy にかけるので、どこかで `std` に依存すると
    (依存の feature の統合の漏れを含めて) 失敗する。ターゲットは `rustup target add aarch64-unknown-none` で入れる。
    CI は `rust (runtime)` のジョブで流す。no_std に対応したクレートを `NOSTD_CRATES` に足す。
    `HOST_TEST_CRATES` (`ugoe-format`・`ugoe-ffi`) は、`std` を外した形の単体テストもホストで流す ([testing.md](testing.md#std-の無い形の単体テスト))。
- `std::fs`、`std::thread`、`std::time` を使わないこと。時間は `dt` として受け取ります。
- グローバルな状態 (static mut、グローバルなロガーの初期化など) を持たないこと。
  例外: 出力に影響しない一意な ID を振るためのアトミックなカウンタ (`ModelInstance::id()`) は認める。初期化が不要で、評価結果の決定性にも影響しないため。
  カウンタは `core::sync::atomic::AtomicU64` なので、64 ビットのアトミックを持たないターゲット (`thumbv6m` など) は対象外 (T0181)。
- 重い依存を追加しないこと。追加する場合は ADR を書きます。
- 外から来るデータ (ファイル・ホストの引数) のどんな値でもパニックしないこと ([下](#パニックしない))。

`std` 無しでビルドできるクレート (`NOSTD_CRATES`) は、`ugoe-core`・`ugoe-model`・`ugoe-format`・`ugoe-runtime`・`ugoe-physics`・`ugoe-motion`・
`ugoe` (`render` を外した形)・`ugoe-ffi` です。ゲーム機・組み込み機器のように Rust の `std` が無い環境では、これらを `default-features = false` で使います。
対象にしないのは `ugoe-render` (wgpu) と `ugoe-wasm` (ブラウザ向けで `std` がある) です。PNG とブロック圧縮の KTX2 のデコードは
`ugoe-format` の feature `png` (`std` を含む) の側にあり、`std` 無しで読める画像は圧縮していない KTX2 だけです
([crates/ugoe-format/README.md](../crates/ugoe-format/README.md)。テクスチャのデコードはもともとホストの役目)。
C ABI (`ugoe-ffi`) の `std` の無い形 (staticlib。アロケータはホストの `malloc`・`free` など、パニックはホストの `abort`) の
作り方と組み込みの手順は [crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md#std-の無い環境) にあります (T0491。
[C ABI の規約](#c-abi-ugoe-ffi-の規約))。`ugoe-ffi` は `#![cfg_attr(not(test), no_std)]` で書き、`catch_unwind` を使う `std` の形だけ
`extern crate std;` を足します。

### パニックしない

`std` の無い環境の C ABI は `panic=abort` で、パニックはアプリ全体を止めます
([ADR 0031](adr/0031-no-std-runtime-and-host-neutral-c-abi.md#ランタイム層ではパニックを起こさない))。そこでランタイム層
(`ugoe-core`・`ugoe-model`・`ugoe-runtime`・`ugoe-physics`・`ugoe-motion`・`ugoe-format`・`ugoe`) と C ABI (`ugoe-ffi`。T0491) は、外から来るデータ
(ファイルのバイト列・JSON・ホストが渡す数と番号と ID・ホストが組み立てた設定) のどんな値でもパニックせず、止まらない (時間の
ループに回数の上限を持つ) ことを守ります。受け付けない値は失敗を返すか、[ホストから来る数](#ホストから来る数) の扱い (書かない・
0 とみなす・使わない) にします。T0184 で決めました。

**書き方**

- 添字 (`v[i]`・`&v[a..b]`) の代わりに `get`・`get_mut`・`zip`・`iter().take()` を使い、範囲の外なら飛ばす・既定値にする。
  検証済みのモデルやコンパイルで作った番号のように、範囲の外が起こらないと示せる箇所でも、書き換えやすければ `get` にする
  (食い違ったデータが来ても止まらないように)。
- `expect`・`panic!`・`unreachable!`・`unimplemented!`・`todo!` を使わない (`unwrap` はワークスペースの lint で禁止済み)。
  `debug_assert!` も使わない (debug ビルドで止まる)。
- 外から来る大きさ・個数・番号の積と和は `checked_*`・`saturating_*` で求める (debug ビルドの桁あふれのパニックと、wasm32 の
  32 ビットの `usize`)。確保の前に上限を確かめる (`Vec::with_capacity` は桁あふれでパニックする)。
- `copy_from_slice` は長さが違うとパニックする。ホストが差し替えうる列 (パラメータの値など) は `clear` と `extend_from_slice`
  で写す (長さが同じなら確保しない)。
- 本当に起こらないと示せる箇所 (`const fn` の中・コンパイルで作った番号だけを使うグラフの探索など) だけ、理由を書いて
  `#[expect(clippy::indexing_slicing, reason = "…")]` にする。

**パニックしてよいもの (Rust の API の約束)**

Rust の公開 API で、添字を引数に取る問い合わせ (`ParameterValues::get`・`set`・`min`・`max`・`default_value`・`blend`、
`ModelInstance::part_opacity`・`part_opacity_override`・`deformer_world`・`deformer_parent_world`・`mesh_parent_world`・
`mesh_local_positions`・`mesh_contains`・`hit_area_contains`、`CompiledModel::mask_refs`・`mask_mode`・`mask_depth`・
`hit_area_mesh`) は、`Vec` の添字と同じく範囲の外でパニックし、doc コメントの `# Panics` に書きます (値を返せないため)。
これらに外から来た番号を渡す上の層 (C ABI・wasm・Godot) は、先に範囲を確かめます。値を書くだけの API
(`ModelInstance::set_part_opacity_override`・`DynamicOffsets` の `add_angle` など) は、範囲の外なら何もしません。

**検査の方法**

- **lint:** 上のクレートの `lib.rs` で `clippy::expect_used`・`clippy::indexing_slicing`・`clippy::panic`・
  `clippy::string_slice`・`clippy::unimplemented`・`clippy::unreachable` を有効にしてあり、`just ci` の clippy (`-D warnings`)
  が止めます。テストのコードは各クレートの `clippy.toml` (`allow-indexing-slicing-in-tests` など) で除きます。
  算術の桁あふれ (`clippy::arithmetic_side_effects`) は浮動小数点の演算まで数えて多すぎるので有効にせず、外から来る大きさ・個数の
  箇所を監査しました (T0184)。
- **壊れた入力のテスト:** `crates/ugoe-format/tests/broken_input.rs` (`assets/compat` のバンドルとモジュールを切り詰め・ビットを
  反転して読み、画像をデコードし、読めたモデルを評価する。`ugoe_model::fixtures` の JSON を切り詰め・書き換え・数を極端な値にして読む。
  深い入れ子)、`crates/ugoe/tests/broken_input.rs` (同じく壊したバンドルとモジュールを `Puppet` で動かす。NaN・無限大・巨大な数の
  パラメータ・`dt`・トラッキング・受け手の入力、範囲の外の番号と知らない ID、検査していない物理の設定・モーション・表情・エフェクト、
  ホストが評価の状態を別のモデルのものに差し替える)、`ugoe-format` の `image` の単体テスト (壊した KTX2 と PNG)。
  どれも決まった入力で、依存を足さずに stable で動きます (乱数による fuzz は使っていない)。
- C ABI (`ugoe-ffi`) も同じ lint を有効にしています (T0491)。C ABI は `std` がある形で `catch_unwind` を残します (後ろの守り)。
  `std` の無い形の静的ライブラリはパニックで `abort` するので、C ABI の入口もパニックしてはなりません。
  `ugoe-wasm` は対象外です (ブラウザ向けで `std` があり、パニックは wasm の trap として JavaScript の例外で届く)。

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

