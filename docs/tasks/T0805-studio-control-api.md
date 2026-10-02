---
id: T0805
title: スタジオ: 外から操作する API (ループバックの WebSocket・トークン・利用者の許可)
status: done
milestone: M8
crates: [apps/studio, docs, Cargo.toml, Cargo.lock]
depends_on: [T0803]
assignee: claude-worker-T0805
---

# T0805: スタジオ: 外から操作する API (ループバックの WebSocket・トークン・利用者の許可)

## 目的

Stream Deck やチャットのボットなど、外のツールから表情やシーンを切り替えられるようにする。

## 受け入れ条件

- [x] ループバックだけで待つ WebSocket の API。初回の接続は利用者の許可 (操作ウィンドウで承認) とトークン。既定は切
- [x] 操作: 表情・モーション・シーンの一覧と切り替え、パラメータの値の設定、着せ替え。イベントの通知 (シーンの変化など)
- [x] メッセージの JSON Schema と docs (英語の API リファレンス)、接続と操作のテスト
- [x] `just ci` が通る

## 参考

- ADR 0028 (安全性)、crates/ugoe-mcp の --http の扱い

## メモ

### やったこと (2026-10-02)

- 「スタジオの操作 API」(ugoe Studio API) を `apps/studio/src-tauri/src/api/` に作った。ループバック (`127.0.0.1` と `::1`) の
  WebSocket (`ws://127.0.0.1:<port>/api`) の上の JSON-RPC 2.0。既定は切で、操作ウィンドウの「操作 API」のパネルで有効にする (選んだことを覚える)。
  既定のポートは 28451 (他製品の既定と重ならず、Linux と Windows の一時的なポートの範囲の外)。
  - 初めてのクライアントは `session.hello` でトークンと名前を示し、操作ウィンドウの上の知らせ (`ApiApproval.svelte`。権限の選択・承認・拒否・後で) で
    利用者が承認するまで応答しない。承認したらクライアント用の鍵を返し、以後は鍵で接続する。違うトークン・知らない鍵・拒否は誤りを返して閉じる。
  - トークンと鍵 (OS の乱数の 256 ビット、定数時間で比べる) はアプリのデータの場所の `studio-api.json` (Unix は 0600。プロファイルには入れない)。
    パネルでトークンを写す・作り直す、承認したアプリを切断する・取り消す、「後で」で隠した承認を出し直す。待ち受けている間はツールバーに「API」の印。
  - 経路に依らない検査 (`http.rs` の `check_request`): `Host` がループバックの名前でない・転送ヘッダが付いた・スタジオ自身のページでない
    `Origin` の要求は `403`。T0808 のブラウザソースは `server.rs` の `route` に経路を足して同じ待ち受けを使える (HTTP と WebSocket を同じ口で扱う)。
  - メソッド: `studio.state`、`scene.list`/`activate`、`expression.list`/`set`/`add`/`remove`/`toggle`/`clear`、`motion.list`/`play`/`stop`、
    `parameter.list`/`set` (重みと期限。既定 1 秒、最大 10 秒)、`dressup.list`/`set` (プロファイルのシーンとホットキーの着せ替えにあるモジュールだけ)、
    ホットキーと同じ `tracking.pause`/`lipsync.pause`/`tracking.calibrate`、`events.subscribe`/`unsubscribe`。権限は読むだけ / 操作する。
  - 通知: `scene.changed`・`scene.listChanged`・`expression.changed`・`motion.started`/`finished`/`event`・`dressup.changed`・`tracking.changed`。
  - パラメータの上書きは指揮役 (`conductor.rs` の `override_parameter`) で、物理の段に入る直前の値 (受け手に配る値) に刻みごとに当て、`dt` で期限を減らす
    (指揮役の `Puppet` には書かないので、切れたらすぐ元の値に戻る)。モーションのイベントも指揮役が集める (`take_motion_events`)。
  - WebSocket は **tungstenite 0.30 (同期版) に任せる** (利用者の決定、2026-10-02)。HTTP の要求の読み取り・Host/Origin/転送ヘッダの検査・経路は自前のまま
    (T0808 と口を共有)、`/api` の Upgrade のときだけ `derive_accept_key` (握手の応答の鍵) と `WebSocket::from_partially_read`
    (`from_raw_socket` の仲間。握手の後に読み込んだバイトも渡す) に任せる。設定は `max_message_size` 256 KiB・`max_frame_size` 64 KiB。
    `WebSocket` は読みと書きを分けられないので、接続ごとに 1 つのスレッドで、読むのを 20 ms で区切って合間に列のもの (応答・通知・閉じる) を送る。
    pong とクライアントの close への応答は tungstenite が返し、決まりに反するフレーム・大きすぎるもの・UTF-8 でないテキストは 1002・1009・1007 で閉じる。
    - 依存: ルートの `[workspace.dependencies]` に `tungstenite = { version = "0.30", default-features = false, features = ["handshake"] }`
      (`handshake` は `derive_accept_key` に要る)。依存するのは apps/studio (`ugoe-studio`) だけ。`getrandom` はワークスペースにあるものを足した。
    - Cargo.lock に増えたクレート: tungstenite 0.30.0・sha1 0.11.0・digest 0.11.3・block-buffer 0.12.1・crypto-common 0.2.2・hybrid-array 0.4.15・
      const-oid 0.10.2・cpufeatures 0.3.1・data-encoding 2.11.1・rand 0.10.3・rand_core 0.10.1・chacha20 0.10.2
      (bytes・http・httparse・log・thiserror は既にあった)。ライセンスはどれも MIT か Apache-2.0 との二重 (`just licenses` で確かめた)。
