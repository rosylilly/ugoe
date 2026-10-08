---
id: T0858
title: "スタジオ: 出力ウィンドウとブラウザソースの描く解像度の倍率 (1・0.75・0.5) を選べるようにする"
status: done
milestone: M8
crates: [packages/player, apps/studio, docs]
depends_on: [T0857]
assignee: claude-worker-T0858
---

# T0858: スタジオ: 描く解像度の倍率

## 目的

利用者の要望「GPU の使用量を減らす (描く量で負荷を抑える)」の 2 つ目 (T0076 で調べた)。

出力ウィンドウとブラウザソースのページは、canvas の描画バッファを「表示の大きさ (CSS ピクセル) × devicePixelRatio」の画素で描く
(`packages/player/src/surface.ts`)。出力ウィンドウの大きさのプリセットは物理ピクセルなので、1920×1080 の出力では 1920×1080 画素を毎回描く。
4K の配信や重いモデル (法線マップ・光源) では、フラグメントの量が GPU の負荷の大半になる。

描画バッファを倍率 (1・0.75・0.5) だけ小さくして描き、canvas は CSS で表示の大きさに引き伸ばす。取り込まれる画は粗くなる (拡大でぼける) ので、
既定は 1 (今と同じ) にし、利用者が負荷と画質を見比べて選ぶ。

仮想カメラは解像度 (1920×1080・1280×720) を既に選べるので、この倍率の対象にしない。アンチエイリアスは、レンダラが MSAA を使っておらず
(`sample_count: 1`)、WebGL2 のコンテキストも `antialias: false` で作る (wgpu-hal の GL のバックエンド) ので、下げる設定は無い (T0076 のメモ)。

## 受け入れ条件

- [x] `PuppetSurface` に描画バッファの倍率 (0 より大きく 1 以下の有限の数。それ以外は 1 として扱う) を持たせ、描画バッファの画素を
      「CSS ピクセル × devicePixelRatio × 倍率」(1 画素以上、デバイスの上限まで) にする。ビュー (ズーム・パン)・`fit`・ポインタの座標の換算・
      作り直しで覚えるビューが倍率を含めて正しく換算されること (今の devicePixelRatio の換算と同じ所を通す) をテストで確かめた
- [x] Player に倍率を変える口を足し、変えたら描画バッファの大きさを合わせ直して描き直す (T0857 の印を立てる)。受け手のモードでなくても使える
      (エディタ・ビューアは使わない。既定の 1 では今と同じ)
