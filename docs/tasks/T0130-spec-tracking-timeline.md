---
id: T0130
title: 公開仕様: トラッキングとホストの操作 (ポーズの表示・効果の差し替え) の時間の例
status: done
milestone: M5
crates: [spec, tools/spec-check, docs, ugoe, ugoe-render]
depends_on: [T0126]
assignee: claude-worker-T0130
---

# T0130: 公開仕様: トラッキングとホストの操作の時間の例

## 目的

T0126 の後続の候補。時間の例 (`spec/examples/*/timeline.json`) に、トラッキングの入力 (spec/avatar-profile.md)、ホストがポーズの表示を直接選ぶ操作、効果の設定の差し替えが含まれていない。

## 受け入れ条件

- [x] 作業者は `crates/` と `docs/spec/` を読まず、`spec/` だけを参照する (T0120・T0126 と同じ手順)
- [x] timeline.json の操作にトラッキングの入力・ポーズの表示の選択・効果の差し替えを足し、例を足す。形式の変更は `spec/examples/README.md` に書く
- [x] `tools/spec-check` の独立の実装を合わせる。参照実装での照合 (`crates/ugoe/tests/spec_timelines.rs`) の拡張は team-lead に頼む
- [x] 見つかった仕様の抜け・曖昧さの一覧と `spec/` への修正 (`docs/spec/` の同期は team-lead に頼む)
- [x] `Parameter.mirror` を省略した意味: 英語版は「対を記録していない」とだけ書くが、参照実装のミラーは省略を「自分自身の同じ値のキーに対応する」として扱う (T0129 の同期で見つかった)。spec/model.md に書き足すかを決める
- [x] 描画の例 (spec/examples/render-*) に、鏡映・回転するビュー変換、9 個以上の光源、モデルの `lighting: false` の例を足す (T0127 の後続の候補。参照実装の照合は team-lead が拡張する)
- [ ] `just ci` が通る

## 参考

- `docs/tasks/T0126-spec-physics-motion-examples.md` のメモ

## メモ

### 独立性を保った手順

- 作業者 (claude-worker-T0130) は `crates/` と `docs/spec/` を読まず、`spec/`・`tools/spec-check/`・ワークフローの文書だけを参照した。
  参照実装の振る舞い (トラッキングの入力の持ち方・ポーズの表示の選択・効果の差し替え・ビュー変換・`Parameter.mirror` の省略) は team-lead に聞き、言葉で答えてもらった。
  参照実装は黒箱として `target/debug/ugoe render` の PNG (render-reference.ts) だけを使い、`just ci` の失敗はテスト名とパニックの 1 行だけを見た。
- 期待値は参照実装の出力を写していない。描画は調べる画素だけで式を直接計算する Python の転記 (scratchpad。既存の全期待値を再現することを確かめた)、
  時間の例は手で導いた値と Python の転記 (SplitMix64・呼吸・フェード) で出し、TS の独立の実装と一致を確かめた。転記はコミットしていない (T0126・T0127 と同じ)。

### やったこと

- 描画の例: `render.json` の case に任意のビュー変換 `view` (`[a, b, c, d, e, f]`、キャンバスの px → 描画先の px、y 下向き) を足した。
  render-culling に鏡映 (左右・上下と 2 倍)・90° の回転の 3 ケース、render-lighting に鏡映のビューの平行光・回転と鏡映のビューの点光源・
  無効な光源 4 個の後の有効な光源 8 個・どの 8 個でも飽和する 12 個の光源の 5 ケース、新しい例 render-lighting-off (モデルの `lighting: false`)。
  8 個を超える光源のケースは、どの 8 個を使っても調べる画素が同じになるように選ぶ、と README に書いた。
  `ugoe render` との黒箱の比較: view の無いケースはすべて一致 (CLI はビューを指定できないので view のケースは飛ばす)。
- 時間の例の操作: `tracking` (以後毎フレーム与える入力。null で外す)・`show_pose` (エントリの part ID)・`set_effects` (Effects 全体。seed は正確に読む)。
  例: tracking (標準の対応・上書き・無視される要素・internal・切り詰め・フレームの順の位置・入力を外した後)、pose-select、effects-replace。
- spec-check: 追従の対応 (`tracking.ts`、`test/tracking.test.ts`)、フレームの手順 3、ポーズの表示の選択、効果の差し替え、ラスタライザのビュー変換。
- `Parameter.mirror` の省略: 「ミラーで変わらず、同じパラメータの同じキーに対応する」と spec/model.md に書き、メッシュ・デフォーマ (省略も自分自身も同じ意味) との違いも書いた (参照実装と同じ)。