- 文書: 英語の API リファレンス `docs/studio-api.md`、JSON Schema `docs/studio-api.schema.json`、`docs/studio.md#外部-api`、`apps/studio/README.md`、
  ADR 0028 の 6. のリンク、`docs/README.md`。
- テスト (`api/tests.rs` ほか): 承認・権限・拒否・切断・取り消し・トークンの作り直し、シーン・表情・モーション・パラメータの上書き (クランプ・重み・期限切れ)・
  着せ替え (プロファイルに無いパスを断る)・通知、本物の待ち受け (Host・Origin・転送ヘッダの `403`、握手、ping、バイナリで閉じる、止めると閉じる)、
  設定の保存と読み直し。受け取った応答と通知は、すべて JSON Schema と比べる (Schema の小さな検査をテストに置いた)。WebSocket の握手は RFC の例、フレームは
  規則に反するフレーム (大きすぎる・マスクの無い・UTF-8 でない) と分割したメッセージを本物の待ち受けで確かめる。TS は `src/lib/api.test.ts`。

### 確かめたこと

- `just ci` (tungstenite に替えて main (T0811 まで) に rebase した後)、`just licenses`、`cargo test -p ugoe-studio`。`just test-gpu` は自前の WebSocket のときに通した
  (その後の変更は api/ の中だけ)。
- ヘッドレスの Chrome と偽の StudioHost (API の様子は固定の値) で操作ウィンドウを撮った: 承認の知らせ・パネル (英語・日本語・暗い色・切・ポートを使えない)、
  承認のボタンが選んだ権限で `approveApiClient` を呼ぶこと (scratchpad/T0805/shots)。

### 確かめていないこと

- 本物のアプリ (Tauri) での操作と、実際の Stream Deck などのクライアントからの接続 (テストのクライアントは自前の最小のもの)。
- Windows と macOS での待ち受け (`::1` に束ねられないときは `127.0.0.1` だけで続ける)・ファイアウォールの知らせ。

### 後続の候補

- Stream Deck のプラグインなど、よく使うクライアントの見本 (この API の上で)。
- 承認の知らせを OS の通知にも出す (操作ウィンドウが隠れているとき)。
- T0808 (ブラウザソース): `server.rs` の `route` に `/source/<トークン>/…` を足す。有効にするスイッチとトークンは別に持つ (ADR 0028 の 6.)。

- 範囲外の変更 (team-lead が承認): ルートの Cargo.toml (workspace.dependencies に tungstenite) と Cargo.lock。利用者の判断 (2026-10-02) で WebSocket は tungstenite の同期版。