- [x] スタジオのプロファイルの `output` に `renderScale` (`1`・`0.75`・`0.5`。省くか選べない値なら `1`) を足し、出力ウィンドウとブラウザソースの
      受け手が `scene` の通知 (か出力の様子) で受け取って当てる。プロファイルの形式の表 (docs/studio.md#プロファイルの形式) を直した
      (リリース前なので移行の説明は要らない)
- [x] 操作ウィンドウの「フレームレートと負荷」に「描く解像度」(100%・75%・50%) を足し、選ぶと次のフレームから効く。画質が落ちることを説明の文に書く。
      文言は `apps/studio/locales/*.json` (英語と日本語)
- [x] 出力ウィンドウ・ブラウザソースの画の確かめ (docs/studio.md#画の確かめ-adr-0028-の-10) を倍率 0.5 でも行い、ページどうし (出力ウィンドウと
      ブラウザソース) が一致することを確かめた (仮想カメラとは倍率 1 のときだけ比べる)
- [x] docs/studio.md の「出力ウィンドウ」「負荷の表示」と、docs/manual/studio.md の負荷の説明に、倍率と画質の関係を書いた
- [x] `just e2e --only studio` (パネルに部品を足すので) と `just ci` が通る

### 確かめていないこと (実機)

- 実際の GPU で、倍率を下げると GPU の使用率が下がること、OBS のウィンドウの取り込み・ブラウザソースで画がどのくらい粗く見えるか。
  docs/manual-check.md の「S. スタジオ」に手順を足す

## 参考

- packages/player/src/surface.ts (描画バッファの大きさ・devicePixelRatio の追従)、packages/player/src/player.svelte.ts
- docs/studio.md#出力ウィンドウ・#プロファイルの形式・#負荷の表示、apps/studio/src/LoadPanel.svelte
- T0076 のメモ (調べた結果)

## メモ

- T0076 で起票した (利用者の要望 2026-10-08「GPU の優先度」の 3. 描く量で負荷を抑える)。依存・unsafe・OS の設定は要らないので判断は要らない。
- T0857 (変わったときだけ描く) の印を使うので、その後に行う。
- 倍率の選択肢を 3 つに絞ったのは、細かく選べても見比べにくいため。要望があれば自由な値 (0.25〜1) にする。
- ブラウザソースは OBS が決めた大きさ (ソースの幅と高さ) で描くので、倍率はその大きさに掛かる。

### やったこと (claude-worker-T0858、2026-10-08)

- **`PuppetSurface.renderScale`** (packages/player/src/surface.ts): 0 より大きく 1 以下の有限の数 (それ以外は 1。`renderScaleOf`)。`pixelRatio` を
  「devicePixelRatio × 倍率」にしたので、描画バッファの大きさ・Player のビュー (`view`)・`zoomBy`・`panBy`・作り直しで覚えるビューは今までの
  devicePixelRatio の換算と同じ所を通る。上限はこれまでどおり wasm の `resize` (`max_texture_dimension_2d`) が決める。
  倍率を変えたら描画バッファを合わせ直し、パン (描画バッファの画素) を新しい換算に直して CSS ピクセルで同じ場所を見せる (`#rescale`)。
  - **範囲の少し外 (packages/player の中):** devicePixelRatio だけが変わったときも同じ `#rescale` を通すようにした。今まではパンが 0 でないと
    (利用者がパンした・スタジオの配置でずらした) 倍率の違う画面へ移したときに位置がずれていた (`player-render-scale` を足して分かった)。
    エディタのビューポートも同じ PuppetSurface なので、エディタでも直る。
- **`Player.renderScale`** (受け手のモードでなくても使える): 変えたら `#syncView` で印 (`#dirty`) を立てて描き直す。
- **スタジオ:** プロファイルの `output.renderScale` (1・0.75・0.5。選べない値は読むときに 1)、コマンド `set_render_scale` (選べない値は
  `render_scale.unsupported`)。受け手へは `scene` の通知の `renderScale` で配る。倍率はシーンの状態ではないので `revision` は上げず、`with_stage` の後に
  `Conductor::set_render_scale` で今のシーンを書き換え、変わったら同じシーンを出力ウィンドウとブラウザソースにだけ配り直す (操作ウィンドウと
  仮想カメラには送らない。読めなかったことの一覧も消えない)。接続したばかりの受け手には最初のシーンの通知で届く。Output.svelte は通知が届いたら
  すぐ `player.renderScale` に当てる。操作ウィンドウの「フレームレートと負荷」に「描く解像度」(100%・75%・50%) と説明の文。`renderHeavy` の注意の文も
  描く解像度を案内するようにした。
- **テスト:** surface.test.ts (大きさ・範囲の外・倍率を変えたときのパン・作り直しで覚えるビュー・devicePixelRatio の変化のパン)、player.test.ts
  (ビューとポインタの換算)、playerFollower.test.ts (倍率の変化で描く)、Output.svelte.test.ts (通知の倍率を当てる・倍率だけの配り直しで読み直さない)、
  settings.test.ts、profile.rs (読み書き・既定・収める)、api/tests.rs (`the_render_scale_reaches_the_pages_with_the_scene`: ページへの配り直し・
  同じ版・同じ値なら送らない・次のシーンと新しいプロファイル)。`just player-browser` に `player-render-scale` (本物の wasm。パンを直さないと落ちる
  ことを確かめた) を足し、テストのページの canvas をアプリと同じく CSS で引き伸ばすようにした。
- **画の確かめ (480×270、`UGOE_STUDIO_CHECK_RENDER_SCALE` を source_check.rs に足した):** 倍率 0.5 で回転なし・回転 8 度とも、ページどうし
  (出力ウィンドウとブラウザソース) は完全に一致 (描画バッファ 240×135・314×314)。仮想カメラとは倍率 1 で平均 0.445 (T0808 と同じ)、
  0.5 では平均 4.5 (縁のぼけ。差の画で位置がずれていないことを見た)。docs/studio.md#画の確かめ-adr-0028-の-10 に書いた。
- **文書:** docs/studio.md (出力ウィンドウの表と「描く解像度」・プロファイルの形式・シーンの切り替えと受け手・画の確かめ・負荷の表示)、
  docs/manual/studio.md (フレームレートと負荷・困ったとき)、docs/manual-check.md の S34 (T0860 の S33 と重なったので 34 にした)、docs/ui.md、
  docs/testing.md、packages/player/README.md、apps/studio/README.md。マニュアルの画像 `studio-load.png` を撮り直した (`just e2e --manual --only studio`
  で撮り、その 1 枚だけを 256 色にして写した。ほかの画像は変えていない)。
- **検査:** `just ci`・`just player-browser` (24 個)・`just e2e --only studio` (13 個)・`just ui-audit --only studio` (183 場面で 0 件) が通った
  (main (T0860 を含む) に rebase した後にもう一度流した)。

- **置き場所 (T0860 の機械ごとの設定とそろえるか):** プロファイルの `output.renderScale` のままにした (受け入れ条件どおり)。理由: 倍率は配信に出る画の
  解像度を決める出力の設定で、プロファイルの `output` にある出力の大きさ・刻みの速さ・仮想カメラの解像度と同じ種類 (人に渡したプロファイルで
  同じ画になる。T0827 の「localStorage に持たない」の考え)。使う GPU はその機械のハードウェアを指し、起動し直しも要るので機械ごと
  (`studio-settings.json`) にしたが、倍率はどの機械でも同じ意味で次のフレームから効く。docs/studio.md の「描く解像度」に書いた。
  T0860 は main に入った後に rebase してあり (f9d7cb65 を含む)、衝突 (LoadPanel・Control・host.ts・lib.rs・locales・capabilities・ui-audit の偽のホスト・
  manual-check の S33) は両方を残して解き、検査はその後に流し直した。

### やらなかったこと・確かめていないこと

- 実際の GPU で GPU の使用率が下がること、OBS のウィンドウの取り込み・ブラウザソースでの粗さの見え方 (manual-check の S34)。
- 仮想カメラは対象にしない (解像度を別に選べる)。倍率は 3 つに絞った (自由な値は要望があれば)。
- スクリーンショット (`Player.capture`) は描画バッファの大きさで読むので、倍率を下げると小さな画になる。スタジオの受け手は使わないので、そのままにした。

### 起票の候補

- なし (要望があれば、倍率を自由な値 (0.25〜1) にする)。
