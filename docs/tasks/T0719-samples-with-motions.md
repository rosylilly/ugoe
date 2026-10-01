---
id: T0719
title: 見本: 見本のモデルにモーションと表情を入れる・npm パッケージの serde_json の見直し・平滑化の係数
status: done
milestone: M4
crates: [ugoe-cli, assets, bindings/web, ugoe-wasm, docs]
depends_on: [T0413]
assignee: claude-worker-T0719
---

# T0719: 見本のモーションと npm パッケージの残り

## 目的

T0413 の後続の候補をまとめる。

## 受け入れ条件

- [x] 見本のモデル (assets/samples) にモーションと表情を入れる (ugoe sample の生成を直し、コミットした見本を作り直す。ビューアに再生の画面が無いので、ビューアでの再生は T0720 に回した)
- [x] npm パッケージの wasm の serde_json (約 0.37MB) を見直す
- [x] トラッキングの平滑化の既定の係数を、実際の顔で調整する (できなければ利用者の確認に残す)
- [x] `just ci`・`just wasm-check` が通る

## 参考

- `docs/tasks/T0413-web-runtime-size-and-tracking.md` のメモ

## メモ

- **見本のモーションと表情**: `crates/ugoe-cli/src/sample/animations/` に JSON で置き (手書き、MIT)、`include_str!` で読む
  (`sample::animations`)。モーションは `idle`・`nod`・`shake` (bindings/web の見本から移した) と `tilt` (新規。ベジェと `EyeBallX`)
  の 4 つで、見本のアバターと着せ替えの本体で共有する。表情はアバターが `smile`・`surprised`・`sad`・`angry`、着せ替えの本体が
  `smile`・`surprised`・`sad` (レベル 2 のパラメータだけ)。アバターはエディタと同じ `Command::AddMotion`・`AddExpression` で入れ、
  `avatar-lit` は `light_avatar` が引き継ぐ。着せ替えは `Bundle` に直接入れ、`save_projects` の本体のプロジェクトにも入れる。
  `animations::check` がモデルに無いパラメータを動かすものを誤りにする。`committed_sample_is_up_to_date` (アバター) に
  モーションと表情の比較を足し、着せ替えはバイト列の比較のまま。`avatar.ugoe`・`avatar-lit.ugoe`・`dress-up/dress-up.ugoe` を作り直した
  (それぞれ約 +3KB・+3KB・+2.4KB。モジュールは変わらない)。
- **見本の指紋が変わった**: ビューアが覚えたライティング・着せ替えの組 (`rememberedModels`・`dressUpMemory`) は、中身の指紋か
  場所 (`sample:avatar.ugoe` など) の鍵で戻す。3 つの見本は指紋が変わったので、作り直しの前に覚えたものは場所の鍵 (同梱の見本を
  「サンプル」から開いたとき) でしか戻らない。ファイルとして開いた見本の古い記憶は戻らない (指紋が違う別のモデルとして扱われる)。
- **bindings/web の見本の整理**: `examples/basic/make-motion-sample.mjs` と `sample:motion`・`motions/`・`expressions/`・`.gitignore` の
  `generated/` を消した (見本のモデルだけで `motion` / `expression` 属性を試せる)。README を直した。
- **ビューアでの再生**: ビューアにはモーションと表情を再生する画面がまだ無い (T0701 の後続のまま)。team-lead が T0720 に分けたので、
  受け入れ条件の 1 つ目のうち「ビューアのサンプルで再生できる」は満たしていない (チェックを付けていない)。
  エディタの File > Open Sample… (`apps/editor/src-tauri/src/sample.rs`) は本体のプロジェクトに物理だけを写すので、見本の
  モーションと表情はエディタの見本のプロジェクトに入らない (範囲外。`base.motions = bundle.motions` と表情の 2 行で直る。後続の候補)。
- **serde_json の見直し**: **変えなかった**。twiggy (名前を残した `web-dist` のビルド) で `serde_json` の関数は約 414KB で、ほとんどが
  モデルの型の `Deserialize` の JSON 用の展開。約半分 (約 200KB) はモデルの JSON の読み込み (`load_json`。npm の `loadModel` に文字列、
  エディタのビューポート) で、残りはモーション・表情・物理・エフェクト・ライティングの JSON (どれも公開の API)。上限を知るため
  モデルの JSON の読み込みを外したビルドを一時的に測ると、パッケージの wasm は 4,475,706 → 4,289,977 (−4.2%)、gzip 1,311,151 → 1,270,981
  (−3.1%)、brotli 969,758 → 946,831 (−2.4%)。API を保ったまま減らす方法 (別の形に写してから読む・自前の読み込み) はこれより減らず、
  エラーの文などの結果も変わるので、T0413 と同じ基準で入れなかった。数字は `bindings/web/README.md#wasm-の大きさ` に書いた。
  `crates/ugoe-wasm` は変えていない。
- **平滑化の係数**: 実際の顔では調整していない (利用者の確認に残す)。調整できるように、見本 (`examples/basic`) に
  「camera (MediaPipe)」(Face Landmarker を CDN から読む。パッケージは MediaPipe に依存しない) と、係数の欄 (URL でも指定できる)、
  揺れの目安 (頭のヨーの 2 階差分の平均) を足した。手順と見るべき点 (止まった顔の揺れ・速い動きの遅れ・まばたきが閉じきるか・
  明るさとフレームレート) は `bindings/web/README.md#平滑化の係数を実際の顔で調整する`。
- 確かめたこと: `just ci`・`just wasm-check`・`just test-gpu`・`just conformance --only c` (53 件)・`just conformance --only wasm` (53 件)。
  `ugoe render --expression/--motion` で見本の表情とモーションを描いた (scratchpad の `t0719/avatar-expr.png`・`dress-expr.png`・
  `avatar-motions.png`)。`pnpm pack` した tarball を見本に入れ、ヘッドレス Chrome (SwiftShader) の WebGL2 と WebGPU で、
  画面の選択欄からモーション (`nod`・`tilt`) と表情 (4 つ) が動くこと、`?src=samples/dress-up/dress-up.ugoe` でも動くことを確かめた
  (`t0719/shots/`、`t0719/browser.png`)。カメラは Chrome の偽のカメラに見本のアバターの絵 (`AngleX` を振った 24 枚) を流し、
  MediaPipe が顔を見つけて `<ugoe-avatar>` に渡ること、揺れの目安が出ることを確かめた (`t0719/shots/webgl2-camera.png`。
  入力 0.40 → 平滑化 0.07)。確認に使ったサーバーは止めた。
- **確かめていないこと**: 本物のカメラと人の顔での平滑化 (上のとおり)。本物の GPU、Firefox・Safari。ビューアでの再生 (T0720)。
- **範囲外の変更**: なし。
- **後続の候補**: エディタの File > Open Sample… で見本のモーションと表情もプロジェクトに入れる (`apps/editor/src-tauri/src/sample.rs`)。
  平滑化の既定の係数を実際の顔で決めたら `tracking-filter.ts` の既定を直す。
