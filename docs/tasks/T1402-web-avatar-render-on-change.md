---
id: T1402
title: "@ugoe/runtime: <ugoe-avatar> も、形が変わらない (パラメータが変わらず物理が止まっている) 間は描かない (判断が要る)"
status: done
milestone: M8
crates: [bindings/web, ugoe-wasm, ugoe, docs]
depends_on: [T1401]
assignee: claude-worker-T1402
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

- [x] 決めたことと理由を bindings/web/README.md (描画のループ) に書いた。A なら docs/spec/ (評価の入力が変わったかの定義) と crates/ugoe/README.md、
      crates/ugoe-wasm/README.md に口を書いた (英語の公開仕様 `spec/` に対応する節があれば同じ PR で直す)
- [x] `<ugoe-avatar>` の `#tick` は、時間を進めていても、形が変わらず見せ方 (大きさ・ビュー・背景・光源・着せ替え・GPU の作り直し) も変わらない
      フレームでは `render()` を呼ばない。`requestRender()`・属性の変更・トラッキングの入力のあとは描く
- [x] テスト: 動かない (モーション・目パチ・呼吸・物理の無い) モデルで `render()` が最初の 1 回の後に呼ばれないこと、物理が揺れている間は呼ばれ、
      止まったら呼ばれなくなること、パラメータを変える・`requestRender()`・大きさを変えると呼ばれること。A なら `ugoe` の単体テスト
      (フィクスチャで、入力が変わらなければ「変わっていない」・物理が閾値を下回ったら「変わっていない」)
- [x] `just wasm-check` と `just ci` が通る

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
- やったこと (2026-10-08、案 A):
  - `ugoe`: `Puppet::changed()` を足した (`src/shape.rs`)。`update`・`follow` の後に、評価の出力 (頂点位置・不透明度・色・マスクの効き具合・
    表示・描画順) を **最後に `true` を返したときの出力の控え** と許容誤差付きで比べる。控えと比べるので、1 フレームでは許容誤差を下回る
    ゆっくりした変化も積もれば報告する。直前に見た出力のちょうど 1 つ後で変更フラグが何も立っていなければ比べない (止まったモデルの費用は
    変更フラグの確認だけ)。モデルの差し替え (着せ替え・`ModelInstance` の id の変化) は変わったとする。
  - タスクの案 A は「評価の入力 (パラメータ・パーツの不透明度・物理の出力) を比べる」と書いていたが、**評価の出力で比べた**。理由:
    動的物理の加算量 (パラメータでない揺れ)・着せ替え・ホストの `instance_mut` の書き換え・受け手の `follow` を 1 つの規則で扱え、
    閾値を見た目の単位 (画素・8 ビットの段) で決められ、描画に効かないパラメータの変化では描かないため。物理専用の閾値は持たない
    (物理の出力も評価の出力に現れる)。公開の口の形 (`Puppet::changed`) は案 A のまま。
  - 閾値: 頂点位置はキャンバスの長い辺 × 1e-5 (画面に収めた表示で 4K でも 0.04 画素。f32 の座標の刻みの約 100 倍で、収まりかけた
    物理の端数を拾わない)、不透明度・色・マスクの効き具合は 1e-3 (8 ビットの 1 段の 4 分の 1)。理由は docs/spec/evaluation.md#見た目の変化。
  - `ugoe-wasm`: `WebPuppet::shape_changed()` を足した (公開の API)。`update`・`follow` の `Puppet::changed` を `render` まで積む
    (`Puppet::changed` は最後に報告した出力と比べるので、報告を見て描かなかったことを wasm が覚える)。評価待ち (`dirty`)・読み込み・
    外した後も `true`。`render` が描けなかった (サーフェスの作り直し・時間切れ) ときは下ろさない。スタブ (`bindings/web/stub/ugoe_wasm.d.ts`) も直した。
  - `<ugoe-avatar>`: `#tick` は `update(dt)` の後、`requestRender()` の印か `shape_changed()` が真のときだけ `render()` を呼ぶ。大きさ・属性・
    `setParameter`・トラッキング・読み込み・GPU の作り直しは以前から `requestRender()` を呼んでいるので、見せ方の変化では描く。
  - テスト: `ugoe` の単体テスト 8 つ (止まったモデル・パラメータの変化・許容誤差の内の変化と積もり・`dynamic_hair` の物理が揺れている間と
    収まった後 (描かない間も出力は動いていて時間が進むこと・動き出したフレームですぐ変わること)・`update` の外の評価・着せ替え・`follow`・NaN)。
    `<ugoe-avatar>` の vitest 5 つ (止まったモデルは 1 回だけ描き時間は進む・揺れている間だけ描く・パラメータ/`requestRender`/トラッキング/
    大きさの変化で描く・`paused` の間のパラメータの変化)。前の `#tick` (毎フレーム描く) では 5 つとも落ちることを確かめた。
  - 文書: docs/spec/evaluation.md#見た目の変化 (定義と閾値の理由)、docs/spec/rendering.md#描画の省略、spec/rendering.md の Implementation notes
    (informative の 1 項)、docs/spec/public-spec.md の対応表、crates/ugoe/README.md、crates/ugoe-wasm/README.md#描くのを省く-t1402、
    bindings/web/README.md#描画のループ (決めたことと理由。canvas を写すときの注意も)。
  - 範囲外の小さな変更: `tests/conformance/web/avatar.js` の `expectDrawn` を、要素が描いた直後 (`render` を包んで同じタスク) に canvas を
    写すようにした。要素が描かないフレームの後では WebGL2 の描画バッファが合成の後に空で (preserveDrawingBuffer: false)、2 フレーム後に写す
    今までのやり方では 10 中 8 のシナリオが「canvas が空」で落ちたため (表示そのものは前の画のまま)。
  - 検査: `just ci`・`just wasm-check`・`just nostd-check`・`just sdk-no-render-check`・`just conformance --only avatar,example` (avatar 10/10・
    example ok)・`just wasm-types` (スタブと、`just wasm` で作った生成物の両方と比べて一致)・`just perf` (生成のモデルの `update` の中央値
    54.8µs / 上限 300、`avatar.ugoe` 21.1µs / 上限 100。docs/testing.md の T0190 の値 54.5・18.0 とほぼ同じ。毎フレーム形が変わる例なので、
    比較と控えのコピーの分 (生成のモデルの `follow` の最速で約 4µs) が増える)。
- やらなかったこと:
  - C ABI (`ugoe_puppet_changed`)・Unity・Godot への口 (`crates:` の外)。`@ugoe/player` (エディタ・ビューア・スタジオ) の描画のループでも使っていない。
  - 実機のブラウザで電力・GPU の使用率が下がることは測っていない (ヘッドレスの Chrome の適合テストで描く・描かないの振る舞いだけを確かめた)。
- 起票の候補:
  - C ABI に `ugoe_puppet_changed` を足し、Unity・Godot のバインディングでも形が変わらないフレームの描画を省く。
  - `@ugoe/player` (ビューアのループ・エディタのプレビュー) でも `shape_changed` で描くのを省く (スタジオの受け手は T0857 で別の仕組み)。
