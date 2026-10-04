---
id: T0724
title: コードレビュー: ビューア
status: done
milestone: M9
crates: [apps/viewer, docs]
depends_on: []
assignee: claude-worker-T0724
---

# T0724: コードレビュー: ビューア

## 目的

ビューア のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `apps/viewer` のすべて (src と src-tauri)

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - ファイルを開く・ドロップの経路のパスの扱い (T0713)、wasm と WebGL の資源の解放
  - `@ugoe/player`・`@ugoe/face-tracking` と重複した処理
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0726〜T0730 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

範囲: `apps/viewer` の src (Svelte・TS とテスト)・src-tauri (Rust とテスト・tauri.conf・capabilities・build.rs)・設定 (vite・tsconfig・package.json・index.html)・
`locales/`・README。ドロップと「開く」の経路 (T0713) は、OS からのドロップを絶対パスで受け、`read_model_file` が拡張子と大きさだけを見て読む
(パスのたどりは利用者が選んだもの・落としたもので、ほかの経路から来るのは覚えた `path:` の場所だけ)。wasm と WebGL/WebGPU の資源は
`Viewport.svelte` の `onMount` の後片付けで `Player.detach` が解放する (ビューアでは App が外れないので、実際に外すのは開発の HMR だけ)。
`@ugoe/player`・`@ugoe/face-tracking` との重複は、`host.ts` の `downloadJson` (face-tracking にもある) と、ブラウザの wasm の型 (`wasm/stub.d.ts`。
生成物が無いときの型で、`web.rs` の公開の口と食い違いが無いことを確かめた) だけ。

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 中 | `src/lib/host.ts` (`tauriHost.saveJsonFile`) | 保存のダイアログが拡張子の無いパスを返すと (Linux のダイアログ)、バックエンドの `write_json_file` が `.json` でないとして拒み、トラッキングの記録を保存できない。PNG は `.png` を付け足していたが JSON は付けていなかった | 直した (`withExtension` を JSON と PNG で共有。`host.test.ts` を足した) |
| 中 | `src/lib/screenshot.ts` | 画面 (`screenshot.error`) に出る例外の文が日本語の直書きで、英語の画面でも日本語になる。`just i18n` の直書きの検査は例外の文を対象の外にしているので見つからない | 直した (カタログ `screenshot.invalidSize` などで訳す。テストを足した) |
| 中 | `src-tauri/src/lib.rs` (`read_model_file`・`write_*`) | 同期のコマンドなのでメインスレッドで動き、最大 512 MiB のファイルを読む間ウィンドウが止まる | T0726 を起票 |
| 中 | `src-tauri/tauri.conf.json`・`capabilities/default.json`・`lib.rs` | CSP が `null`。アプリのコマンドを権限で絞っていない。`read_model_file`・`write_json_file` は拡張子さえ合えばどのパスでも読み書きする | T0727 を起票 |
| 中 | `src-tauri/src/lib.rs` (`write_json_file`・`write_png`) | 一時ファイルを経ずに `std::fs::write` で直接上書きするので、途中で失敗すると利用者が選んだ既存のファイルが壊れる | T0728 を起票 |
| 中 | `src-tauri/src/lib.rs` (すべてのコマンド) | エラーを英語の文で返し、画面はそのまま出す (ドロップの知らせの理由・保存の失敗)。ADR 0029 の `code`・`args` の形でない | T0729 を起票 |
| 低 | `src/lib/screenshot.ts`・`src-tauri/src/lib.rs` (`write_png`・`percent_decode`) | エディタの `viewport/screenshot.ts`・`src-tauri/src/screenshot.rs` とほぼ同じものが 2 つある | T0730 を起票 |
| 低 | `src-tauri/src/lib.rs` (`initial_file` の doc コメント) | 「OS の『このアプリで開く』」と書くが、`tauri.conf.json` に `fileAssociations` が無く、macOS ではファイルを引数ではなく `RunEvent::Opened` で渡すので届かない | 起票の候補 (下。枠が足りない) |
| 低 | `src/lib/host.ts` (`downloadJson`) | `@ugoe/face-tracking` の `downloadJson` と重複 (ビューアは常に `saveJson` を渡すので、face-tracking の既定は使われない) | 見送り (片方を公開して寄せるだけの小さな話。packages を触る T0730 などのついでに) |
| 低 | `src/App.svelte` (`onKeyDown`) | ダイアログを開いている間も `F` (全体を表示) と `Ctrl+O` が効く。`contenteditable` を除いていない | 見送り (害が小さい。キーボード操作の見直し (M10) の候補) |
| 低 | `src/lib/LightMarkers.svelte` (`wheelTimer`) | タイマーが済んだ後も `wheelTimer` を戻さないので、印が消えるときに `commitLighting` をもう一度呼ぶ | 見送り (`commitLighting` は保留中の変更が無ければ何もしないので、動きは変わらない) |
| 低 | `src/lib/drop.ts` (`classifyDragType`) | 引数を小文字にした局所の変数 `t` が、文言の `t()` を隠す | 見送り (いまは中で `t()` を呼んでいない。名前だけの話) |
| 低 | `src/lib/LicenseDialog.svelte` | 読み込みに失敗すると失敗の文が `text` に入り、次に開いても読み直さない | 見送り (同梱のファイルなので失敗は稀) |
| 低 | `src-tauri/src/lib.rs` (`read_model_file`) | 大きさを `metadata` で見てから `fs::read` で読むので、その間に大きくなったファイルは上限を超えて読む | 見送り (T0726 で読み方を変えるときに、上限付きの読み込み (`take`) にするとよい) |

