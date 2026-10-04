---
id: T0211
title: コードレビュー: ugoe-render
status: done
milestone: M9
crates: [ugoe-render, docs]
depends_on: []
assignee: claude-worker-T0211
---

# T0211: コードレビュー: ugoe-render

## 目的

ugoe-render のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-render` のすべて (シェーダを含む)

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - デバイスとサーフェスを所有しない規約、資源の作り直しと解放 (リーク)
  - マスク・合成モード・ライティングが `docs/spec/rendering.md` と一致するか、プリマルチプライドの扱い
  - 毎フレームのバッファの確保・転送の省略 (T0207)・バッチ (T0208) の正しさ
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0213〜T0217 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

読んだもの: `src/` (lib・mask・upload・pipeline・lighting・`shaders/drawable.wgsl`)、`tests/gpu.rs`・`tests/spec_render.rs`、README、
`docs/spec/rendering.md`、`spec/rendering.md` の該当節。`just test-gpu` 相当 (`cargo test -p ugoe-render --features gpu-tests`) は
この環境 (WSL2 の dzn) で通る。

確かめたこと: デバイス・キュー・サーフェスは受け取るだけで所有しない。アトラス・ジオメトリ・テクスチャは置き換え時に `Drop` で解放され、
アトラスの縮小は `AtlasPool` のヒステリシスどおり (既存のテストあり)。シェーディング・合成モードの BlendState・マスクの式 (`Product`/`Clip`・
`k = 1 − s(1 − m)`・入れ子の継承・`mask_only`)・ライティング (法線の回転と反転・減衰・光源の上限) は仕様書と一致し、乗算済みのまま扱う。
転送の省略 (T0207) の 4 つの場合分けとマスクアトラスの使い回しの条件、`queue.write_buffer` の 4 バイト整列、バッチ (T0208) の描画の結果は正しい。

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 中 | `src/lib.rs:1100` (`encode` の本描画) | `batch_draws = false` のとき、`state_changes` を直前の要素の状態と比べていたので、別のアトラスを読む要素の間にマスクを読まない要素を挟むとアトラスの切り替えを数え損ない、まとめた場合と値が違った (仕様は同じ値) | 直した (まとめた場合の状態で数える。GPU テスト `unmasked_drawable_between_atlases_counts_the_atlas_change` を先に追加) |
| 中 | `src/lighting.rs:93` | 平行光源の向きの長さを成分の 2 乗の和で求めていたので、有限で 0 でない向きでも 2 乗があふれる (成分が ~1.8e19 超) か 0 に潰れる (~4e-23 未満) と「使えない光源」として捨てていた (仕様は「0 か有限でない向き」だけを捨てる) | 直した (最大の成分で割ってから正規化。単体テストを先に追加) |
| 中 | `src/lib.rs` (`set_texture`・`set_normal_map`) | デバイスの `max_texture_dimension_2d` を超える大きさを確かめずに wgpu に渡す (検証エラーで既定はパニック)。`index` で `Vec` を伸ばすので極端な番号はメモリを使い切る。呼び出し側 (`ugoe-headless`・`ugoe-wasm`) が同じ検査を重複して持つ | T0213 を起票 (`RenderError` の variant の追加と、呼び出し側の範囲の外の変更を伴う) |
| 低 | `src/lib.rs:1511` | `set_texture` と `set_normal_map` が同じ長さの検査を重複して持ち、`width × height × 4` を `usize` で掛けるので wasm32 ではあふれうる (debug ではパニック) | 直した (`check_rgba8` にまとめ、`checked_mul` で求める。単体テストを追加) |
| 低 | `src/lib.rs:32`・`:390` | 公開の `RenderError` (と variant)・`Renderer::new`・`with_options`・`target_format` に doc コメントが無い。`encode` の `target` の前提 (形式と mip 0 全体のビュー。マスクの解像度をテクスチャの大きさで決めるため) が書かれていない | 直した (doc コメントを追加) |
| 低 | `tests/spec_render.rs:172` | `gpu-tests` を有効にすると clippy (`chunks_exact` に定数) で落ちる。CI の lint は feature を有効にしないので見つからなかった | 直した (`as_chunks::<4>()`) |
| 低 | `README.md` | 関連タスクに T0205・T0206・T0207 が無い | 直した |
| 低 | `src/lib.rs:745` 付近 (`encode`) | 毎フレーム `draws`・`bboxes`・`active_of`・`requests`・`active`・`calls` と `mask::allocate` の作業用の `Vec` を確保する | 見送り (要素数に比例する小さな確保で、1 体 1ms の目標に対して測れるほどではない。計測で問題になったら作業領域に移す) |
| 低 | `src/pipeline.rs` | `BlendMode::Unknown` が `Normal` と同じ BlendState の別のパイプラインになり、両者は別の描画の状態として数えられる | 見送り (未知の合成モードを含むモデルでだけ起き、見た目は正しい) |
| 低 | `tests/gpu.rs` | `encode` がエラーのとき何も積まないこと (仕様の「コマンドを積みません」) を直接確かめるテストが無い | 見送り (エラーはコマンドを積む前の検査でだけ返り、コードから明らか) |

### 横断

- **呼び出し側での入力の検査の重複:** デバイスの上限 (`max_texture_dimension_2d`) の検査が `ugoe-headless` と `ugoe-wasm` に別々にある。
  下位のクレート (ここではレンダラ) がエラーで返さないと、上位のクレートごとに同じ検査が増える。他のクレートでも「誰が外からの値を検査するか」を揃えたい。
- **feature を有効にしたときの clippy:** CI の lint は既定の feature だけなので、`gpu-tests` のような feature の下のコードの警告が溜まる。
  `cargo clippy --all-features` (またはテストを走らせる feature の組) を lint に含めるかを決めたい。
- **`usize` の掛け算と wasm32:** 大きさを `as usize` で掛けるコードは、wasm32 (32 ビット) ではあふれうる。ランタイム層では `checked_mul` か `u64` で求める。
- **`f32` のベクトルの正規化:** 長さを 2 乗の和で求めると、有限な値でもあふれたり 0 に潰れたりする。外から来る向き (光源・入力) を正規化する箇所では、最大の成分で割ってから求める。
- **`just ci` の負荷による不安定:** 並行のビルドで負荷が高いと (load average ~48)、`apps/editor/src/lib/preview/tracking.test.ts` の `beforeAll` (動的 import) が 10 秒の hook の時間切れで落ちる。
  変更と無関係で、`--hookTimeout 120000` で単独に流すと通り、`just web-check` を流し直すと全体が通った。

### just ci

`just ci` の 1 回目は `web-check` で `apps/editor/src/lib/preview/tracking.test.ts` が `Hook timed out in 10000ms` で落ちた
(このブランチはフロントエンドを触っていない。負荷による時間切れ。上の「横断」)。それより前の段 (fmt-check・lint・test・layers・groups・licenses・
publish-check・spec) は通った。`just web-check` を流し直して通り、残りの `just i18n app-build-check` と `just wasm-check` も通った。

