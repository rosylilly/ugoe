---
id: T0137
title: コードレビュー: ugoe-runtime
status: done
milestone: M9
crates: [ugoe-runtime, docs]
depends_on: []
assignee: claude-worker-T0137
---

# T0137: コードレビュー: ugoe-runtime

## 目的

ugoe-runtime のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-runtime` のすべて

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - キーフォームの補間とデフォーマの評価が `docs/spec/evaluation.md` と一致するか (境界・外挿・退化した格子)
  - 決定性 (浮動小数点の演算の順序・HashMap の順序) と、毎フレームの確保
  - 変更フラグ (T0106) の立て忘れ・立てすぎ
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0145〜T0149 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

`crates/ugoe-runtime` のソース・テスト・例をすべて読み、`docs/spec/evaluation.md`・`docs/spec/modules.md#合成`・公開仕様 (`spec/evaluation.md`) と照らした。
`cargo clippy -p ugoe-runtime -- -W clippy::pedantic` も参考に流した (`must_use`・`as` の警告が大半で、`as` は値域を確かめた箇所だけだった)。

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 高 | `src/parameters.rs:81` (`set`)、`src/keyform.rs:155` (`locate`) | `ParameterValues::set` が NaN をそのまま値にし (`f32::clamp` は NaN を通す)、`locate` の `partition_point(..) - 1` があふれて `update` がパニックする (リリースでは添字の範囲外)。Rust の API・物理・モーションの出力から入りうる | 直した。公開仕様の「有限でない書き込みは無視してよい」に従い、`set` (と `blend`) は NaN を無視する。`ParameterValues` の doc に規則を書いた (日本語側の対応箇所)。テスト `nan_writes_are_ignored`・`nan_parameter_does_not_panic` |
| 高 | `src/hit.rs:69` (`in_triangle`) | 面積 0 の三角形では、頂点を通る直線上のすべての点で辺の符号が 0 になり当たりになる。1 点に潰れた三角形 (拡大率 0 の回転デフォーマで隠したメッシュなど) はキャンバス全体で当たる。頂点が NaN の三角形も全面で当たる。エディタの要素選択も `mesh_contains` を使う | 直した。符号がすべて 0 以上かすべて 0 以下 (NaN は外) で、かつ外接矩形の中にあることを条件にした (面積を持つ三角形の結果は変わらない)。テスト `degenerate_triangles_contain_only_their_points` |
| 中 | `src/compose.rs:440` (`Renamer::grid`) | 合成でミラー元 (`mirror_source`) の ID を付け替えるのが最上位の格子だけで、ブレンドシェイプ格子のミラー元が残る。ミラー元を記録したブレンドシェイプ格子を持つモジュールは、合成後の検証で `UnknownReference` になる (modules.md の合成 2. は「ミラー元」を付け替える) | 直した (ブレンドシェイプ格子へ再帰する)。テスト `mirror_sources_are_renamed_in_blend_shape_grids` |
| 中 | `src/instance.rs:1025` (テスト) | 「`update` で毎フレームの確保をしない」のテストが、曲線補間・ブレンドシェイプ (重みの作業領域を 2 組使う)・2 段階の評価のモデルを含んでいなかった | 直した。すべてのフィクスチャで確かめ、`two_stage_update_does_not_reallocate_buffers` を足した (どちらも確保は無かった) |
| 中 | `src/compile.rs:18`、`src/compose.rs:83` | `CompileError::Invalid` / `ComposeError::Invalid` の文言が検証エラーの列の `Debug` 表記で、利用者に見せるには読みにくい | 直した。各エラーの `Display` を `; ` で並べる (C ABI・wasm の書き方に合わせた)。テスト `invalid_error_lists_validation_messages` |
| 中 | `src/parameters.rs` (`index_of`)、`src/compile.rs` (`part_index` など) | ID から添字への引き当てが線形探索で、`Puppet::apply_part_opacities` (カーブごと) と C ABI の ID によるパラメータ設定 (値ごと) が毎フレーム呼ぶ | T0146 を起票 (計測してから。範囲が `ugoe`・`ugoe-ffi` に及ぶ) |
| 中 | `ugoe-model` の `validate` | 軸のキーとパラメータの値域に無限大を許す。キーが無限大だと `t` と接線が NaN になり、その格子の結果がすべて 0 になる (パニックはしない)。公開仕様の範囲内だが作者の誤りを見逃す | T0145 を起票 (範囲の外・検証の規則の判断) |
| 低 | `src/instance.rs:333` (`set_part_opacity_override`) | 重みが NaN だと `clamp` を通り、パーツの不透明度が NaN になる | 見送り。C ABI・wasm は NaN を拒否してから呼ぶ。結果が有限でないので仕様の許容の外で、パニックもしない |
| 低 | `src/instance.rs:833`・`:863` | `mesh_local_positions`・`rotation_local_keyform` が `update` の補間の式を写している (重複) | 見送り。ビット単位の一致をテスト (`mesh_parent_world_reproduces_update_output`・`second_stage_picks_up_blend_shape_parameters`) が守っている。まとめると毎フレームの経路で作業領域の借用の組み替えが要り、得るものが小さい |
| 低 | `src/hit.rs:48` (`hit_areas_at`) | 描画順のメッシュごとに当たり判定の列を走査する (メッシュ数 × 当たり判定数) | 見送り。問い合わせ用で、当たり判定は数個 |
| 低 | `src/compile.rs:114` | 参照先が見つからないときのエラーが `Unsupported("dangling reference")` | 見送り。検証の後なので起きない |
| 低 | `src/keyform.rs` (`ResolvedBinding`) | 公開のフィールドでキーと接線の係数の食い違いを作れ、キーが空だと `weights` がパニックする | doc に前提 (空でない狭義の昇順) を書いた。型の変更は `ugoe-editor-core` も使う公開 API なので見送り |
| 低 | 公開 API の doc | `CompiledModel::compile`・`ModelInstance::new`・`ParameterValues` の各メソッド・`DrawableState` のフィールド・`CompileError` の各値などに doc が無く、`dynamic_offsets` の doc は `update_first_stage` でも状態ができることを書いていなかった。`# Panics`・`# Errors` も不足 | 直した |
| 低 | `src/deform.rs`・`src/instance.rs` | `lerp` が 2 か所に同じ定義 | 直した (`deform::lerp` を共有) |
| 低 | `src/instance.rs` (`new`) | 重みの事前確保の上限 `1 << 20` の理由が書かれていない | 直した (コメント) |

