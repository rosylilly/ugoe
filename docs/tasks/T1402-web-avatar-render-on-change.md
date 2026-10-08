---
id: T1402
title: "@ugoe/runtime: <ugoe-avatar> も、形が変わらない (パラメータが変わらず物理が止まっている) 間は描かない (判断が要る)"
status: todo
milestone: M8
crates: [bindings/web, ugoe-wasm, ugoe, docs]
depends_on: [T1401]
assignee:
---

# T1402: @ugoe/runtime: <ugoe-avatar> は変わったときだけ描く

## 目的

スタジオの受け手 (出力ウィンドウ・ブラウザソース) は、新しいフレームか見せ方の変化があったときだけ描くようにした (T0857)。
`<ugoe-avatar>` は、時間を進めている間 (`paused` でなく画面に見えている) は `requestAnimationFrame` のたびに `update` と `render` を呼ぶ
(bindings/web/src/avatar.ts の `#tick`)。待機のモーション・目パチ・呼吸の無いモデルや、それらを止めたページでは、形が変わらないのに
画面のリフレッシュレートで描き続け、ページに埋め込む用途で GPU と電池を無駄に使う。形が変わらない間は描かない。

**判断が要る (公開 API と、「変わらない」の決め方)。** 作業者は下の推奨で進めてよいが、公開の API の形を変えるなら利用者に確かめる。

### 決めること

1. **「形が変わらない」をどこで決めるか:**
   - 案 A: `ugoe` の `Puppet` に、前の `update` から評価の入力 (パラメータの値・パーツの不透明度・物理の出力) が変わったかを返す口を足す
     (例: `Puppet::update` の後に読む `changed()`)。物理は出力の変化が小さな閾値を下回ったら「止まっている」とみなす。wasm でも同じ口を出す。
   - 案 B: `<ugoe-avatar>` の中で、`update` の前後のパラメータの値を wasm から読んで比べる (物理の出力もパラメータなので含まれる)。
     `ugoe` は変えないが、毎フレームにパラメータの数だけ境界を越えて値を読む。
   **推奨:** A (判断を下の層に置き、FFI・Unity・Godot のホストも同じ口で描くのを省ける。比べるのは `Puppet` の中で安い)。
2. **閾値:** 物理の出力の変化がどれだけ小さければ止まったとみなすか (見た目で区別できない大きさ。例: パラメータの範囲の 1e-4)。
   閾値を下回ったまま描かない間も時間 (物理) は進め、動き出したらすぐ描く。
3. **公開の形:** `<ugoe-avatar>` の振る舞いの変更 (描く回数が減る) で、利用者の操作は要らない。`WebPuppet` を直接触ったら `requestRender()` を呼ぶ今の約束は変えない。

## 受け入れ条件

- [ ] 決めたことと理由を bindings/web/README.md (描画のループ) に書いた。A なら docs/spec/ (評価の入力が変わったかの定義) と crates/ugoe/README.md、
      crates/ugoe-wasm/README.md に口を書いた (英語の公開仕様 `spec/` に対応する節があれば同じ PR で直す)
- [ ] `<ugoe-avatar>` の `#tick` は、時間を進めていても、形が変わらず見せ方 (大きさ・ビュー・背景・光源・着せ替え・GPU の作り直し) も変わらない
      フレームでは `render()` を呼ばない。`requestRender()`・属性の変更・トラッキングの入力のあとは描く
- [ ] テスト: 動かない (モーション・目パチ・呼吸・物理の無い) モデルで `render()` が最初の 1 回の後に呼ばれないこと、物理が揺れている間は呼ばれ、
      止まったら呼ばれなくなること、パラメータを変える・`requestRender()`・大きさを変えると呼ばれること。A なら `ugoe` の単体テスト
      (フィクスチャで、入力が変わらなければ「変わっていない」・物理が閾値を下回ったら「変わっていない」)
- [ ] `just wasm-check` と `just ci` が通る

## 参考

- T0857 (スタジオの受け手は変わったときだけ描く) のメモ、docs/studio.md#出力ウィンドウ
- bindings/web/src/avatar.ts (`#tick`・`#animating`・`requestRender`)、crates/ugoe/src/lib.rs (`Puppet::update`)、crates/ugoe-wasm/src/web.rs、
  docs/spec/evaluation.md・docs/spec/physics.md

## メモ

- T0077 で起票した (元の候補: T0857 (`<ugoe-avatar>` も、パラメータが変わらず物理が止まっているときは描かない))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- T1401 と同じ avatar.ts を触るので、T1401 の後にした。T1400 も ugoe-wasm を触るが、足す場所 (`@internal` の問い合わせと評価の口) が別なので依存は付けていない。
- WebGPU・WebGL2 の canvas は、描かなければ前の画を出し続ける (T0857 のメモ)。
- 判断 (2026-10-08): 利用者が案 A (ugoe の Puppet に前の update から変わったかを返す口を足し、wasm にも出す) を選んだ。
