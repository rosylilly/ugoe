---
id: T0201
title: 参照レンダラ: パイプラインと描画
status: done
milestone: M1
crates: [ugoe-render]
depends_on: []
assignee: claude-worker-T0201
---

# T0201: 参照レンダラ: パイプラインと描画

## 目的

`Renderer` の `set_texture` / `encode` を実装し、マスクなしの描画可能要素を 3 種類の合成モードで描画する。

## 受け入れ条件

- [x] パイプラインを (合成モード × カリング) の組み合わせでキャッシュする
- [x] 乗算色・スクリーン色・不透明度が仕様どおりに反映される
- [x] WebGL2 で使えない機能を使っていない (`just wasm-check` が通る)
- [x] `gpu-tests` feature で、オフスクリーン描画して画素を検証するテストがある
- [x] `just ci` が通る

## 参考

- `docs/spec/rendering.md`

## メモ

### やったこと

- `Renderer::new` / `set_texture` / `encode` をマスクなしの要素について実装した (`crates/ugoe-render/src/lib.rs`、`pipeline.rs`)。
  - 3 種類の合成モード (BlendState は仕様の表どおり)、裏面カリング、乗算色・スクリーン色・不透明度、`render_order()` 順の描画、非表示要素の省略。
  - パイプラインは (合成モード × カリング時に表とする向き) でキャッシュする。マスクの variant は T0202 で追加する。
  - UV・インデックスはモデルごとに一度だけアップロードし (インスタンスのモデルが変わったら作り直す)、位置と要素ごとのユニフォーム (動的オフセット、`min_uniform_buffer_offset_alignment` に整列) を毎フレーム書き込む。
  - WebGL2 は `base_vertex` を使えないので、全要素をまとめたバッファのスライスを要素ごとに束縛する。
  - マスクは 1x1 のダミーテクスチャと `mask_flags.x = 0` で無効化した。`TODO(T0202)` を置いた。
- `set_texture` の戻り値を `Result<(), RenderError>` に変え、データ長の不一致を `RenderError::InvalidTextureData` で返すようにした。
- `ugoe-render` の通常の依存に `ugoe-model` を追加した (`BlendMode` と静的な UV・インデックスの参照のため。依存方向の規則の範囲内)。
- 仕様書 (`docs/spec/rendering.md`) で決めたこと:
  - カリングの「反時計回り」は画面上の見た目 (キャンバス座標での符号付き面積が負) と定義し、`view_proj` の線形部分の行列式の符号で `FrontFace` を切り替える。
  - テクスチャは `Rgba8Unorm`、描画先はクリアしない (`LoadOp::Load`)、`encode` は submit ごとに 1 回、未登録テクスチャはエラー。

### GPU テストの状況

- `tests/gpu.rs` (`gpu-tests` feature) に 14 件: normal 合成と被覆範囲、不透明度、非表示要素の省略、未登録テクスチャのエラー、乗算色、スクリーン色 (不透明・半透明)、additive と multiply (既知のクリア色の上)、カリング (表・裏・カリングなし・ビューの Y 反転)、描画順。WebGL2 の既定の制限 (`Limits::downlevel_webgl2_defaults`) でデバイスを作る。
- アダプタが見つからない場合は、黙って飛ばさずに理由を書いて失敗する。
- この環境 (WSL2) で、既定のアダプタ (lavapipe、Vulkan)、`VK_DRIVER_FILES` で lavapipe に限定した場合、`WGPU_BACKEND=gl` (llvmpipe) のいずれでも 14 件すべて通ることを確認した。カリングの判定を逆にすると 2 件が失敗することも確認した。
- WSL2 では Vulkan ローダが dzn (Vulkan on D3D12) も読み込み、wgpu のインスタンスを破棄するときに SIGSEGV で落ちる (テストの成否とは無関係。ugoe のコードなしの最小構成でも再現)。テストではデバイスを static に置いて破棄しないことで回避した。
- 実機の WebGL2 (ブラウザ) での動作は確認していない (`just wasm-check` のビルド検査のみ)。

### 後続タスクの候補

- T0202: マスクアトラスとマスク付きパイプライン (`TODO(T0202)` の箇所)。
- T0203: 期待画像 (golden) との比較と CI (lavapipe) での GPU テスト実行。
- ミップマップの生成 (縮小表示時のエイリアシング対策)。今は 1 レベルのみ。
