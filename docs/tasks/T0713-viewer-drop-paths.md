---
id: T0713
title: ビューア: Tauri でドロップしたファイルのパスを受け取り、起動し直した後も戻せるようにする
status: done
milestone: M4
crates: [apps/viewer, docs]
depends_on: [T0711]
assignee: claude-worker-T0713
---

# T0713: ビューア: ドロップしたファイルのパスを受け取る

## 目的

T0711 の後続の候補。ドロップしたモデルやモジュールは場所が分からないので、付けていた組を戻せるのはアプリを開いている間だけになっている。
Tauri のウィンドウではドロップしたファイルのパスを受け取れるので、`path:` の場所として覚え、起動し直した後も戻せるようにする。

## 受け入れ条件

- [x] Tauri 版では、ドロップしたファイルのパスを受け取り、`ModelFile.location` (`path:`) に入れる。ブラウザ版は今のまま
- [x] モデルとモジュールのどちらのドロップでも、覚えた組を起動し直した後に戻せる
- [x] Vitest。Tauri のウィンドウでの確認は利用者の確認に残してよい (その旨を書く)
- [x] `just ci`・ビューアのビルドが通る

## 参考

- `docs/tasks/T0711-viewer-dress-up-per-model.md` のメモ、`apps/viewer/README.md#モデルごとに覚える-t0711`

## メモ

### やったこと

- **Tauri のドロップ:** `tauri.conf.json` の `dragDropEnabled: false` をやめ (既定の有効に戻す)、OS からのドロップを Tauri に
  受けさせる。`host.ts` の `ViewerHost` に `listenFileDrop(listener)` を足した (Tauri 版は `getCurrentWebview().onDragDropEvent`。
  `enter`/`leave` → `{type: "hover", active}`、`drop` → `{type: "drop", paths}`。`over` は使わない)。`App.svelte` はホストに
  `listenFileDrop` があれば購読し、落としたパスを `readFileAt("path:…")` (= `read_model_file`) で読むので、`ModelFile.location` に
  `path:` が付く。このとき HTML のドロップ (`dragover`/`drop`) は扱わない (二重に開かないため)。ブラウザ版は今のまま (File API、場所なし)。
  ビューアのページには HTML5 の DnD (draggable) が無いので、Windows で HTML5 の DnD が止まる影響は無い (光源の取っ手・パンは pointer イベント)。
- 開き方 (モデルとモジュールを一緒に落としたとき、モデルは最後の 1 つ、モジュールだけなら 1 つずつ) を `lib/drop.ts` の
  `openDroppedFiles` に移した (振る舞いは T0711 と同じ。HTML とパスの両方の経路で使う)。
- Rust のバックエンドは変えていない (`read_model_file` は `.ugoe`・`.json`・`.ugoemod` だけを読むので、ほかの拡張子を落とすと
  そのエラーがステータスに出る)。IPC も増やしていない。capabilities も `core:default` (イベントの listen を含む) のまま。
- 文書: `apps/viewer/README.md` (構成に `drop.ts`、「ブラウザ版と Tauri 版の違い」・モデルの鍵・ヘッドレスでの確かめ方)、
  `docs/manual/dress-up.md` (アプリではドロップしたファイルも起動し直した後に戻せる。ブラウザでは今までどおり)。
- Vitest: `drop.test.ts` (3 件)。Tauri の `invoke`・`onDragDropEvent` を模し、パスのドロップで `path:` の場所が付くこと、
  モジュールだけのドロップ、覚えた組と光源が「起動し直した後」(新しい入れ物・手元の中身なし・モデルの中身が変わっても場所で同じ) に
  場所から戻せること。

### 確かめたこと

- `just ci`・`vite build`・`cargo check -p ugoe-viewer`。
- ヘッドレスの Chrome (ビルドした `dist/` を `vite preview`) で、`__UGOE_VIEWER_HOST__` に `readPath` と `listenFileDrop` を持つ
  Tauri を模したホストを置いて: HTML のドロップは無視される → パスで dress-up.ugoe + twin-tails + sailor をドロップ (付く。
  localStorage の鍵とモジュールがすべて `path:`) → avatar-lit.ugoe をドロップして光源を変える (`path:` の鍵で覚える) →
  ページを読み直す (起動し直しの代わり) → dress-up.ugoe をドロップすると尋ねる → 「戻す」で 2 つのモジュールをパスから読み直して
  付く → avatar-lit.ugoe をドロップすると覚えた光源 (`lightingRemembered`) → モジュールだけのドロップ。
  スクリプトとスクリーンショットは scratchpad の `T0713/` (`check.mjs`、`shots/01-dropped.png`・`02-offer-after-restart.png`・
  `03-restored.png`)。

### 確かめていないこと (利用者の確認に残す)

- **Tauri (WebKitGTK・WebView2・WKWebView) の本物のウィンドウでのドロップ** は動かしていない。確かめること: `just viewer` で
  モデルとモジュールをエクスプローラなどから落として開けること、落とすときの枠 (`dropping`) が出て消えること、閉じて起動し直した後に
  同じモデルを落とすと覚えた組を「戻す」で戻せること、光源も戻ること。`.deb` などのビルドもしていない。

### 後続の候補

- ドロップしたものがフォルダや読めない拡張子のとき、今は `read_model_file` のエラー (英語) がそのまま出る。読める拡張子だけを
  開いて、残りは「開けないファイルを飛ばしました」と知らせる方が親切。
- Tauri の `enter` の `paths` で、落とす前に読めるファイルかどうかを枠の色などで示せる (エディタの DressUpSection のように)。
