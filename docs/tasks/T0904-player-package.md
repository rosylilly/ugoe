---
id: T0904
title: 共有: ビューアのモデル再生の層を @ugoe/player に切り出す
status: done
milestone: M8
crates: [packages, apps/viewer, docs, justfile, .github/workflows, lefthook.yml]
depends_on: [T0900]
assignee: claude-worker-T0904
---

# T0904: 共有: ビューアのモデル再生の層を @ugoe/player に切り出す

## 目的

スタジオはビューアと同じ再生の層 (モデルの読み込み・モーション・表情・トラッキング・ライティング・着せ替え・覚えた設定) を使う。
コピーせずに共有するため、ビューアから切り出す。

## 受け入れ条件

- [x] apps/viewer/src/lib の player.svelte.ts・animation・lighting・dressUp・drivers・modelMemory などを packages/player に移し、ビューアはそれを使う
- [x] 画面に依存する部分 (Svelte の部品) とそうでない部分を分ける。wasm の読み込みはアプリが渡す形にする
- [x] 振る舞いを変えない (ビューアの Vitest がそのまま通り、ヘッドレスの Chrome で見た目が同じ)
- [x] `just ci`・ビューアのビルドが通る

## 参考

- apps/viewer/README.md

## メモ

- **範囲の追加 (team-lead の承認済み):** justfile の web-check・ci.yml のフロントエンドのジョブ・lefthook.yml の pre-push に
  `@ugoe/player` の check と test を face-tracking の隣に足した。ビューアから移したテスト (103 件) が CI で走るようにするため。
- **分け方:** 画面に依存しない TS (`player.svelte.ts`・`animation`・`lighting`・`color`・`dressUp`・`dressUpMemory`・`drivers`・
  `loadWarnings`・`modelFile`・`modelMemory`・`parameters`・`rememberedModels` と各テスト) を `packages/player/src/` に `git mv` し、
  `src/index.ts` からまとめて出す (face-tracking と同じく、サブパスの import はしない)。Svelte のパネル (パラメータ・ライティング・
  着せ替え・モーション・覚えたもの・ビューポート・光源の印) はビューアに残した。ADR 0029 の「パネルを部品に移した後は @ugoe/ui も」
  のとおり、`@ugoe/ui` ができてから部品に作り直して移す (後続)。ビューアに残したのは `host.ts`・`samples.ts`・`drop.ts`・
  `screenshot.ts` (PNG とファイル名。`Screenshot` の型は player から)・`wasm/`・`App.svelte`・`global.d.ts` (`__UGOE_VIEWER__`)。
- **wasm はアプリが渡す:** `new Player({ loadWasm, readFile, wasmMissingMessage })` (`PlayerHost`)。player の `src/wasm.ts` は
  Player が使う `WebPuppet` のメソッドだけを構造の型で持ち (`TrackingPuppet` を継ぐ)、ビューアの生成物がそれを満たすかは
  ビューアの型検査で確かめる。ビューアの `wasm/load.ts`・`stub.d.ts` と生成物の扱いは変えていない (T0133 との衝突を避けた)。
  なお `stub.d.ts` には `set_motion_over_tracking`・`set_tracking_kept`・`active_expressions` が無く、生成物が無いときのビューアの
  svelte-check は前から失敗する (前は 3 件、いまは `App.svelte` の 1 件にまとまる)。stub に 3 行足せば直る (T0133 か後続で)。
- **受け手のモードの下ごしらえ:** `#tick` (時計・失った GPU の回復・描画) から 1 フレーム分を進める `#advance(puppet, dt)` を
  分けた (中身は同じ順)。T0801 は `#tick` で届いたフレームを順に取り出して値を書き `update(dt)` する分岐を足せばよい
  (packages/player/README.md)。
- **後続の候補:** パネル (パラメータ・ライティング・着せ替え・モーション・覚えたもの・ビューポート・光源の印) を `@ugoe/ui` の部品で
  作り直して packages/player に移す。文言を `@ugoe/i18n` に移す。範囲外のコメントのパス (下) を直す。
- **localStorage のキー** はビューアの名前 (`ugoe-viewer.*`) のまま。スタジオで使うときに接頭辞をアプリが渡せるようにする (T0801 か後続)。
- **確かめたこと:** Vitest は player 103 件 + ビューア 22 件 = 前のビューアの 125 件と同じ。`just ci`・`pnpm --filter @ugoe/viewer build`。
  ヘッドレス Chrome (SwiftShader の WebGL2、ビルドした前後の `dist` を `vite preview`、`requestAnimationFrame` の時刻を 0 に固定して
  dt = 0 にし、呼吸・物理・モーションを止める) で 7 場面 (空・アバター・AngleX 20・ライティング・着せ替え・Hair を外す・覚えたもの) を
  撮り、前後を ImageMagick で比べた: 6 場面は画素が一致、覚えたものの一覧は 62 画素 (前どうしを 2 回撮っても同じ 62 画素。
  覚えた時刻の表示)。時間を止めない撮り方では前どうしでも呼吸と揺れで違うので使わない。スクリプトとスクリーンショットは
  scratchpad の `T0904/` (`shots.mjs`・`run.sh`・`shots/before-*.png`・`after-*.png`・`diff-*.png`)。
- **確かめていないこと:** Tauri 版 (WebKitGTK)、WebGPU、本物のカメラ。
- **範囲外で残したもの:** `crates/ugoe-wasm/README.md` の 225 行と `apps/editor/src/lib/preview/preview.ts` の 2 行目のコメントが、
  移す前のパス (`apps/viewer/src/lib/loadWarnings.ts`・`parameters.ts`) を指している (コメントだけ。後続で packages/player に直す)。

