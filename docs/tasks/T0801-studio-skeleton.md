---
id: T0801
title: スタジオ: アプリの骨組み (操作ウィンドウと出力ウィンドウ)
status: done
milestone: M8
crates: [apps/studio, packages, AGENTS.md, scripts, justfile, commitlint.config.mjs, .github/workflows, docs, Cargo.toml, Cargo.lock, pnpm-lock.yaml, lefthook.yml]
depends_on: [T0800, T0901, T0902, T0904, T0133]
assignee: claude-worker-T0801
---

# T0801: スタジオ: アプリの骨組み (操作ウィンドウと出力ウィンドウ)

## 目的

apps/studio を作り、モデルを出力ウィンドウに出して操作ウィンドウから動かせるところまで作る。

## 受け入れ条件

- [x] Tauri v2 + Svelte 5 のアプリ (GPL-3.0-only)。@ugoe/player・@ugoe/ui・@ugoe/i18n を使う。OS の言語は tauri-plugin-os の locale() で取る
- [x] 操作ウィンドウと出力ウィンドウ。出力ウィンドウは枠なし・背景の透過・最前面 (切り替え)・大きさの指定 (1920x1080 などのプリセット)
- [x] ADR 0028 の指揮役: バックエンドに物理を切った Puppet を 1 つ置き、刻みごとに {seq, dt, values} を配る。出力ウィンドウは受け手として全フレームを順に適用し、自分で物理を回して描く
- [x] ADR 0028 の 10. の一致の検査 (記録したフレームの列を受け手に適用し、物理を有効にした 1 つの Puppet と頂点・不透明度を比べる)
- [x] @ugoe/player に受け手のモード (値と dt を受けて進める) を足す
- [x] `just studio`、CI のフロントエンドの検査、ライセンスの検査、commitlint の scope `studio`、AGENTS.md の表とライセンスの行に apps/studio
- [x] `just ci` とスタジオのビルドが通る

## 参考

- ADR 0028、docs/studio.md

## メモ

### やったこと (2026-10-02)

- `apps/studio` (GPL-3.0-only、Rust のクレート `ugoe-studio`、npm の `@ugoe/studio`) を Tauri v2 + Svelte 5 で作った。構成は `apps/studio/README.md`。
  - 操作ウィンドウ (`index.html`・`Control.svelte`): モデルを開く・出力ウィンドウの表示/最前面/大きさのプリセット・いまの状態 (モデル・フレーム・
    1 刻みの時間・受け手の数)・パラメータのスライダー・言語 (OS に合わせる / 日本語 / English)。モデルは描かない。部品は @ugoe/ui、文言は @ugoe/i18n
    (名前空間 `studio`、`locales/`)、密度は comfortable、強調色はトークンの紫。
  - 出力ウィンドウ (`output.html`・`Output.svelte`): 枠なし・背景の透過・既定は隠す・閉じる操作で隠す・Alt + ドラッグで動かす。
    @ugoe/player の受け手のモードでモデルだけを描く。
  - OS の言語は tauri-plugin-os の `locale()` を `setSystemLanguages` に渡す。
- 指揮役 (`src-tauri/src/conductor.rs`): 物理を切ったネイティブの Puppet。刻みのスレッドが 60 回/秒で `dt` を測り (0.1 秒で頭打ち)、
  フレームを Tauri の `Channel` で配る。T0134 に合わせ、`values` は `values_before_physics`、リップシンクの入力も載せる。
  フレームの形 (バイト列) と `scene` の JSON は docs/studio.md に書いた。T0800 の `reset` メッセージはフレームのフラグにした (理由は docs/studio.md)。
- @ugoe/player に受け手のモード (`startFollowing`・`followScene`・`pushFrame`、`src/follower.ts` の `decodeFrame`・`FrameQueue`・`applyFrame`)。
  `follow` を呼ぶのは TS は `applyFrame`、Rust は `follower.rs` の `apply_frame` の 1 か所ずつ。
