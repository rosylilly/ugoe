# 移植性とエンジン組み込み

## 方針

ランタイムは次の 3 段階で組み込めるようにします。深い段階ほど手間がかかりますが、ホスト側の自由度が上がります。

| 段階 | 使うもの | 向いている場面 |
| --- | --- | --- |
| A. 全部入り | `ugoe` + `ugoe-render` (wgpu) | Rust 製のアプリ・Bevy 以外の wgpu アプリ・Web (wasm) |
| B. 評価だけ | `ugoe` (+ C ABI) で頂点を取り出し、描画はエンジンで行う | Unity、Unreal、Godot、Bevy (wgpu のバージョンが合わない場合) |
| C. 再実装 | 仕様書 (`docs/spec/`) だけを使う | Rust を持ち込めない環境 |

**段階 B を第一級に扱います。** ゲームエンジンは自前の描画パイプライン (ソート・バッチ・ポストエフェクト) を持つので、描画はエンジンに任せ、ugoe はエンジンに頂点と状態を渡すのが最も組み込みやすいためです。
そのため `ugoe-runtime` の出力 (`DrawableState`) は、GPU の知識なしで使える素直なデータにしています。描画の式は [spec/rendering.md](spec/rendering.md) が正であり、参照シェーダ (WGSL) は naga で HLSL / GLSL / MSL / SPIR-V に変換できます。

## ターゲット別

| ターゲット | 方式 | 備考 |
| --- | --- | --- |
| Windows / macOS / Linux | 段階 A (Vulkan / Metal / DX12 / GL) | |
| iOS / Android | 段階 A または B | `ugoe-ffi` を staticlib / cdylib としてビルドする |
| Web | `ugoe-wasm` (WebGPU、なければ WebGL2) | npm パッケージとして配布する (M4) |
| Unity | 段階 B: `ugoe-ffi` + C# (P/Invoke) + Unity の Mesh / Material | 描画はエンジンのシェーダで行う |
| Unreal | 段階 B: `ugoe-ffi` + C++ プラグイン + ProceduralMesh または独自の SceneProxy | |
| Godot | 段階 B: GDExtension (godot-rust 経由で ugoe を直接使うのが有力) | |
| Bevy | 段階 A (wgpu のバージョンが一致する場合) または B | Bevy が使う wgpu のバージョンに合わせた ugoe-render のリリースを検討する |

## C ABI (`ugoe-ffi`) の規約

- 公開する関数はすべて `ugoe_` で始めます。
- オブジェクトは不透明なポインタで渡し、`ugoe_*_create` / `ugoe_*_destroy` で寿命を管理します。
- 関数はエラーコード (`UgoeResult`、`i32`) を返し、詳細なエラーメッセージはスレッドローカルの `ugoe_last_error()` で取得できるようにします。
- パニックは `catch_unwind` で捕まえてエラーコードに変換します。FFI の境界の外にパニックを漏らしません。
- 頂点データは、ホストが確保したバッファにコピーする API (`ugoe_puppet_copy_positions(p, drawable, out_ptr, len)`) と、内部のバッファを借用する API (次の `update` まで有効) の両方を提供します。
- ヘッダファイルは cbindgen で生成して `crates/ugoe-ffi/include/ugoe.h` に置き、CI で最新かどうかを検査します。
- `ugoe_abi_version()` の値は、ABI を壊す変更をしたら上げます。

## ランタイム層が守る制約 (移植性のため)

- wasm32-unknown-unknown でビルドできること (`just wasm-check`)。
- `std::fs`、`std::thread`、`std::time` を使わないこと。時間は `dt` として受け取ります。
- グローバルな状態 (static mut、グローバルなロガーの初期化など) を持たないこと。
- 重い依存を追加しないこと。追加する場合は ADR を書きます。

> 未決: 組み込み機器向けに `no_std + alloc` に対応するか。`ugoe-core`、`ugoe-model`、`ugoe-runtime` は対応できる設計にしておき、需要があれば対応する。
