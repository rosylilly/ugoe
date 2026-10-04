---
id: T0427
title: コードレビュー: bindings/godot (GDExtension)
status: done
milestone: M9
crates: [bindings/godot, docs]
depends_on: []
assignee: claude-worker-T0427
---

# T0427: コードレビュー: bindings/godot (GDExtension)

## 目的

bindings/godot (GDExtension) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `bindings/godot` のすべて (src・example・build.mjs。ワークスペースの外、ADR 0026)

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - godot-rust (MPL-2.0) 以外のコピーレフトの依存が無いか (`node scripts/check-licenses.mjs --godot`)
  - Godot のノードの寿命 (ツリーへの出入り・解放) と資源の解放、`_process` の毎フレームの確保
  - SDK (`ugoe` クレート) の API とのずれ・機能の欠け (ほかのバインディングとの比較)
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る (エディタの vitest の 1 件がフックの時間切れで落ち、単独と残りの段の再実行で通った。下の「検査」)

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0448〜T0452 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 高 | `src/node.rs` (`set_parameter`・`set_lip_sync_level`・`set_tracking_*`・`advance`・`speed`) | GDScript の `NAN`・`INF` と負の時間をそのまま `Puppet` に渡していた (C ABI は弾く)。NaN のパラメータで `ugoe-runtime` の `keyform::locate` が毎フレームパニックし、`advance(INF)` は目パチの無限ループに入る | 直した (C ABI と同じ条件で拒んでエラーを出す。`speed` に setter を足した。smoke に検査を追加) |
| 高 | `crates/ugoe-runtime/src/keyform.rs:161` (範囲の外) | パラメータが NaN だと `partition_point(...) - 1` がアンダーフローしてパニックする。`ParameterValues::set` は NaN を素通しする | runtime 側は T0137 で直した (NaN の書き込みを無視し、`locate` も安全に)。Godot の入口の検査は多層の守りとして残す |
| 高 | `src/model.rs` (`texture_count`・`LoadedModel::from_bundle`) | 検証の前にメッシュの `texture` の番号からテクスチャの数を決めていた。モデルの JSON (`load_model_from_json`) で番号が `u32::MAX` だと約 43 億枚分を確保しようとして止まる (バンドルは `read_bundle` が先に検証するので起きない)。T0424 (wasm) の所見の伝達で確かめた | 直した (`model.textures` の数だけを数える。テストを追加) |
| 中 | `src/model.rs` (`MaskArea::fit`) | `mask_scale` が無限大だと余白の足し算が u32 であふれる (デバッグビルドでパニック)。NaN の頂点でも大きさが壊れる | 直した (有限でない入力を丸め、`saturating_add`。`set_mask_scale` も無限大を拒む。テストを追加) |
| 中 | `src/model.rs` (`LightUniforms::new`)・`src/node.rs` (`set_ambient_light`・`light_from_dictionary`) | 光源と環境光の色の NaN・無限大・負を検査せず、シェーダに NaN が入る (doc の「有限でない値を除く」とも食い違い) | 直した (C ABI と同じく有限で負でない色だけ。テストを追加) |
| 中 | `src/node.rs` (`enter_tree`・`set_model`) | 木の外で `model` を変えると、木に戻っても読み込まない (`ready` は最初の 1 回だけ) | 直した (戻ったときに読む。smoke に検査を追加) |
| 中 | `src/node.rs` (`get_module_names`) | 読み込むときに付けられなかったモジュールも `modules` の配列から名前を返していた。適合テストの実行器の「モジュールが付いたか」の検査が効いていなかった | 直した (読み込んでいれば実際に付いているもの。smoke に検査を追加) |
| 中 | `src/model.rs` (`from_bundle_bytes`・`ModuleData::from_bytes`) | 読み込みの警告 (知らない任意の機能・未知の値 (ADR 0013)・画像の無いテクスチャ) を捨てていた | 直した (Godot の警告として出す。`assets/compat/future` でテスト)。スクリプトから読む API は T0449 |
| 中 | `src/node.rs` 全体 | SDK・C ABI にある機能 (受け手の `follow`・モーションの優先・標準の当たり判定・表情を止める・パーツの不透明度・ポーズ・効果と物理の設定・警告の取り出し) が無い | T0449 を起票 |
| 中 | `src/render.rs` (`load_texture`) | KTX2 を乗算済みかどうか確かめずに使う。仕様が決まっていない。`TODO(T0406)` は完了したタスクを指していた | T0448 を起票し、TODO を付け替えた |
| 低 | `src/node.rs` (`set_parameter`) | 読み込んだモデルに無い ID を黙って覚え、`get_parameter` がその値を返していた | 直した (警告を出して捨てる。読み込む前は従来どおり覚える) |
| 低 | `src/node.rs` (`load_model_from_buffer`) | doc コメントが `#[func]` の前後に分かれていた | 直した |
| 低 | `src/render.rs` (`draw`)・`src/node.rs` (`update_scene_lights`) | 毎フレームの `PackedVector2Array`・`Vec` の確保。`Renderer::new` でビューポートのテクスチャが無いと作ったビューポートを解放しない | T0450 を起票 (測ってから) |
| 低 | `src/node.rs` (シグナル) | `&mut self` の中で `signals().emit()` している | 見送り (godot-rust 0.5 は `base_mut()` を通すので、ハンドラから同じノードを呼んでも借用の衝突は起きない。確かめた) |
| 低 | `example/addons/ugoe/ugoe_resource_loader.gd` | ファイル全体を読み、先頭 4 バイトだけ確かめる | 見送り (中身は Rust で検査する。ADR 0026) |

依存のライセンス: `node scripts/check-licenses.mjs --godot` は通る (godot-rust 以外のコピーレフト無し)。ノードの寿命: キャンバスアイテムは
`Renderer` の `Drop` で、マスクのビューポートはノードの子として解放される。モデルの差し替えでは古いビューポートを `queue_free` する。

### 検査

- `cargo test` / `cargo clippy --all-targets -D warnings` / `cargo fmt --check` (bindings/godot): 通る (テスト 32 件)
- `node bindings/godot/build.mjs --check --conformance`: ライセンス ok、smoke ok、適合テスト 53 / 53 (4 件は計画どおり skip)
- `just ci`: `web-check` の `apps/editor` の `src/lib/preview/tracking.test.ts` が `beforeAll` の 10 秒の時間切れで落ちた
  (並行のビルドで重いとき。このタスクは apps/editor を触っていない)。そのファイルを単独で流すと 7 件とも通り、`just web-check i18n app-build-check`
  (落ちた段から後ろ) を流し直して通った。それより前の段 (fmt・clippy・テスト・layers・licenses など) は 1 回目で通った。

### 横断

- 組み込み先ごとに、外から来る数値 (NaN・無限大・負の dt) の検査がばらばら (C ABI は弾く、Godot は弾いていなかった)。SDK (`ugoe::Puppet`) か
  `ParameterValues::set` の側で NaN を止めるか、組み込み先の検査の表を 1 か所にまとめるとよい。wasm と Unity も同じ観点で見るとよい。
- ランタイム層の `keyform::locate` のように、NaN で添字の計算が壊れる箇所がほかにもありそう (`partition_point` などの前提)。
- 読み込みの警告 (ADR 0013) を組み込み先が捨てていないか (Godot は捨てていた)。
