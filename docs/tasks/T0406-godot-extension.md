---
id: T0406
title: Godot GDExtension
status: done
milestone: M7
crates: [bindings/godot, docs]
depends_on: []
assignee: claude-worker-T0406
---

# T0406: Godot GDExtension

## 目的

godot-rust で `ugoe` を使う GDExtension を作る。

## 受け入れ条件

- [x] サンプルシーンでモデルが動く
- [x] Godot のノードとしてパラメータを設定できる
- [x] ライセンスの扱い (利用者の決定、2026-10-01): `bindings/godot` は crates/ の外の独立したクレートにし、バインディング自体は MIT のまま、godot-rust (gdext、MPL-2.0) に依存してよい。ワークスペースの外に置くか、ライセンスの検査の対象から外す方法を決め、ADR 0026 に書く (MPL はファイル単位なので、ugoe 側のコードは MIT のまま)
- [x] `just ci` が通る

## 参考

- `docs/portability.md`

## メモ

### やったこと

- `bindings/godot` に GDExtension のクレート `ugoe-godot` (MIT) を作った。godot-rust 0.5 (`api-4-4`、Godot 4.4 以降) で `ugoe::Puppet` を直接使う。
  - ルートのワークスペースの外 (独自の `[workspace]` と `Cargo.lock`)。ルートの `Cargo.toml`・`just ci`・`just licenses` は変えていない。
    依存のライセンスは `bindings/godot/check-licenses.mjs` で別に検査する (寛容なもの + godot-rust のクレートだけ MPL-2.0)。ADR 0026。
  - ノード `UgoePuppet2D` (Node2D、tool): `model_path` で `.ugoe` を読み、毎フレーム `Puppet::update` を進めて描く。
    パラメータはインスペクタの `parameters/<ID>` (設定したものだけシーンに保存) と `set_parameter` など。モーション・表情・リップシンク・
    物理のリセット・当たり判定・シグナル (`model_loaded`・`motion_event`) も出した。API の一覧は `bindings/godot/README.md`。
  - 描画は Godot の 2D (RenderingServer のキャンバスアイテムと生成したシェーダ)。乗算済みアルファ・乗算色とスクリーン色・3 つの合成モード
    (`blend_premul_alpha` / `blend_mul` で表す)・マスク (マスク用メッシュごとの `SubViewport` にカバレッジを描き、キャンバス座標から読む。
    入れ子・反転・共通部分・`mask_strength`)・裏面カリング (CPU)。マスクの解像度は画面上の大きさに合わせる (`mask_scale` で倍率)。
- 見本のプロジェクト `bindings/godot/example` (パラメータのスライダー、表情・モーションのボタン、当たり判定の出力)。
- `node bindings/godot/build.mjs` (ビルドと見本への配置)、`--check` (ライセンス + ヘッドレスの Godot での `tests/smoke.gd`)、
  `--conformance` (`ugoe render` の画像との比べ合わせ。`tests/conformance.gd`)。

### 確かめたこと

- `cargo test` / `cargo clippy -D warnings` / `cargo fmt --check` (bindings/godot)、`just ci`。
- この環境の Godot 4.7.1 で `build.mjs --check` (smoke: 読み込み・パラメータの設定とクランプ・プロパティ・シーンへの保存と復元・
  読み込みの失敗) が通る。
- Xvfb (Mesa llvmpipe、`gl_compatibility`) で `build.mjs --conformance`: `avatar.ugoe` と、フィクスチャから作った 4 つのモデル
  (入れ子・反転・共通部分のマスク、合成モード、乗算色・スクリーン色、カリング、半透明のテクスチャ) がすべて 100% の画素で 3/255 以内
  (最大の差 2/255)。見本のシーンを `--write-movie` で撮って、表示と動き (AngleX の揺れ) を見た。

### 確かめていないこと (利用者の確認に残す)

- 画面のある Godot のエディタでの操作 (インスペクタでの `parameters/<ID>` の編集、エディタの中での表示)。ヘッドレスのエディタで
  見本のシーンを開いて閉じられることまでは確かめた。確かめ方: `node bindings/godot/build.mjs` の後に `godot -e --path bindings/godot/example`。
- Forward+ / Mobile のレンダラ (Vulkan) での見た目 (比べ合わせは Compatibility (OpenGL) だけ)。Windows・macOS でのビルド。
- アニメーション中の入れ子のマスクが 1 フレーム遅れないか (Godot の `SubViewport` の描く順に頼っている。静止画では正しい)。

### 分かったこと

- Godot 4.7.1 は、GDExtension のクラスを持つプロジェクトの初回の `--import` の終わりに異常終了することがある (SIGABRT)。
  godot-rust の最小のクラス (フィールドが base だけ) でも起き、2 回目からは起きない。`build.mjs` は 1 回だけやり直す。
- `canvas_item_add_triangle_array` に空の頂点番号を渡すと、Godot は頂点を順に三角形として描く (カリングで全部消えたときに注意)。

### 範囲外 (team-lead に相談した。返事が無かったので入れていない)

- `justfile` のレシピ (`godot-check` など)、`AGENTS.md` のリポジトリ構成への `bindings/godot` の行、`.github/workflows/ci.yml` のジョブ。

### 後続の候補

- CI に `build.mjs --check` と `--conformance` を入れる (Godot と Xvfb が要る)。T0408 (適合テストの共通化) で頂点の比べ合わせも。
- 配布: Godot のアセットライブラリ向けのパッケージ (各 OS のビルド済みライブラリと `.gdextension`)。リリースの自動化 (ADR 0017)。
- ライティング (法線マップと Godot の 2D ライト or ugoe の光源)・着せ替え・トラッキングをノードに出す。
- `.ugoe` を Godot のリソースとして取り込む (`ResourceFormatLoader`)。今はファイルのバイト列を読むので、書き出しの設定が要る。
- 1 要素あたりのマスクの上限 (8) をなくす (マスクのアトラスにまとめる)。
