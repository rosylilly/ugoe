---
id: T0516
title: MCP: 文書の持ち主の抽象化と、エディタへの中継 (--editor)
status: done
milestone: M6
crates: [ugoe-mcp, docs]
depends_on: [T0515]
assignee: claude-worker-T0516
---

# T0516: MCP: 文書の持ち主の抽象化と、エディタへの中継 (--editor)

## 目的

ADR 0023 の 1・3・4 の `ugoe-mcp` 側。ツールをエディタの中でも使えるようにし、`ugoe-mcp --editor` で stdio のクライアントを実行中のエディタへ中継する。

## 受け入れ条件

- [x] ツールの文書の持ち主を抽象化する (例: `DocumentHost` トレイト。ファイルを自分で開く持ち主 = 今の振る舞い、外から借りる持ち主)。適用 (`apply_proposal`・`undo`・`redo`・`save_project`)・プロジェクトの開閉は持ち主が許すかを決められる (エディタでは確認を待つ・拒否する)。ツールの形と結果は変えない。既存のテストが通る
- [x] エディタが組み込める API: 外から渡した持ち主でツールを動かし、HTTP (T0514) で待ち受ける関数 (ループバック・トークン必須・`--root` 相当の制限の扱いを書く)。発見用のファイル (ポート・トークン・PID・版) の形と置き場所 (OS ごと) を決め、読み書きの関数を置く
- [x] `ugoe-mcp --editor`: 発見用のファイルを読んで実行中のエディタにつなぎ、stdio のメッセージを中継する。エディタが無い・止まった・版が合わないときの分かりやすいエラー
- [x] WSL: Linux 版の `--editor` は Windows 側の発見用のファイルを探せる (`/mnt/c` の下。Windows の利用者のディレクトリの見つけ方を書く)。ミラーでないネットワークでつながらないときの案内。WSL のパスと Windows のパスの相互の変換 (中継の中で。`/mnt/c/...` ↔ `C:\...`、`/home/...` ↔ `\\wsl.localhost\<distro>\...`) とテスト
- [x] 偽のエディタ (テストの中で持ち主を実装し HTTP で待ち受ける) につないで、中継で一通りのツールが動くテスト
- [x] README と `docs/ai-integration.md` を更新し、`just ci` が通る

## 参考

- `docs/adr/0023-editor-mcp-bridge.md`
- `docs/tasks/T0514-mcp-http-transport.md`・`docs/tasks/T0515-mcp-root-dir.md` のメモ

## メモ

