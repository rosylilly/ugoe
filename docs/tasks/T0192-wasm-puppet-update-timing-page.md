---
id: T0192
title: "性能: wasm (ブラウザ) で Puppet の 1 フレームの評価を測る計測のページを足し、1ms の目標と比べる"
status: done
milestone: M7
crates: [ugoe, bindings/web, docs]
depends_on: []
assignee: claude-worker-T0192
---

# T0192: 性能: wasm (ブラウザ) で 1 フレームの評価を測る

## 目的

M7 の性能の目標「1 体を 1ms 以内 (中程度の PC、数千頂点) で評価する」は、ネイティブでは `puppet_timing` (T0190) で測り、
手元の機械で約 0.05ms と確かめた。Web の組み込み (`@ugoe/runtime`・スタジオの出力ウィンドウ・ブラウザソース) では同じ評価を
wasm で回すが、wasm (ブラウザ) での時間は測っていない (T0190 の「確かめていないこと」)。同じ生成のモデルと見本のモデルで、
ブラウザの中の `WebPuppet.update` を測れるようにし、1ms の目標と比べる。

「評価」は T0190 と同じく `update(dt)` の 1 回 (描画・GPU への転送・ファイルの読み込みは含めない)。

## 受け入れ条件

- [x] `puppet_timing` の生成のモデルを、例から `.ugoe` に書き出せる (例: `cargo run --release -p ugoe --example puppet_timing -- --write <path>`。
      `ugoe-model` の公開 API とフィクスチャは増やさない。書き出したモデルは検査 (`validate`) を通る)
- [x] ブラウザで開く計測のページ (例: `bindings/web/examples/timing/`) を足した。書き出した生成のモデルと `assets/samples/avatar.ugoe` を読み、
      `WebPuppet.update` (入力を毎フレーム動かす。`puppet_timing` と同じくトラッキング相当のパラメータの書き込みとモーションの再生を含む) を
      一定のフレーム数ずつ測り、平均のうち最も速いものと中央値を表で出す。頂点の数・ブラウザ (`navigator.userAgent`)・描画の API も出す
- [x] 手元の機械のヘッドレスの Chrome (tests/ui-audit と同じもの) で測った結果を、docs/testing.md#1-フレームの評価 にネイティブの値と並べて書き、
      1ms の目標を満たすかと、ネイティブとの比 (wasm の遅さ) を書いた。満たさなければ重い段 (評価・物理など) をメモに書き、改善の候補を「起票の候補」に書いた
- [x] 計測のページの開き方 (ビルドと配り方) を bindings/web/README.md か docs/testing.md#性能の計測 の表に書いた
- [x] `just ci` が通る

## 参考

- T0190 のメモ (結果・確かめていないこと・起票の候補)、docs/testing.md#性能の計測・#1-フレームの評価
- crates/ugoe/examples/puppet_timing.rs、bindings/web/examples/basic (ページの置き方)、crates/ugoe-wasm/README.md

## メモ

- T0077 で起票した (元の候補: T0190 (wasm (ブラウザ) で同じモデルの `update` を測る例か計測ページ))。
- M7 の後続 (docs/roadmap.md の M7 の性能の行)。M7 の完了の条件には含めない。
- 時間の上限の検査 (T0191 の `just perf`・perf.yml) に wasm を入れるかは、測った値の揺れを見てから決める (このタスクでは入れない)。
- 確かめられないこと: 中程度の PC (ノート PC など) の実機のブラウザ、Safari・Firefox。測った範囲をメモに書く。
- やったこと (T0192):
  - `crates/ugoe/examples/puppet_timing.rs` に `--write <path>` を足した。測らずに生成のモデル (物理・モーション `idle`・
    1 画素の白の PNG のテクスチャ) を `.ugoe` に書き、読み直して (`read_bundle` が `validate`・`validate_physics` を通す) 書いたものと
    同じことを確かめる。`ugoe-model` の公開 API とフィクスチャは増やしていない。
  - `bindings/web/examples/timing/` を足した: `index.html` (計測のページ。`update`・受け手の `follow`・物理を止めた `update` を
    200 フレームずつの平均の最速と中央値で表にし、頂点の数・`navigator.userAgent`・描画の API・アダプタ・`crossOriginIsolated`・
    タイマーの刻みも出す。結果は `window.ugoeTiming`)、`serve.mjs` (依存なしの静的サーバー。`bindings/web` の `dist/` をそのまま配り、
    COOP と COEP で cross-origin isolated にして `performance.now()` を 5µs の刻みにする)、`measure.mjs` (ルートの playwright-core で
    tests/ui-audit と同じ Chrome をヘッドレスで起動し、`--runs` 回の表と中央値、load average を出す)。生成のモデルは gitignore。
  - 結果を docs/testing.md#1-フレームの評価 にネイティブと並べて書き、性能の計測の表に行を足した。bindings/web/README.md に
    「計測のページ」の節、crates/ugoe/README.md に `--write`、docs/roadmap.md の M7 の性能の行に結果を足した。
- 結果 (Ryzen 9 7900・WSL2、ヘッドレスの Chrome 154・WebGL2 (SwiftShader)・cross-origin isolated。ほかのワーカーのビルドが並行して
  load average 8〜9。5 回開き直した中央値、µs): 生成のモデル `update` 69.3 (最速 64.8)・`follow` 69.5・物理を止めた `update` 48.2、
  `avatar.ugoe` `update` 17.9・`follow` 18.8・物理を止めた `update` 16.7。同じ時刻のネイティブ (3 回の中央値) は 54.9 と 18.1。
  1ms の目標を満たす (生成のモデルで約 7%)。wasm / ネイティブは生成のモデルで約 1.3 倍、`avatar.ugoe` で約 1.0 倍。
  物理を止めるとネイティブとほぼ同じ (48 と 47µs) で、差の大半は物理の段 (wasm 約 21µs、ネイティブ約 8µs)。
- 揺れ: 回ごとの生成のモデルの `update` の中央値は 67.6〜87.1µs (load average 8〜9)。フレームの数を 1,200 に減らすと 113µs と遅く出た
  (V8 の Liftoff から TurboFan への切り替えの前のフレームが混じるため。docs に書いた)。
- 確かめていないこと: 中程度の PC (ノート PC など) の実機のブラウザ、Safari・Firefox、WebGPU を有効にした Chrome (ヘッドレスでは
  `auto` が WebGL2 になった。`update` は CPU だけなので描画の API は効かない見込み)、cross-origin isolated でないページ (100µs の刻み)。
- `just ci` は通った (rebase の後。load average 17 前後)。
- 起票の候補:
  - wasm の物理の段がネイティブの約 2.5 倍遅い (生成のモデルで約 21µs と約 8µs) 理由を調べる (三角関数などの浮動小数点の関数が wasm で
    遅い見込み)。目標の 1ms には遠いので急がない。
  - 時間の上限の検査 (`just perf`・perf.yml) に wasm を入れるか: 回ごとの中央値の揺れは手元で 3 割ほど (67〜87µs)。入れるなら
    `measure.mjs` に上限の判定を足し、CI の Chrome の入ったジョブで流す (上限はネイティブと同じ 300µs が目安)。
