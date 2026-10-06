---
id: T0947
title: "MediaPipe の利用状況の送信: web の見本でも止め、版を上げたときに送り方の変化を自動で見つける"
status: todo
milestone: M9
crates: [bindings/web, packages/face-tracking, docs]
depends_on: []
assignee:
---

# T0947: MediaPipe の利用状況の送信: web の見本でも止め、版を上げたときに送り方の変化を自動で見つける

## 目的

T0943 で、`@ugoe/face-tracking` はランドマーカーを作る前に MediaPipe の利用状況の送信 (`odml.pa.googleapis.com`) を止めるようになった。残ったもの (T0943 の候補):

- `bindings/web` の見本 (`examples/basic`) の「camera (MediaPipe)」は今も送る。見本でも `fetch` を包むか CSP を付ける。
- MediaPipe の版を上げたとき、送り先と `fetch` の使い方が変わっていないかを手で確かめるだけになっている (README)。

**ライセンスの注意:** `bindings/web` は MIT。GPL の `packages/face-tracking` のコードを import したり写したりしない (AGENTS.md)。見本の止め方は自分で書くか、CSP にする。

## 受け入れ条件

- [ ] `bindings/web/examples/basic` で camera (MediaPipe) を使っても利用状況が送られない (`fetch` を包むか CSP)。止めていることを見本の README か
      bindings/web/README.md のトラッキングの説明に書いた
- [ ] `packages/face-tracking` に、入れている `@mediapipe/tasks-vision` の配布物 (`vision_bundle.mjs` など) を調べて、送り先 (`odml.pa.googleapis.com`) と
      送り方 (`fetch(`) が T0943 で止めた形のままかを確かめる検査 (Vitest かスクリプト) があり、変わると落ちる。`just ci` から走る
- [ ] packages/face-tracking/README.md の版を上げるときの手順を直した
- [ ] `just ci` が通る

## 参考

- T0943 のメモ (送信の仕組み・止め方・起票の候補)
- bindings/web/examples/basic、bindings/web/src/tracking.ts、packages/face-tracking

## メモ

- T0040 で起票した (元の候補: T0943 の 2 つ)。