確かめて問題が無かったこと:

- **仕様との一致:** 軸の位置 (端のキーへのクランプ・外挿なし・キー 1 つの軸)、多重線形の重みの並びと 0 の除去、曲線補間 (接線の係数・`base` と使う個数・形以外のフィールドは線形・キーの上で線形とビット一致)、真偽値の同点の扱い、ワープの範囲外の外挿 (四隅のアフィン近似)、退化した格子 (行か列が 0 はコンパイルで拒否、潰れた格子では δ = 0)、親がワープの回転デフォーマ、不透明度の伝播と上書き、グルー、ブレンドシェイプの加算順と「既定値と同じ重みなら加えない」、描画順の安定ソート (`total_cmp`)、2 段階の評価で評価し直すデフォーマの選び方
- **決定性:** `HashMap` はコンパイル時の ID の解決と `carry_over` の引き当てだけで、順序に依存しない。和はいつも重みの並びの順。トポロジカル順は入力だけで決まる
- **毎フレームの確保:** 無い (上の 2 つのテスト)。ワープの制御点は `mem::take` で入れ替えるだけ
- **変更フラグ (T0106):** ビット列の比較、最初の 2 回の報告、`mark_all_changed`、クランプ後の `mask_strength`、パーツの不透明度の上書き・ブレンドシェイプ・1 段目 (出力を変えない) のどれでも、立て忘れ・立てすぎは見つからなかった

### 横断 (T0008 向け)

- **ホストが書く値の NaN:** ランタイムは `ParameterValues::set` で NaN を無視するようにしたが、パーツの不透明度の上書き・`DynamicOffsets`・物理とモーションの出力など、ほかの入口は NaN を確かめない。C ABI・wasm は入口で拒否している。どこで拒否・無視するかをクレートをまたいで揃えるとよい
- **ID の線形探索:** `index_of`・`*_index` を毎フレームの経路で呼ぶ箇所がほかのクレートにもありうる (T0146)
- **検証エラーの文言:** `{0:?}` で列を出す (`ugoe/src/dress_up.rs` の物理の検証エラーなど)、`; ` で並べる (C ABI・wasm、このタスクで `ugoe-runtime` も)、件数だけを出す (`ugoe-format`) が混在している
- **キーフォーム格子をたどるコードの `blend_shapes` の漏れ:** 合成の付け替えで漏れていた。格子の中の ID を書き換えるコード (エディタの改名・ミラー、形式の移行など) も、ブレンドシェイプ格子までたどっているか確かめるとよい
- **面積 0 の三角形の判定:** 三角形の内外判定や重心座標を使うコード (エディタの選択・メッシュ生成・UV の逆写像など) で、潰れた三角形が全面で当たる・0 で割るといった誤りがないか


### 検査

- `just ci` と `just wasm-check` が通った (`git rebase main` の後)。
