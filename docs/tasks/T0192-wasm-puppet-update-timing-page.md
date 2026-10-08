---
id: T0192
title: "性能: wasm (ブラウザ) で Puppet の 1 フレームの評価を測る計測のページを足し、1ms の目標と比べる"
status: todo
milestone: M7
crates: [ugoe, bindings/web, docs]
depends_on: []
assignee:
---

# T0192: 性能: wasm (ブラウザ) で 1 フレームの評価を測る

## 目的

M7 の性能の目標「1 体を 1ms 以内 (中程度の PC、数千頂点) で評価する」は、ネイティブでは `puppet_timing` (T0190) で測り、
手元の機械で約 0.05ms と確かめた。Web の組み込み (`@ugoe/runtime`・スタジオの出力ウィンドウ・ブラウザソース) では同じ評価を
wasm で回すが、wasm (ブラウザ) での時間は測っていない (T0190 の「確かめていないこと」)。同じ生成のモデルと見本のモデルで、
ブラウザの中の `WebPuppet.update` を測れるようにし、1ms の目標と比べる。

「評価」は T0190 と同じく `update(dt)` の 1 回 (描画・GPU への転送・ファイルの読み込みは含めない)。

## 受け入れ条件

- [ ] `puppet_timing` の生成のモデルを、例から `.ugoe` に書き出せる (例: `cargo run --release -p ugoe --example puppet_timing -- --write <path>`。
      `ugoe-model` の公開 API とフィクスチャは増やさない。書き出したモデルは検査 (`validate`) を通る)
- [ ] ブラウザで開く計測のページ (例: `bindings/web/examples/timing/`) を足した。書き出した生成のモデルと `assets/samples/avatar.ugoe` を読み、
      `WebPuppet.update` (入力を毎フレーム動かす。`puppet_timing` と同じくトラッキング相当のパラメータの書き込みとモーションの再生を含む) を
      一定のフレーム数ずつ測り、平均のうち最も速いものと中央値を表で出す。頂点の数・ブラウザ (`navigator.userAgent`)・描画の API も出す
- [ ] 手元の機械のヘッドレスの Chrome (tests/ui-audit と同じもの) で測った結果を、docs/testing.md#1-フレームの評価 にネイティブの値と並べて書き、
      1ms の目標を満たすかと、ネイティブとの比 (wasm の遅さ) を書いた。満たさなければ重い段 (評価・物理など) をメモに書き、改善の候補を「起票の候補」に書いた
- [ ] 計測のページの開き方 (ビルドと配り方) を bindings/web/README.md か docs/testing.md#性能の計測 の表に書いた
- [ ] `just ci` が通る

## 参考

- T0190 のメモ (結果・確かめていないこと・起票の候補)、docs/testing.md#性能の計測・#1-フレームの評価
- crates/ugoe/examples/puppet_timing.rs、bindings/web/examples/basic (ページの置き方)、crates/ugoe-wasm/README.md

## メモ

- T0077 で起票した (元の候補: T0190 (wasm (ブラウザ) で同じモデルの `update` を測る例か計測ページ))。
- M7 の後続 (docs/roadmap.md の M7 の性能の行)。M7 の完了の条件には含めない。
- 時間の上限の検査 (T0191 の `just perf`・perf.yml) に wasm を入れるかは、測った値の揺れを見てから決める (このタスクでは入れない)。
- 確かめられないこと: 中程度の PC (ノート PC など) の実機のブラウザ、Safari・Firefox。測った範囲をメモに書く。