#### 起票の候補 (枠が足りないもの)

- ビューアを OS の「このアプリで開く」に対応させる: `tauri.conf.json` の `bundle.fileAssociations` (`.ugoe`・`.ugoemod`) と、macOS の
  `RunEvent::Opened` の受け取り (起動中に開いたときの扱いを含む)。それまでは `initial_file` の doc コメントを「起動時の引数」に限って書き直す。

### 横断 (T0008 向け)

- **保存のダイアログの拡張子:** Linux のダイアログはフィルタの拡張子を付けずにパスを返すことがある。エディタは `withExtension` で付けているが、
  ビューアの JSON は付けていなかった (直した)。スタジオの `host.ts` の `pickSavePath` (プロファイル) も付けていないので、`save_profile` が拡張子を
  見るなら同じ失敗になる。保存のダイアログを使うところすべてで確かめる。
- **例外の文の直書き:** `scripts/check-hardcoded-strings.mjs` は例外 (`throw`・`new Error`) の文を対象の外にしているが、画面は `e.message` を
  そのまま出すところが多い (ビューアの画像の保存がそうだった)。画面に出る例外は `t()` で作るか、検査で「`e.message` を出すところに届く例外」を
  見る方法を考える。
- **Tauri のコマンドのエラーの形:** ビューアのコマンドは ADR 0029 のコードの形でなく英語の文を返す (T0729)。ほかのアプリのコマンドも確かめる。
- **同期のコマンド・CSP・直接の上書き:** エディタ (T0685・T0686)・スタジオ (T0819・T0820)・ビューア (T0726〜T0728) の 3 つのアプリで同じ形。
  直し方 (非同期にする関数・一時ファイルの書き方・CSP の中身) を 1 つに揃えるとよい。
- **`Player.attach` と `detach` の競り合い (`packages/player`):** `attach` は wasm を待ってから WebPuppet を作り `requestAnimationFrame` を始めるので、
  待っている間に `detach` されると、外した後で WebPuppet と描画のループが動き出す。ビューアでは App が外れないので起きないが、
  `@ugoe/player` を使うほかの画面 (部品を出し入れするところ) では起こりうる。`@ugoe/player` のレビューで確かめる。
