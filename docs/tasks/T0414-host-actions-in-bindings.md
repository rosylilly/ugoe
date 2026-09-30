---
id: T0414
title: バインディング: ポーズの表示の選択・効果の差し替えを C ABI と wasm に出す・CLI のビュー変換
status: done
milestone: M7
crates: [ugoe-ffi, ugoe-wasm, ugoe-cli, spec, docs, bindings/web]
depends_on: [T0130, T0412]
assignee: claude-worker-T0414
---

# T0414: ホストの操作をバインディングに揃える

## 目的

T0130 の後続の候補をまとめる。公開仕様のホストの操作 (spec/motion.md) のうち、参照実装の Rust の API にしかないものがある。

## 受け入れ条件

- [x] ポーズの表示の選択 (`PosePlayer::show`、パーツ ID で指定) を C ABI と wasm に出す
- [x] 効果の設定の差し替え (`Puppet::set_effects`) を C ABI に出す (wasm は set_effects_json がある)
- [x] `ugoe render` にビュー変換 (6 数) の指定を足し、描画の例の view のケースを黒箱でも比べられるようにする (tools/spec-check/src/render-reference.ts はワーカーが spec/ だけを見て直す別の作業でもよい)
- [x] ビュー変換が退化したとき (逆変換が無い) の点光源の扱いを仕様で決めるか、適合の対象外と書く (spec/rendering.md と docs/spec の同期)
- [x] 視線 (look_at) を時間の例に入れるかを決める
- [x] C ABI からパーツの不透明度を読む API を足す (T0412 の後続の候補)
- [x] 着せ替えのモジュールを付けている間に差し替えた物理の設定が、次の付け外しで本体の設定から作り直されて失われる (wasm と C ABI で同じ)。保つかどうかを決める (T0412 の後続の候補)
- [x] `just ci`・`just wasm-check`・`just ffi-check` が通る

## 参考

- `docs/tasks/T0130-spec-tracking-timeline.md` のメモ

## メモ

- **C ABI (ABI の版は 1 のまま。足しただけ):** `src/part.rs` に `ugoe_puppet_part_count` / `_part_id` / `_part_index`、
  `ugoe_puppet_part_opacity` (`UgoePartOpacity { value, weight }`)、`ugoe_puppet_show_pose` (part の ID。要素でなければ `*out_shown = false`
  でエラーにしない)。`src/animation.rs` に `ugoe_puppet_set_effects_json` (モデルの `effects` の JSON。読めなければ `UGOE_ERROR_FORMAT`)。
  JSON を読むため ugoe-ffi に `serde_json` (ワークスペースにある) を足した。ヘッダは `just ffi-header` で作り直し、C の見本でも呼ぶ。
- **パーツの不透明度** は「直前の update が書いた上書き (value, weight)」を返す (公開仕様の時間の例の `part_opacity` と同じもの。
  適合テストで照合できる)。キーフォームと親を掛けた評価後の値は `ModelInstance` が公開していない (ugoe-runtime は範囲外) ので出していない。
- **wasm:** `WebPuppet::show_pose(part) -> bool` (`animation::show_pose` でネイティブのテスト)。
- **CLI:** `ugoe render --view A,B,C,D,E,F` (`--fit` と排他。描画の例の `view` と同じ意味)。`prepare` の後に `Prepared::view` を差し替える
  (ugoe-headless は変えていない)。clippy の large_enum_variant のため `Cmd::Render` を `Box` にした。
  黒箱の比較: render-reference.ts を scratchpad に写して view のケースで `--view` を渡すようにし、全例・全ケースで 0 mismatches
  (PNG で比べられない 145 チャンネルは従来どおり飛ばす)。tools/spec-check は変えていない。
- **決めたこと:**
  - 退化したビュー変換: spec/rendering.md の Inputs と docs/spec/rendering.md#ビュー変換 に「逆変換の無いビューは三角形が画素の中心を覆わず
    何も描かないので、点光源の位置は要らない」と書いた。参照実装で確かめた (`--view 1,1,1,1,0,0` はマスク・点光源ありでも全画素 0)。
  - 視線: 時間の例の操作に入れない。視線の対応はアプリの推奨 (ランタイムの操作ではない) で、結果は `set` で書けるパラメータ値になるため。
    spec/examples/README.md の操作の表の後に書いた。
  - 着せ替え中に差し替えた物理: 保たない (今のまま)。合成した設定は本体の分とモジュールのリグの分に分けられず、本体として覚え直すと次の合成で
    リグが二重になるため。docs/spec/modules.md#puppet-での着せ替え、ffi と wasm の README に書いた。
- **テスト:** `crates/ugoe-ffi/tests/host.rs` (パーツの一覧、ポーズの選択と上書きが Rust の Puppet と一致、効果の差し替えで目パチ・呼吸・口が
  一致し seed を丸めない、エラー)。wasm の `show_pose_switches_the_shown_entry`。CLI の `--view` の引数と変換。
- **確認:** `just ci`・`just wasm-check`・`just ffi-check`・`just spec` (main 3397c9e に rebase 後)。`just test-gpu` は描画のコードを変えていないので
  回していない (CLI の描画は上の黒箱の比較で確かめた)。
- **範囲外 (未対応・team-lead に相談中):** `bindings/web/stub/ugoe_wasm.d.ts` に `show_pose(part: string): boolean;` を足す必要がある
  (bindings/web/README の規則)。返事が来る前に review にしたので、足していない。
- **確かめていないこと:** Windows と macOS での C の見本。ビューア・エディタの画面 (API を足しただけで呼んでいない)。
- **後続の候補:**
  - tools/spec-check/src/render-reference.ts で view のケースに `--view` を渡す (spec/ だけを見て直す作業)。
  - 評価後のパーツの不透明度を ugoe-runtime に公開し、C ABI と wasm から読む。wasm にもパーツの不透明度の上書きを読む API。
  - 着せ替え中も本体の物理の設定を差し替える SDK の API (`Puppet::set_base_physics` のようなもの。差し替えて合成し直す)。

- 範囲外: bindings/web/stub/ugoe_wasm.d.ts に show_pose の 1 行 (bindings/web/README.md の規則。team-lead 承認済み。review の後に届いたので team-lead が足した)。