### 見つけた仕様の抜け・曖昧さ

| # | 箇所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 1 | motion.md Pose | 「ホストが表示するエントリを直接選んでよい」だけで、指定の仕方・効く時点・同じフレームの要求との優先・pose の段が off のときが無い | 「Showing an entry」を書いた (part ID で選ぶ、リンクは対象外、次のフレームからフェード、値 > 0 の要求が勝つ、off の間は表示だけ変わる。参照実装と同じ) |
| 2 | motion.md Effects | 目パチの「設定を差し替えたとき」とあるが、差し替えの操作・乱数と相・呼吸の位相・リップシンクの音量の扱いが無い | 「Replacing the effect settings」を書いた (全体を差し替え、同じ設定でも始め直す、目パチは新しい seed で open・t = 0 から最初の間隔を引く、呼吸の位相は 0、音量は保つ。参照実装と同じ) |
| 3 | motion.md Consequences | 追従の入力を止めた後の値が書かれていない | 最後に追従した値が基準値として残る、と明記 (参照実装と同じ) |
| 4 | model.md Mirror pairs | パラメータの `mirror` の省略の意味 (「対を記録していない」だけ) | 上のとおり明記 |
| 5 | rendering.md Lights / Conformance | 8 個を超える光源は適合の対象外で、例が作れない | 仕様は直さず、例をどの 8 個でも同じ結果になるように選び、README の作り方の規則に書いた |
| 6 | rendering.md Inputs | ビュー変換が退化 (行列式 0) のときの点光源の p が定まらない | 未修正。退化したビューでは三角形が面積を持たず何も描かれないので実害はない (team-lead: 実装の都合で定めなくてよい) |

### just ci

- 失敗は参照実装の側の照合テスト 2 つだけ (新しい形式を知らないため。team-lead が拡張する):
  - `crates/ugoe/tests/spec_timelines.rs` の `puppet_meets_the_expected_values_of_every_timeline_example`:
    `effects-replace/timeline.json: data did not match any variant of untagged enum Action at line 64 column 7`
  - `just test-gpu` の `crates/ugoe-render/tests/spec_render.rs` の `reference_renderer_meets_the_rendering_examples`:
    `render-culling/render.json: unknown field `view`, expected one of `summary`, `parameters`, `target`, `lights`, `pixels``
- `cargo test --workspace --no-fail-fast` の失敗は上の 1 つだけ。fmt-check・clippy・layers・licenses・spec・web-check は通った。
- main への rebase はしていない (main の差分は spec/ と tools/ に触れない。team-lead が rebase する)。

### docs/spec の同期が必要な箇所 (team-lead に依頼)

- spec/model.md: Mirror pairs (パラメータの `mirror` の省略と、メッシュ・デフォーマとの違い)。
- spec/motion.md: Effects の「Replacing the effect settings」(新しい節)、Pose の手順 1 と「Showing an entry」、Order of operations の Consequences (追従を止めた後)、Determinism のホストの操作の列挙。
- spec/examples/README.md: 時間の例の操作 `tracking`・`show_pose`・`set_effects`、描画の例の `view` と 8 個を超える光源の規則、例の表の行 (tracking・pose-select・effects-replace・render-lighting-off)。

### 確かめていないこと

- 参照実装が新しい例 (view のケース・tracking・pose-select・effects-replace) に一致するか (照合テストの拡張の後)。
- ビュー変換のケースを参照実装の CLI では描けないこと (CLI にビューの指定が無い)。

### 後続の候補

- ポーズのエントリを選ぶ操作と効果の差し替えの ffi (いまは Rust だけ・効果の差し替えは wasm にもある)。ポーズの選択は wasm にも無い。
- 時間の例に視線 (`look_at`) の操作を入れるか (アプリケーションの規則で、フレームの順に位置が無い)。
- `ugoe render` の CLI にビュー変換 (鏡映・回転) の指定を足せば、render-reference.ts で view のケースも黒箱で比べられる。

- 参照実装の側 (crates/ugoe/tests/spec_timelines.rs と crates/ugoe-render/tests/spec_render.rs の拡張) と docs/spec の同期は team-lead が足した (866b0a1・6841a49)。新しい例とケースはすべて参照実装と一致した。