- 一致の検査 (`src-tauri/tests/follow_match.rs`): 物理・動的物理・ポーズのフィクスチャで、書き換え・揃っていない dt・リップシンク (物理の入力に当てる)・
  物理のリセット・モデルの読み直しを含む列を指揮役で記録し、バイト列から受け手に適用して、物理を有効にした 1 つの Puppet と値・頂点・不透明度を比べる。
  まとめて適用しても同じになることも確かめる。指揮役が T0133 の形 (リップシンクの後の値) を送ると落ちることを確かめた。
- Rust と TS のフレームの読み書きは、同じバイト列 (GOLDEN) をそれぞれのテストで読む。
- 登録: `just studio`・`just studio-build`・`just web-check`、ci.yml の web ジョブ (test・wasm・check・build)、editor-build.yml の行列、
  ci-groups の apps、check-layers (`ugoe-studio` はランタイム層 + ugoe-headless)、check-licenses の EDITOR_CRATES、lefthook の pre-push、
  AGENTS.md の構成・コマンド・ライセンスの行、docs/testing.md。commitlint の `studio` は追加済みだった。

### 確かめたこと

- `just ci`、`cargo test -p ugoe-studio`、`just studio-build --bundles deb` (Linux、rebase 前)。
- ヘッドレスの Chrome と使い捨てのブリッジ (本物の指揮役を HTTP で出す。scratchpad/T0801/bridge) で、操作ウィンドウと出力ウィンドウを別のページで開き、
  モデルを開く → 出力ウィンドウが受け手として描く → 操作からパラメータを動かすと出力が動く、を確かめた (SwiftShader の WebGL2)。
  英語・日本語 (`locale()` が ja-JP)・ダーク・狭い幅も撮った。スクリーンショットは scratchpad/T0801/shots/。

### 確かめていないこと (利用者の確認に残す)

- 実際の Tauri のウィンドウ: 出力ウィンドウの背景の透過・枠なし・最前面の切り替え・物理ピクセルでの大きさ・Alt + ドラッグ・閉じる操作で隠れること、
  操作ウィンドウを閉じるとアプリが終わること。Windows・macOS は未確認。
- 透過のために macOS では `macOSPrivateApi` (tauri の `macos-private-api` feature) を入れた。Mac App Store では配布できなくなる (team-lead に相談済み)。

### 範囲外の変更

- ルートの `Cargo.toml` (members と `tauri-plugin-os`)・`Cargo.lock`・`pnpm-lock.yaml` (`@tauri-apps/plugin-os` ほか)・`lefthook.yml` (pre-push の check)。
  どれもアプリを足すのに避けられないもので、team-lead に知らせた。ルートの `.gitignore` は変えず、`apps/studio/.gitignore` を置いた。

### 後続の候補

- wasm の受け手 (`@ugoe/player`) とネイティブの受け手の頂点の比べ合わせ (ADR 0028 の 10. の (2)。T0808 のブラウザソースと一緒に)。
- 刻みの速さの設定 (30 / 60 / 120) と負荷の表示の詳細、WebView の抑制を切る (T0804)。Windows の `thread::sleep` の粒度で 60 刻みが揺れるかも T0804 で測る。
- Rust のエラー (`open_model` など) をコードと引数の形にする (ADR 0029 の 4.)。いまは英語の文を「開けません」の文に添えている。
- 出力の大きさと言語の設定をプロファイル (T0802) へ移す。
- 範囲外の変更 (team-lead が承認、2026-10-02): ルートの `Cargo.toml` (members に apps/studio/src-tauri、[workspace.dependencies] に `tauri-plugin-os = "2"`)・`Cargo.lock`・`pnpm-lock.yaml` (`@ugoe/studio` と `@tauri-apps/plugin-os`)・`lefthook.yml` (pre-push の check に `@ugoe/studio`)。新しいアプリをワークスペースと検査に入れるのに避けられないため。
- macOS の透過は `macOSPrivateApi` を使う (利用者の判断。docs/studio.md)。
- team-lead が review の後に、T0908 の直書き検査が読めない総称の矢印関数を host.ts で関数宣言に書き換え、検査がファイル名を出して失敗するようにした。