- やったこと
  - 持ち主の抽象 (`src/host.rs`): `DocumentHost` の `with_project` (文書・メタ情報・`Proposals` を `Lent` で貸す)・`open`
    (`Opening::Load` か `New`)・`close`・`save`・`authorize` (`Action`: 適用・Undo・Redo・保存・開く・PSD から作る・閉じる。
    待ってよい)・`changed` (`Change`: 握手・提案・編集・保存・開閉。提案にはクライアントの名前)・`instructions`。
    `FileHost` が今までの振る舞い (パスごとに開き、無ければディスクから読む。`--root` のリンクの検査も移した)。
    ツールの形・名前・結果は変えていない (既存の stdio・HTTP・GPU のテストはそのまま通る)。
  - `Tools`・`Server`・HTTP の `Endpoint` を `&self` にした。前は HTTP の要求を `Mutex<Server>` で 1 つずつ処理していたが、
    エディタで確認を待つ適用がほかの要求を止めないよう、接続ごとのスレッドで同時に処理する。持ち主の錠は文書を読み書きする
    短い間だけ取り (描画と PSD の画像は複製を取って錠の外で)、`authorize`・`changed` はどの錠も握らずに呼ぶ。確認の後に
    モデルが変わっていれば、提案が古いというエラー (承認した差分と違う結果にしない)。適用できない提案では確認を出さない。
    ツールだけの状態 (`revision`・PSD と分類) は `Tools` がパスごとに持つ (PSD は別の錠)。
  - `host::Approvals`: 確認を出して答えか時間切れまで待つ・画面から待たずに答える・`deny_all` (エディタが使う部品)。
  - エディタに組み込む API (`src/bridge.rs`): `EditorBridge::start(host, BridgeOptions)` が `127.0.0.1` で待ち受け (トークン
    必須。無ければ作る)、発見用のファイルを書き、`stop`・`Drop` で止めて消す (ほかのエディタが書き換えていれば残す)。
    `HttpServer::spawn` と `RunningHttpServer::stop` を足した (止めた後は開いている接続にも 503)。
  - 発見用のファイル (`src/discovery.rs`): `{format, version, pid, port, token, endpoint}`。Windows は `%LOCALAPPDATA%`、macOS は
    `~/Library/Application Support`、Linux は `$XDG_STATE_HOME` か `~/.local/state` の下の `app.ugoe.editor/editor-mcp.json`。
    `UGOE_MCP_DISCOVERY` で変えられる。Unix はディレクトリ 0700・ファイル 0600、一時ファイルから名前を変えて置き換える。
    ディレクトリ名を `ugoe` にしなかった理由: Windows の `%LOCALAPPDATA%\ugoe` はエディタ (NSIS の利用者ごとの
    インストール) のインストール先だった。エディタの識別子 (`tauri.conf.json` の `identifier`) の名前にした。
    新しい依存は無い (std と環境変数だけ)。
  - `ugoe-mcp --editor` (`src/relay.rs`): 要求のたびに発見用のファイルを探して HTTP で送る (エディタを後から起動・再起動
    してよい)。つながらない・401 なら読み直して 1 回だけ送り直す (要求が届いていないと分かるときだけ)。見つからない・
    つながらない (PID と発見用のファイルを示す)・形の版が違うときは、ツールの呼び出しにはツールのエラー、ほかには
    JSON-RPC のエラー。ugoe の版の違いは警告だけ (ツールはエディタのものなので)。要求ごとに版を示す版のヘッダを本文から
    付ける。相対パスは中継の作業ディレクトリから絶対パスにする。HTTP のクライアントはワークスペースの `ureq`
    (ugoe-ai が使うもので新しい依存ではない。dev-dependency から dependency に移した)。
  - WSL (`src/wsl.rs`): `WSL_DISTRO_NAME` (無ければ `/proc` と `wslpath -w /`) で判定。Windows の `%LOCALAPPDATA%` は
    相互運用の `cmd.exe /d /c echo %LOCALAPPDATA%` (作業ディレクトリは `/mnt/c`)、失敗・無効なら
    `/mnt/*/Users/*/AppData/Local/app.ugoe.editor/editor-mcp.json` を探す。Linux の場所にもあれば新しいほう。
    自動マウントの根は `/etc/wsl.conf` の `[automount] root`。パスの変換 (`/mnt/c/…` ↔ `C:\…`、ほかの絶対パス ↔
    `\\wsl.localhost\<distro>\…`、`\\?\`・`\\wsl$\`・`\\?\UNC\` も受ける) は引数 (`path`・`psd`・`bundle`・パスの形の
    `motion`・`expression`) と結果 (`path`・`psd`・`bundle`・`closed`・エラーの文の `` `…` `` の中)。つながらないときは
    ミラーのネットワークか Windows 版を使うよう案内する。Windows 版は `--wsl-distro` か、作業ディレクトリ
    (`\\wsl.localhost\<名前>\…`) からディストリビューションを知って同じ変換をする。
  - テスト: `tests/editor.rs` (偽のエディタ = エディタの `AppState` と同じく文書と提案を別々の錠に持つ持ち主を
    `EditorBridge` で待ち受け、子プロセスの `--editor` でつなぐ。エディタの起動前・握手・一覧・開く (ほかは断る)・提案と
    クライアントの名前・拒否・時間切れ・承認して適用・Undo/Redo・保存と書き出し・停止・再起動・古い発見用のファイル・
    知らない版・確認を待つ間にほかのクライアントが答えを得る・確認の間にモデルが変わったら適用しない・トークン)。
    変換の規則は `wsl.rs`・`relay.rs` の単体テスト、発見用のファイルは `discovery.rs`、`Approvals` は `host.rs`。
  - 文書: README (`--editor`・発見用のファイル・WSL・エディタに組み込む)、`docs/ai-integration.md` (実行中のエディタに
    つなぐ)。
- 確かめたこと
  - `just ci`・`just test-gpu`・`cargo xwin clippy --target x86_64-pc-windows-msvc -p ugoe-mcp --tests -- -D warnings` が通る。
  - Windows で単体テストを動かした (`cargo xwin test --no-run` の .exe を WSL の相互運用で実行)。`tests/editor.rs` の
    トークンの検査も Windows で通った (ほかは子プロセスのパスが Linux のパスになるので動かせない)。
  - 実機の WSL2 (ミラー) で端から端まで: Windows 側で `EditorBridge` (持ち主は `FileHost`。scratchpad の使い捨ての
    プログラム) を動かし、(1) WSL の Linux 版の `--editor` (発見用のファイルを `cmd.exe` で見つけ、パスを変換) と
    (2) WSL から起動した Windows 版の `ugoe-mcp.exe --editor` (作業ディレクトリからディストリビューションを知る) の両方で、
    `C:` の下と WSL の `/tmp` の下にプロジェクトを作って保存・書き出しし、結果のパスが WSL のパスで返ること、相対パスが
    中継の作業ディレクトリから解決されること、エディタの終了で発見用のファイルが消え、中継が「動いていない」と返すことを
    確かめた。試しに置いたファイルは消した。
- 確かめていないこと
  - WSL の NAT のネットワーク (つながらないときの案内の文) と、相互運用が無効な WSL (`/mnt/*/Users/*` を探す経路)。
  - macOS の場所。実際の Tauri のエディタに組み込むこと (T0642)。Claude Code・OpenCode から実際に登録してつなぐこと。
- 範囲外で見つけたこと (直していない)
  - Windows で `http::tests::slow_requests_time_out_with_408` が落ちる (T0514 のテスト)。少しずつ送るクライアントに、サーバが
    408 を書いて閉じると、読み残しがあるため Windows は RST を送り、クライアントは応答を読めない。サーバの振る舞いとしては
    困らない (切るのが目的) が、テストは Unix でしか通らない。
  - T0515 の `root::tests::rejects_dot_dot_in_the_missing_part` は Windows では成り立たない (Windows は無い部分の `..` も字面で
    解く)。字面で解いた結果で root の中かを判定するので守りは保たれる。Unix だけのテストにし、Windows の振る舞いのテストを
    足した (同じクレートなので直した)。
- T0642 への引き継ぎ
  - 持ち主の実装の形は `tests/editor.rs` の `FakeEditor` と README の「エディタに組み込む」。`with_project` は `document` →
    `project` の順に錠を取る。`open` の `Opening::Load` は開いているプロジェクトなら何もしない (置き換えない)。保存は
    エディタの保存と同じ処理にする (`meta_modified` など)。未保存 (パスの無い) プロジェクトはパスで指せないので、
    `instructions` で保存を促すか、画面で保存を求める。
  - 案内の文は「Settings > Connect agents」を指している (`RelayError::NotFound`)。画面の名前が変わったら合わせる。
  - Windows 版の `ugoe-mcp.exe` を同梱したときの WSL の登録のコマンドは README の例 (インストール先のパスは仮)。
- 後続の候補
  - クライアントの区別 (HTTP はセッションを持たないので、提案の出どころは最後に握手したクライアントの名前)。中継が
    ヘッダでクライアントを示す、またはセッションを発行する。
  - 未保存のプロジェクトを指す仮のパス (例: `editor:current`) をツールの形を変えずに受ける。
