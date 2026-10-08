---
id: T0076
title: スタジオの GPU の設定 (使う GPU を選ぶ・ほかのアプリに譲る・描く量で負荷を抑える) を調べてタスクにする
status: in-progress
milestone: M8
crates: [docs]
depends_on: []
assignee: claude-worker-T0076
---

# T0076: スタジオの GPU の設定 (使う GPU を選ぶ・ほかのアプリに譲る・描く量で負荷を抑える) を調べてタスクにする

## 目的

利用者の要望 (2026-10-08): スタジオで GPU の優先度を変えられるようにしたい。利用者が選んだ中身は次の 3 つ。

1. **使う GPU を選ぶ:** ノート PC などで、外付け GPU (高性能) と内蔵 GPU (省電力) のどちらで描くかを選ぶ。
2. **ほかのアプリに GPU を譲る:** ゲームや OBS の描画を邪魔しないよう、スタジオの描画の優先度を下げる。
3. **描く量で負荷を抑える:** フレームレートの上限・描く解像度・アンチエイリアスなどを下げ、GPU の使用量そのものを減らす。

スタジオの描画が今どこで行われているか (WebView の中の wasm の WebGL2 / WebGPU、出力ウィンドウ、ブラウザソース、仮想カメラの読み戻し) によって、
できることが OS と WebView (Windows の WebView2・macOS の WKWebView・Linux の WebKitGTK) ごとに変わる。調べてから、実現できる形でタスクにする。

## 受け入れ条件

- [ ] 今のスタジオの描画の経路 (どのウィンドウ・どの API (WebGL2 / WebGPU)・アダプタの選び方・読み戻し) と、すでにある設定 (「フレームレートと負荷」のパネルなど) を調べてメモに書いた
- [ ] 3 つの中身ごとに、OS と WebView ごとに取れる手段 (例: WebGPU の `powerPreference`・WebGL の `powerPreference` の属性・Windows のアプリごとのグラフィックスの設定・プロセスの優先度・D3D12 / Metal の優先度・WebView2 の起動の引数・フレームレートの上限・描く解像度の倍率) と、
      効く範囲・制約・要る依存や unsafe を、出典 (公式の文書など) の URL と一緒にメモに書いた
- [ ] 実現できるものを docs/tasks/README.md の形でタスクにした (milestone は M8 の後続。受け入れ条件は検証できる形。実機でしか確かめられないことは「確かめていないこと」として分ける)
- [ ] 依存の追加・unsafe・OS の設定を書き換える操作・ADR が要るものは、本文に「判断が要る」と書いて案と推奨を並べた
- [ ] docs/roadmap.md の M8 の後続のタスクに起票した ID を書いた
- [ ] `node scripts/tasks.mjs` と `just ci` が通る

## 参考

- docs/studio.md、docs/adr/0028-studio.md、apps/studio、packages/player、crates/ugoe-wasm、docs/portability.md
- リリース前なので、移行の説明や互換の別名は要らない (利用者の方針)

## メモ

- 利用者の要望で起票した (2026-10-08)。
