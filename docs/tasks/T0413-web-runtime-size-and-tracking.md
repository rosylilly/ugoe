---
id: T0413
title: npm: wasm の読み込みコードの縮小・エディタとビューアの wasm のプロファイル・トラッキングの平滑化
status: done
milestone: M7
crates: [bindings/web, ugoe-wasm, ugoe-format, docs]
depends_on: [T0410]
assignee: claude-worker-T0413
---

# T0413: npm パッケージの後続

## 目的

T0410 の後続の候補をまとめる。

## 受け入れ条件

- [x] MessagePack (`rmp_serde`) の読み込みコード (約 0.8MB) の大きさを見直す。読み込みの結果と互換性テストは変えない
- [x] エディタとビューアの wasm にも、パッケージと同じビルドのプロファイル (大きさ優先・毎フレーム動くクレートは速さ優先) を使うかを決める。使うなら数字を書く
- [x] トラッキングの平滑化とキャリブレーションを、MIT の別の部品として `bindings/web` に置くかを決める (GPL の `packages/face-tracking` は使えない。ADR 0025)
- [x] `assets/samples/` の見本にモーションを入れるかを決める (いまは `bindings/web` の見本だけが生成している)
- [x] `just ci`・`just wasm-check` が通る

## 参考

- `docs/tasks/T0410-web-runtime-release.md` のメモ、`docs/adr/0025-web-runtime-package.md`

## メモ

- **MessagePack の読み込み**: 見直した結果、**`rmp_serde` のまま**にした (利用者の判断)。
  - 試したこと: 読み込みの結果 (受け付ける入力・拒否する入力・読んだ値) を `rmp_serde` 1.3 と同じに保つ自前の `Deserializer` を ugoe-format に書き
    (書き出しは `rmp_serde` のまま)、フィクスチャ・仕様の例・見本と互換性テストと仕様の例のバンドル・それらを壊した入力 (一度だけ約 60 万個) で
    両方を比べて食い違いが無いことを確かめた。enum を array `[バリアント]` で書いたものを `rmp_serde` は拒否する、などの細部まで揃える必要があった。
  - 数字 (バイト。Node の zlib): パッケージの wasm は 4,455,303 → 3,947,331 (生で −11%)、gzip 1,301,592 → 1,269,809 (**−2.4%**)、
    brotli 964,869 → 948,050 (−1.7%)。繰り返しの多いコードだったので圧縮後はあまり減らない。読み込みはネイティブで約 0.39ms → 約 0.46ms
    (見本の `MODL` 122KB)。twiggy で見ると、読み込みまわりは約 1.15MB → 約 0.63MB (残りはモデルの型の `Deserialize` そのもの)。
  - **入れなかった理由**: 外から来る入力を読む部分を自前で持つ負担 (`rmp_serde` との振る舞いの一致を保ち続ける) に対して、
    転送の大きさの効果 (gzip で 2.4%) が小さい。いったんコミットした (11bdb00) が revert した。数字は `bindings/web/README.md#wasm-の大きさ` にも残した。
- **エディタとビューアの wasm のプロファイル**: **今のまま (release・名前あり) にする**と決めた。scripts/wasm.mjs・justfile は変えていない。
  理由と数字は `bindings/web/README.md#エディタとビューアの-wasm-は今のまま-t0413` (アプリの中の wasm でネットワークを通らない、
  V8 のコンパイルは約 5.9ms と約 4.9ms で差が小さい、名前の節は開発中に要る、fat LTO で作り直しが約 4.1 → 7.3 秒)。
  ビューアのブラウザ版を Web に置くようになったら、そのビルドには `web-dist` を使う。
- **トラッキングの平滑化とキャリブレーション**: `@ugoe/runtime` の中に MIT で置いた (`bindings/web/src/tracking-filter.ts`。ADR 0025 に追記)。
  `TrackingFilter` (値ごとの One Euro フィルタ、途切れた値は忘れる、`reset()`)・`captureNeutral` (平均)・`calibrateTracking`
  (回転は中立を引く、ブレンドシェイプは `(v − n) / (1 − n)`、体のピッチと肩の上下は中立が無ければ渡さない)。
  GPL の `packages/face-tracking` は読まずに、論文の定義から書いた。Vitest で 9 件。見本の合成のトラッキングに揺れを足して
  `TrackingFilter` を通した (`?tracking-filter=off` で比べられる)。ヘッドレス Chrome (WebGL2) で、頭のヨーのフレームごとの 2 階差分の平均が
  平滑化なし 3.63 → あり 0.52 になった。
- **見本のモデルにモーションを入れるか**: **入れる方がよい**と決めた (後続タスク)。`<ugoe-avatar motion="...">` と `expression` の属性が
  見本のモデルだけで試せるようになり、`examples/basic/make-motion-sample.mjs` (cargo でビルドする手間がかかる) が要らなくなる。
  モーション・表情は小さい (見本の `generated/avatar-motion.ugoe` で +1.3KB)。入れるには `ugoe sample avatar` (ugoe-cli) の変更と
  `assets/samples/` の作り直しが要り、範囲外なので行っていない。
- 確かめたこと: `just ci`・`just wasm-check` (revert の後も)・`just conformance` (C ABI と wasm の 46 件。revert の前)。`pnpm pack` した tarball を見本に入れ、
  ヘッドレス Chrome で WebGL2・WebGPU (SwiftShader) の両方で `<ugoe-avatar>` と `createPuppet` が描画できること、モーション入りの
  見本 (`npm run sample:motion`) で `motion="nod"`・`expression="smile"` が動くことを確かめた
  (revert の前の wasm で。スクリーンショットは scratchpad の `t0413/shots/`)。
- **確かめていないこと**: 本物のカメラと MediaPipe での平滑化とキャリブレーションの効き (合成の入力と単体テストだけ。既定の係数は
  一般的な値から選んだもので、実際の顔で調整していない)、本物の GPU、Firefox・Safari。
- **範囲外の変更**: なし。
- **後続の候補**: 見本のモデル (`assets/samples/`) にモーションと表情を入れる (ugoe-cli の `sample`)。`serde_json` (約 0.37MB) の見直し
  (wasm で JSON を読む型を減らす)。平滑化の既定の係数を実際の顔で調整する。
