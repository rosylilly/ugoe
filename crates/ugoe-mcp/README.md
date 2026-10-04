# ugoe-mcp

**層:** T4 アプリ (依存してよいクレートは `scripts/check-layers.mjs` を参照)

MCP (Model Context Protocol) サーバ。Claude Code などの外部のエージェントが `.ugoeproj` を開いてモデルを読み、
`ugoe_editor_core::Command` の列で編集を提案・適用できるようにする。PSD からの自動リギングでは、画像を見て
レイヤーを分類し、リグの値を決める判断をエージェントに任せる (ugoe は API を呼ばない。ADR 0022)。設計の判断
(ADR 0006 との関係、`.ugoeproj` を直接扱うこと) は [docs/ai-integration.md#mcp-サーバ](../../docs/ai-integration.md#mcp-サーバ)。

`.ugoeproj` を自分で開く形 (既定) のほかに、実行中のエディタの文書を扱う形がある (ADR 0023): エディタが同じツールを
ループバックの HTTP で出し ([エディタに組み込む](#エディタに組み込む))、`ugoe-mcp --editor` が stdio のクライアントを
そこへ中継する ([実行中のエディタにつなぐ](#実行中のエディタにつなぐ---editor))。

## 動かし方

既定は stdio のサーバ (1 行 1 メッセージの JSON-RPC 2.0)。MCP クライアントが子プロセスとして起動する。
`--http` を付けると HTTP (MCP の Streamable HTTP) で待ち受ける ([HTTP で動かす](#http-で動かす-chatgpt-など))。
`--editor` を付けると、サーバの代わりに実行中のエディタへの中継になる ([実行中のエディタにつなぐ](#実行中のエディタにつなぐ---editor))。

```sh
cargo build --release -p ugoe-mcp   # target/release/ugoe-mcp
# あるいは PATH に入れる
cargo install --path crates/ugoe-mcp
```

| オプション | 内容 |
| --- | --- |
| `--read-only` | 読むだけにする。`apply_proposal`・`undo`・`redo`・`save_project` を出さない (提案と描画はできる) |
| `--root <DIR>` | ツールが読み書きするパスを `<DIR>` の下に限る。相対パスも `<DIR>` から解決する ([パスを限る](#パスを限る---root))。`--http` ではこれか `--allow-any-path` が要る |
| `--allow-any-path` | `--http` で `--root` を付けずに、ツールに任意のパスを読み書きさせる (勧めない) |
| `--backend <NAME>` | 描画に使う wgpu のバックエンド (`vulkan` `metal` `dx12` `gl`、カンマ区切り)。省略時は `WGPU_BACKEND`、それも無ければ wgpu の既定 |
| `--http <ADDR>` | stdio の代わりに HTTP で待ち受ける。`127.0.0.1:8765` の形か、ポート番号だけ (`8765` はループバックの `127.0.0.1:8765`)。エンドポイントは `/mcp` |
| `--token <TOKEN>` | HTTP の要求に求めるトークン (32 文字以上。16 進数で 128 ビット以上)。環境変数 `UGOE_MCP_TOKEN` でも渡せる (コマンド行は `ps` でほかの利用者から見えることがあるので、環境変数を勧める) |
| `--generate-token` | トークンを OS の乱数で作り (256 ビット、16 進数で 64 文字)、標準エラーに書く |
| `--allow-origin <ORIGIN>` | ループバックのほかに受け付ける `Origin` (`https://example.com` の形。繰り返せる) |
| `--editor` | 実行中のエディタにつなぐ (stdio をエディタの MCP サーバへ中継する)。ほかのサーバのオプションとは一緒に使えない |
| `--discovery <FILE>` | `--editor` で読む発見用のファイル (既定は [OS ごとの場所](#発見用のファイル)。環境変数 `UGOE_MCP_DISCOVERY` でも変えられる) |
| `--wsl-distro <NAME>` | `--editor` で、エージェントが動く WSL のディストリビューションの名前 (Windows 版を WSL から使うとき。WSL のパスを Windows のパスに変える) |
| `--mcpb <OUT>` | サーバを動かさず、この実行ファイルの `--editor` を包む Claude Desktop 用の MCP Bundle (`.mcpb`) を `<OUT>` に書く ([Claude Desktop に入れる](#claude-desktop-に入れる-mcpb)) |
| `--mcpb-icon <PNG>` | `--mcpb` で束に入れるアイコン (PNG。512×512 を勧める) |

ログは標準エラーに書く (stdio では標準出力は MCP のメッセージだけ)。

### Claude Code に登録する

```sh
# このリポジトリの中だけで使う (プロジェクトのスコープ。.mcp.json に書かれる)
claude mcp add --scope project ugoe -- /path/to/ugoe/target/release/ugoe-mcp
# 自分の環境のどこでも使う
claude mcp add --scope user ugoe -- ugoe-mcp
# 読むだけにする。描画のバックエンドを GL に固定する
claude mcp add ugoe-read -- ugoe-mcp --read-only --backend gl
# 触れるパスを 1 つのディレクトリの下に限る
claude mcp add ugoe-art -- ugoe-mcp --root ~/art
```

`claude mcp list` で `ugoe` が `Connected` になっていればよい。Claude Code の中では `/mcp` で状態とツールを見られる。
相対パスはサーバの作業ディレクトリ (Claude Code を起動したディレクトリ) から解決する (`--root` を付けたらそのディレクトリから)。

読むだけのツールを確認なしで呼ばせるには、Claude Code の設定 (`.claude/settings.json` など) の `permissions.allow` に
足す。`mcp__ugoe__apply_proposal` と `mcp__ugoe__save_project` は足さず、呼ぶたびに確認させる
(提案の承認にあたる。[docs/ai-integration.md#提案と適用-adr-0006-との関係](../../docs/ai-integration.md#提案と適用-adr-0006-との関係))。

```json
{
  "permissions": {
    "allow": [
      "mcp__ugoe__open_project", "mcp__ugoe__get_model_summary", "mcp__ugoe__get_element",
      "mcp__ugoe__propose_edit", "mcp__ugoe__list_proposals", "mcp__ugoe__render_png",
      "mcp__ugoe__import_psd", "mcp__ugoe__get_psd_layers", "mcp__ugoe__set_layer_labels",
      "mcp__ugoe__get_rig_guide", "mcp__ugoe__propose_rig",
      "mcp__ugoe__get_module_rig_guide", "mcp__ugoe__propose_module_rig"
    ]
  }
}
```

`get_psd_layers`・`get_rig_guide`・`get_module_rig_guide` は画像を返す。Claude Code は MCP のツールの結果の大きさに上限 (既定 25,000 トークン。
画像も数える。環境変数 `MAX_MCP_OUTPUT_TOKENS` で変えられる) を設けるので、`get_psd_layers` は既定で 8 枚ずつ返す。

### OpenCode に登録する

`opencode.json` (プロジェクトの直下、または `~/.config/opencode/opencode.json`) の `mcp` に、ローカル (stdio) の
サーバとして書く。

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "ugoe": {
      "type": "local",
      "command": ["/path/to/ugoe/target/release/ugoe-mcp"],
      "enabled": true,
      "timeout": 60000
    }
  },
  "permission": {
    "ugoe_apply_proposal": "ask",
    "ugoe_save_project": "ask"
  }
}
```

- `timeout` はミリ秒。PSD の取り込み (レイヤーごとの自動メッシュ) とリグの提案は数秒かかることがあるので、既定より長くする。
- OpenCode は MCP のツールを `<サーバ名>_<ツール名>` の名前で扱う。適用と保存を呼ぶたびに確かめさせるには、
  `permission` で `ask` にする (ADR 0006 の承認にあたる。ツールの名前での許可の書き方は OpenCode の版で違う
  ことがあるので、[OpenCode の文書](https://opencode.ai/docs/permissions/) で確かめる)。

**確かめていないこと:** Claude Code・OpenCode から実際につなぐことは確かめていない (ユーザーの設定を変えない
ため)。サーバの側は、stdio で JSON-RPC を流すテスト (`tests/stdio.rs`) で同じ流れを確かめている。

### HTTP で動かす (ChatGPT など)

ChatGPT のように、サーバを子プロセスとして起動せず HTTP でつなぐクライアントのために、MCP の Streamable HTTP で
待ち受けられる。ツールと状態は stdio と同じ。

```sh
ugoe-mcp --http 8765 --root ~/art                      # http://127.0.0.1:8765/mcp (ループバックだけ。トークンなし)
ugoe-mcp --http 8765 --root ~/art --generate-token    # トークンを作って求める (標準エラーに書く)
UGOE_MCP_TOKEN=$(openssl rand -hex 32) ugoe-mcp --http 8765 --root ~/art   # 自分で作ったトークンを求める
```

HTTP では `--root <DIR>` (ツールが触れるパスを `<DIR>` の下に限る) か `--allow-any-path` (限らないことを明示する) の
どちらかが要り、どちらも無ければ起動しない ([パスを限る](#パスを限る---root))。
起動すると標準エラーに `listening on http://127.0.0.1:8765/mcp (...; paths confined to ...)` と書く。サーバは Ctrl-C まで動き続け、
開いたプロジェクトと提案はプロセスに 1 つだけ持つ (つないだすべてのクライアントで共有する。保存するまで
ディスクに書かないのは stdio と同じ)。

#### 守り

HTTP で待ち受けると、同じ計算機のほかのプログラムやブラウザのページからも届く。ツールは利用者の権限で
プロジェクトと PSD を読み、`save_project` はディスクに書くので、次のように守る。

- **パスを限る。** `--root <DIR>` で、ツールが触れるパスを `<DIR>` の下に限る ([パスを限る](#パスを限る---root))。
  HTTP では `--root` か `--allow-any-path` のどちらかが要る。`--read-only` だけでは足りない (書けなくても、
  任意のプロジェクトや PSD を読んで画像で返せる)。
- **既定はループバックだけ。** `--http 8765` は `127.0.0.1` に束ねる。ループバック以外 (`0.0.0.0:8765` など) に
  束ねるにはトークンが要り、無ければ起動しない。
- **トークン。** `--token` (または `UGOE_MCP_TOKEN`)・`--generate-token` で、すべての要求にトークンを求める。
  推測されないように 128 ビット以上 (16 進数で 32 文字以上) を求め、`--generate-token` は OS の乱数で 256 ビットを
  作る。比較は定数時間で行う。`Authorization: Bearer <token>` のヘッダで渡すか、ヘッダを送れないクライアントの
  ために URL のパス (`http://.../mcp/<token>`) に入れる。違えば `401`。
  **URL に入れたトークンはログに残りやすい** (トンネルやプロキシのアクセスログ、クライアントの設定や履歴、画面の
  共有)。ヘッダで渡せるクライアントではヘッダを使い、URL の形を使ったら、使い終えたときにトークンを変える。
  `--generate-token` のトークンは標準エラーに書くので、標準エラーをログに残す環境では気を付ける。
- **トンネルを通した公開の防止。** トークンが無いときは、`Host` がループバックの名前 (`localhost`・`127.0.0.1`・
  `[::1]`) で、プロキシやトンネルの転送ヘッダ (`X-Forwarded-For`・`Forwarded`・`CF-Connecting-IP` など) が無い
  要求だけを受け、ほかは `403`。ngrok や Cloudflare Tunnel などでループバックのポートを外に出しても、
  トークンなしでは使えない (転送ヘッダを付けず `Host` を書き換えるトンネルは見分けられないので、
  トンネルを使うときは必ずトークンを付ける)。
- **DNS リバインディング対策。** `Origin` が付いた要求は、ループバックの `Origin` か `--allow-origin` で
  許したものだけを受け、ほかは `403` (MCP の仕様の求め)。ブラウザで開いた悪意のあるページからは届かない。
- **上限。** 本文は 32 MiB まで (超えたら `413`)、要求の行とヘッダは合わせて 64 KiB・100 個まで (`431`)、
  1 つの要求を読み終えるまで 60 秒まで (keep-alive で次の要求を待つ時間を含む。少しずつ送り続ける相手も切る。
  `408`)、同時の接続は 32 まで (`503`)。超えたら答えて接続を閉じる。
- TLS は扱わない (外に出すときは HTTPS を終端するトンネルを使う)。

#### パスを限る (`--root`)

`--root <DIR>` で起動すると、ツールが読み書きするすべてのパスを `<DIR>` の下に限る (`src/root.rs`)。プロジェクト
(`path`)・`import_psd` と `get_psd_layers` の `psd`・`save_project` の `bundle`・`render_png` の `motion` と `expression`
(プロジェクトの中の名前でなくファイルのパスのとき) が対象。stdio でも HTTP でも使える (stdio の既定は限らない)。

- 相対パスは `<DIR>` から解決する。`<DIR>` の下を指す絶対パスも使える。
- 正規化 (シンボリックリンクと `..` を解く) してから `<DIR>` の下にあるかを判定する。`..` で上に出るパス・外の絶対パス・
  外を指すシンボリックリンク・Windows のほかのドライブや UNC のパスは、ツールのエラー (`... is outside the root
  directory ...`) になる。まだ無いパス (新しいプロジェクト・書き出し先) は、存在する祖先を正規化して判定する
  (無い部分に `..` があったり、行き先の無いリンクがあったりすれば拒否する)。
- プロジェクトのディレクトリの中のファイルが外を指すシンボリックリンクなら、開く前と保存の前に拒否する。
- **モジュールの本体も限る (T0524)。** モジュールのプロジェクトの本体の参照 (`project.json` の `module.base`。相対・絶対・
  `..` を含むもの) と `import_psd` の `base` は、解決した本体のパスを同じ規則で判定し、外なら本体を開く前にエラー
  (`base model <本体のパス>: outside the root directory ...`) にする。外を指すシンボリックリンクと、本体の `.ugoeproj` の
  中の外へのリンクも拒否する。外のまだ無いパスも「見つからない」ではなく「外」と答える (外のファイルの有無を探らせない)。
  本体にモジュールのプロジェクトを選んだときは、その本体まで読みに行かずにエラーにする。
- **防がないこと:** 判定の後、読み書きの前に、ほかのプロセスが `<DIR>` の中をシンボリックリンクに差し替える競合 (TOCTOU)。
  ツールはリンクを作れないので MCP のクライアントだけでは起こせないが、`<DIR>` を信頼しないほかのプログラムに書かせない
  こと。判定はツールを呼ぶたびにし直す。設計は [docs/ai-integration.md#パスを限る---root](../../docs/ai-integration.md#パスを限る---root)。

#### ChatGPT から使う

ChatGPT の開発者モード (設定の「Security and login」→「Developer mode」。使えるプランは
[OpenAI の文書](https://developers.openai.com/api/docs/guides/developer-mode) を見る) で、リモートの MCP サーバとして
登録する。ChatGPT は OpenAI のサーバからつなぐので、手元のループバックのサーバには直接届かない。どちらかの
方法で届くようにする。認証は「No Authentication」を選ぶ (ChatGPT のコネクタは OAuth か認証なしだけで、
Bearer のヘッダを付けられない)。

1. **Secure MCP Tunnel (勧める)。** OpenAI の [`tunnel-client`](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels)
   を手元で動かし、外向きの接続だけで ChatGPT とサーバをつなぐ (公開の URL を作らない)。`tunnel-client` は
   サーバを子プロセスとして stdio で起動できるので、この場合 `--http` は要らない (`tunnel_id` と API キーは
   OpenAI の Platform の設定で作る)。

   ```sh
   tunnel-client init --sample sample_mcp_stdio_local --profile ugoe \
     --tunnel-id <tunnel_id> --mcp-command "ugoe-mcp --read-only --root /path/to/art"
   tunnel-client doctor --profile ugoe --explain
   tunnel-client run --profile ugoe
   ```

   HTTP のサーバにつなぐなら、`ugoe-mcp --http 8765 --read-only --root /path/to/art` を動かし、`--mcp-command` の代わりに
   `--mcp-server-url http://127.0.0.1:8765/mcp` を渡す (対応する `--sample` の名前は OpenAI の文書で確かめる)。
   `tunnel-client` が転送ヘッダを付けるなどで `403` になるときは、トークンを付けてパスの形の URL
   (`http://127.0.0.1:8765/mcp/<token>`) を渡す。ChatGPT のプラグインの画面で開発者モードのアプリを作り、
   接続に「Tunnel」を選ぶ。
2. **公開のトンネル (ngrok・Cloudflare Tunnel など)。** 公開の HTTPS の URL を作るので、**必ずトークンを付ける**。
   ChatGPT はヘッダを送れないので、URL のパスにトークンを入れる。

   ```sh
   ugoe-mcp --http 8765 --read-only --root ~/art --generate-token   # 標準エラーの token をひかえる
   ngrok http 8765          # https://<name>.ngrok.app を得る
   ```

   ChatGPT には `https://<name>.ngrok.app/mcp/<token>` を登録する。URL を知っている人は誰でもツールを呼べるので、
   URL をほかの人に見せない (トンネルの事業者のログ・ChatGPT の設定・ブラウザの履歴に残りうる)。使い終えたら
   トンネルを止め、トークンを変える (`--generate-token` なら起動し直すたびに変わる)。

- ChatGPT がブラウザのように `Origin` を付けて `403` になるときは、`--allow-origin https://chatgpt.com` を足す。
- ChatGPT は `readOnlyHint` の無いツール (`apply_proposal`・`undo`・`redo`・`save_project`) を書き込みとして扱い、
  呼ぶ前に確認を求める (ADR 0006 の承認にあたる)。外に出すときは `--read-only` で始め、提案 (`propose_edit`) と
  描画 (`render_png`) だけを使わせ、適用は手元の Claude Code やエディタで行うことを勧める。
- パスは `--root` のディレクトリから解決する。ChatGPT には root からの相対パス (例: `character.ugoeproj`) を伝える。

**確かめていないこと:** 実際の ChatGPT (開発者モード・Secure MCP Tunnel・ngrok) からつなぐことは確かめていない。
サーバの側は、HTTP で握手・ツールの一覧・ツールの呼び出し (提案・適用・保存) を流すテスト (`tests/http.rs`) で
確かめている。

### 実行中のエディタにつなぐ (`--editor`)

エディタの設定の「Connect agents」で有効にすると (T0642)、エディタは開いている文書を対象にした同じツールを
ループバックの HTTP で出し、ポートとトークンを [発見用のファイル](#発見用のファイル) に書く。`ugoe-mcp --editor` は
そのファイルを読み、stdio のメッセージをエディタへ中継する。stdio しか話せないクライアント (Claude Code・OpenCode) は
これを登録する。

```sh
claude mcp add --scope user ugoe-editor -- ugoe-mcp --editor
```

- **提案はエディタの画面に出る。** `propose_edit`・`propose_rig` の提案は、エディタの AI タブの提案の一覧に置かれる。
  `apply_proposal`・`undo`・`redo`・`save_project` は、エディタが確認のダイアログを出し、利用者が承認したときだけ
  行う (拒否・時間切れはツールのエラー)。確認の間に利用者がモデルを変えたら、提案が古いというエラーになる。
- **開けるのはエディタで開いているプロジェクトだけ。** `open_project` はそのプロジェクトのパスなら状態を返し
  (ディスクの内容で置き換えない)、ほかのプロジェクトを開く・作る・閉じる・`import_psd` はできない (エディタが断る)。
  `instructions` に、開いているプロジェクトのパスが書かれる。
- **相対パス:** 中継の作業ディレクトリ (Claude Code を起動したディレクトリ) から絶対パスにしてエディタへ渡す。
- **エディタが無いとき:** 中継は起動したままにし、要求のたびに発見用のファイルを探す。見つからない・つながらない・
  形の版が違うときは、直し方をツールのエラー (ツールの呼び出し) か JSON-RPC のエラー (ほかの要求) で返す。
  エディタを後から起動・再起動 (ポートとトークンが変わる) してもよい。エディタと中継の ugoe の版が違えば、標準エラーに
  警告を書く (ツールはエディタのものが使われる)。

#### 発見用のファイル

エディタが起動中に書き、止めると消す (`src/discovery.rs`)。中身はポート・トークン・PID・版 (`format` と `version`)。
トークンを含むので、Unix ではディレクトリを 0700、ファイルを 0600 で作る (Windows の `%LOCALAPPDATA%` は利用者ごとの
アクセス権を持つ)。ディレクトリの名前はエディタの識別子 (`app.ugoe.editor`)。

| OS | 場所 |
| --- | --- |
| Windows | `%LOCALAPPDATA%\app.ugoe.editor\editor-mcp.json` |
| macOS | `~/Library/Application Support/app.ugoe.editor/editor-mcp.json` |
| Linux | `$XDG_STATE_HOME/app.ugoe.editor/editor-mcp.json` (無ければ `~/.local/state/…`) |

中継は、`endpoint` が `/` で始まる URL のパス (`@`・`?`・`#` などを含まない) で、`port` が 0 でないファイルだけを読む
(ほかのホストへトークンを送らないように)。
環境変数 `UGOE_MCP_DISCOVERY` でファイルのパスを変えられる (エディタと中継の両方が見る)。エディタを 2 つ起動すると、
後から有効にしたほうが書く (中継は後のほうにつなぐ)。

#### Windows のエディタと WSL のエージェント

推奨は、WSL から Windows 版の `ugoe-mcp.exe --editor` を登録すること (WSL の相互運用で .exe を起動でき、通信は
Windows の中のループバックで完結する)。`ugoe-mcp.exe` はエディタに同梱され、エディタの実行ファイルの隣に入る
(NSIS の利用者ごとのインストールなら `%LOCALAPPDATA%\ugoe\`)。エディタの設定の「Connect agents」の「Agents in WSL」が、
実際のパスを埋めたコマンドを出す。

```sh
# WSL の中で。エディタのインストール先の ugoe-mcp.exe を指す
claude mcp add --scope user ugoe-editor -- /mnt/c/Users/<名前>/AppData/Local/ugoe/ugoe-mcp.exe --editor --wsl-distro "$WSL_DISTRO_NAME"
```

- **パスの変換:** エージェントが渡す WSL のパスを Windows のパスに、結果のパスを WSL のパスに戻す (下の表)。
  `--wsl-distro` を省いても、Linux のディレクトリ (`\\wsl.localhost\<名前>\…` になる) で起動すれば名前が分かる。
  `/mnt/c/…` で起動するなら `--wsl-distro` が要る (無ければ `/home/…` のパスを変えられない)。
- **Linux 版でもつなげる:** WSL の中の Linux 版の `ugoe-mcp --editor` は、Linux の場所に発見用のファイルが無ければ、
  Windows の `%LOCALAPPDATA%` (相互運用の `cmd.exe` で尋ねる。相互運用が無効なら `/mnt/<ドライブ>/Users/*/AppData/Local`
  の下を探す。ほかの利用者も書ける `Public`・`Default` などの共有のプロファイルは見ない) の発見用のファイルを読み、パスを変換して `127.0.0.1` へつなぐ。WSL のネットワークが「ミラー」
  (`%UserProfile%\.wslconfig` の `networkingMode=mirrored`) のときだけ Windows のループバックに届く。NAT のときは
  つながらず、そう案内するエラーを返す (Windows 版を使うか、ミラーにする)。

| WSL | Windows |
| --- | --- |
| `/mnt/c/Users/a/x.ugoeproj` | `C:\Users\a\x.ugoeproj` |
| `/home/a/x.ugoeproj` | `\\wsl.localhost\<distro>\home\a\x.ugoeproj` |

変換するのは、ツールの引数の `path`・`psd`・`bundle`・パスの形の `motion` と `expression`、結果の `path`・`psd`・
`bundle`・`closed` と、エラーの文の `` `…` `` で囲んだパス (`src/relay.rs`・`src/wsl.rs`)。自動マウントの根は
`/etc/wsl.conf` の `[automount] root` (既定 `/mnt/`) に従う。

**確かめたこと (この環境は WSL2 のミラー):** Windows 側で `EditorBridge` (持ち主は `FileHost`) を動かし、WSL の Linux 版の
`--editor` (Windows の `%LOCALAPPDATA%` を `cmd.exe` で見つけ、パスを変換) と、WSL から起動した Windows 版の `--editor`
(作業ディレクトリからディストリビューションの名前を知る) の両方で、`C:` の下と WSL の `/tmp` の下のプロジェクトを作って保存し、
結果のパスが WSL のパスで返ることを確かめた。NAT のネットワークと、相互運用が無効な WSL では確かめていない。

### Claude Desktop に入れる (`.mcpb`)

Claude Desktop には、MCP の stdio のサーバを MCP Bundle (`.mcpb`) にして拡張機能として入れられる。`--mcpb` は
`--editor` の中継を包む (`src/mcpb.rs`)。エディタは「Connect agents」の「Claude Desktop」で、同梱の `ugoe-mcp` から同じ束を
その場で作る (利用者向けは [操作マニュアル](../../docs/manual/mcp.md#claude-desktop))。

```sh
cargo build --release -p ugoe-mcp
target/release/ugoe-mcp --mcpb ugoe-editor.mcpb --mcpb-icon apps/editor/src-tauri/icons/icon.png
# 公式の CLI で確かめる (任意。npm から取ってくる)
npx @anthropic-ai/mcpb info ugoe-editor.mcpb
npx @anthropic-ai/mcpb unpack ugoe-editor.mcpb out && npx @anthropic-ai/mcpb validate out/manifest.json
```

- 束は zip で、`manifest.json`・`icon.png` (あれば)・`server/ugoe-mcp` (Windows は `server/ugoe-mcp.exe`。Unix の権限 0755) を持つ。
- `manifest.json` は `manifest_version` 0.3 (公式の CLI の既定)、`server.type` は `binary`、`mcp_config` は
  `${__dirname}/server/ugoe-mcp[.exe]` と `--editor`。1 つの束には 1 つの OS の実行ファイルだけを入れ、`compatibility.platforms`
  をその OS (`darwin`・`win32`・`linux`) にする。`tools` は `tools/list` (エディタのブリッジと同じ既定の設定) から作り、
  説明には英語の `title` を使う。`user_config` は無い (ポートとトークンは発見用のファイルから読む)。
- 仕様: [MANIFEST.md](https://github.com/modelcontextprotocol/mcpb/blob/main/MANIFEST.md)・
  [manifest 0.3 の JSON Schema](https://github.com/modelcontextprotocol/mcpb/blob/main/schemas/mcpb-manifest-v0.3.schema.json)・
  [Build a desktop extension with MCPB](https://claude.com/docs/connectors/building/mcpb)。署名 (`mcpb sign`) はしていない
  (公式の CLI の `info` は「Not signed」と出す。Claude Desktop が署名の無い束をどう見せるかは確かめていない)。
- zip は自前の小さな書き手 (deflate は `miniz_oxide`。ugoe-psd が使っているもの)。`zip` のクレートは使っていない
  (書くだけで、新しい依存を増やさないため)。時刻は固定し、同じ入力から同じバイト列を作る。
- 確かめたこと: `tests/editor.rs` が `--mcpb` で束を作り、展開して manifest を確かめ、manifest のコマンドを起動して偽のエディタに
  つながることを確かめる。公式の CLI (`@anthropic-ai/mcpb` 2.1.2) の `info`・`unpack`・`validate` も手で通した (Linux の束)。
  **実際の Claude Desktop に入れることは確かめていない** (開発した環境が WSL のため)。

### エディタに組み込む

エディタ (T0642) は、開いている文書を貸す持ち主 (`ugoe_mcp::host::DocumentHost`) を実装し、
`ugoe_mcp::EditorBridge::start` で待ち受ける。実装は `apps/editor/src-tauri/src/mcp.rs` の `EditorHost`
(設計は [docs/editor.md#エージェントをつなぐ](../../docs/editor.md#エージェントをつなぐ)、操作は
[操作マニュアル](../../docs/manual/mcp.md))。エディタはこのクレートの `ugoe-mcp` をサイドカーとして同梱する。

```rust,ignore
let host: Arc<dyn DocumentHost> = Arc::new(EditorHost::new(app_handle));
let bridge = EditorBridge::start(host, BridgeOptions::default())?; // 127.0.0.1 の空きポート・トークンを作る・発見用のファイルを書く
// bridge.url()・bridge.token() を設定の画面に出す。止めるときは bridge.stop() (発見用のファイルを消す)
```

- **`with_project(path, f)`:** 開いているプロジェクトなら、文書 (`Mutex<Document>`)・メタ情報・提案 (`Proposals`) の錠を
  取って `Lent` を `f` に渡す。ほかのパスは `HostError::Denied`。`f` の中でツールは待たない (描画や PSD の画像は複製を
  取ってから錠の外で扱う)。
- **`authorize(action)`:** 適用・Undo・Redo・保存は確認のダイアログを出して待つ (`host::Approvals`: `request` で番号を作って
  画面にイベントを送り、答えか時間切れまで待つ。画面の Tauri のコマンドが `answer` で答える)。開く・閉じる・`import_psd` は
  断る (開いているプロジェクトを `open_project` するのは許してよい)。どの錠も握らずに呼ばれる。
- **`open`・`close`・`save`:** `open` は開いているプロジェクトの `Opening::Load` なら何もしない (利用者の文書を置き換えない)。
  `save` はエディタの保存と同じ処理 (メタ情報の変更の扱いを含む)。
- **`changed(change)`:** 提案が置かれた (クライアントの名前付き)・文書が変わった・保存した・クライアントが握手した、を知らせる。
  画面へのイベントを送るだけにし、待たない。
- **`instructions()`:** つながっていることと、開いているプロジェクトのパスを返す (エージェントがパスを知るため)。
- **スレッド:** ツールは HTTP の接続ごとのスレッドで同期に動く (Tauri のイベントループの外)。確認を待つ間も、ほかの要求と
  エディタの編集は進む。止めるときは `Approvals::deny_all` で待っている確認を終わらせてから `bridge.stop()` する。
- **守り:** 待ち受けは `127.0.0.1` だけで、トークンは必須。パスの制限 (`--root` 相当) は `BridgeOptions::options.root` で
  決める。既定は限らない (ほかのプロジェクトは開けず、書く操作は利用者が確認するので)。読むだけのツール (PSD・モーションの
  JSON) は、トークンを持つクライアントなら利用者の権限で読める。
- **確かめたこと:** `tests/editor.rs` の偽のエディタ (上の形で持ち主を実装する) に、子プロセスの `--editor` でつなぎ、
  握手・一覧・開く (ほかは断る)・提案・拒否と時間切れ・承認して適用・Undo/Redo・保存と書き出し・エディタの停止と
  再起動・古い発見用のファイル・知らない版を確かめる。確認を待つ間にほかのクライアントが答えを得ること、確認の間に
  モデルが変わったら適用しないことも確かめる。

### 簡単な編集の例 (パラメータの作成とキーの追加)

Claude Code に次のように頼む:

> `character.ugoeproj` を開いて、パラメータ `AngleX` (-30〜30、既定 0) を作り、メッシュ `Face` の格子に
> `AngleX` の -30 と 30 のキーを加えて。適用する前に提案の差分を見せて。

エージェントは次のようにツールを呼ぶ。

1. `open_project { "path": "character.ugoeproj" }`、`get_model_summary` でメッシュの ID を確かめる
2. `propose_edit` に `commands` を渡す:

   ```json
   [
     { "op": "add_parameter", "parameter": { "id": "AngleX", "name": "Angle X", "min": -30, "default": 0, "max": 30 } },
     { "op": "add_keyform_key", "target": { "kind": "mesh", "id": "Face" }, "parameter": "AngleX", "key": -30 },
     { "op": "add_keyform_key", "target": { "kind": "mesh", "id": "Face" }, "parameter": "AngleX", "key": 30 }
   ]
   ```

   結果は提案の ID (`p1`)・`report` (警告)・`changes` (`{"parameters": {"added": ["AngleX"]}, "meshes": {"changed": ["Face"]}}`。
   種類ごとに `added`・`removed`・`changed`、並べ替え (`reorder_mesh` など) なら `"reordered": true`。テクスチャは同じ大きさの画像への差し替えも `changed`)。
   モデルはまだ変わらない。
3. ユーザーが差分を見て承認すると `apply_proposal { "proposal": "p1" }` (Claude Code は呼ぶ前に許可を求める)
4. キーのセルの形を変える (`set_mesh_keyform` など) 提案を同じように出し、`render_png { "params": { "AngleX": 30 } }` で見た目を確かめる
5. `save_project` で保存する (`bundle` を渡すと `.ugoe` にも書き出す)

同じ流れを `tests/stdio.rs` が、サーバを子プロセスとして起動して確かめている。

### PSD からの自動リギング (判断はエージェント)

エージェントに次のように頼む:

> `character.psd` から `character.ugoeproj` を作って自動リギングして。レイヤーの画像を見て分類し、
> 絵に合う値を選んで提案を出して。適用する前に提案を見せて。

エージェントは次のようにツールを呼ぶ。画像を見て判断するのはエージェントのモデルで、サーバは API を呼ばない。

1. `import_psd { "path": "character.ugoeproj", "psd": "character.psd" }` (レイヤーごとのメッシュを持つ新しい
   プロジェクト。保存するまでディスクに書かない)。PSD から作った既存のプロジェクトなら `open_project` して、
   次の `get_psd_layers` に `psd` を渡す
2. `get_psd_layers { "path": ... }` → 最初のページ: 分類の指示と語彙 (`instructions`)・全体の合成画像・8 枚分の
   サムネイル (`layer N: 名前` の文の直後にその画像)。各レイヤーには名前の規則による案 (`label`・`side`・
   `set_by: "rules"`) と対応するメッシュ (`mesh`) が付く。`next_start` で続きを読む
3. 画像を見て案と違うものを `set_layer_labels` で渡す:

   ```json
   { "path": "character.ugoeproj",
     "layers": [ { "index": 12, "label": "eye_closed", "side": "left", "confidence": 0.9 } ] }
   ```

   結果はメッシュとの対応・どのメッシュにも対応しないレイヤー・確かめたほうがよいレイヤー (`needs_review`)・
   リグを組めるか (`rig.ready`。顔がないと組めない)。同じ部品の別の絵 (一度に 1 つだけ見せる袖 A / B・同じ所にかぶる帽子と
   キャップ) があれば、`"variant_sets": [[3, 5]]` のようにレイヤーの番号の組も渡せる (`layers` を省いて組だけでもよい。
   渡すと前の組をすべて置き換え、`[]` で消す。影・線画と目や口の開閉は組にしない)。組は結果の `variant_sets` と
   `get_psd_layers` の各レイヤーの `variant` (組の最も小さい番号) に出る
4. `get_rig_guide` → 値の説明と範囲 (`instructions`)・顔・目・口の位置 (`layout`)・原画
5. 値を選んで `propose_rig` に渡す:

   ```json
   { "path": "character.ugoeproj",
     "params": { "turn_angle_scale": 0.6, "eye_close_line": 0.72, "eye_close_bend": 0.05,
                 "mouth_open_height": 0.4, "reason": "large anime eyes; keep the turn small" } }
   ```

   範囲の外の値や、組んだリグが検証を通らないときはエラー (`turn_angle_scale is 2.5, but it must be between
   0.3 and 1.2` など) が返るので、直して呼び直す。成功すると提案の ID・理由・差分・コマンドの要約
6. ユーザーが承認したら `apply_proposal` (一部のコマンドを外すなら `exclude`)。`render_png` で
   `{ "AngleX": 30 }`・`{ "EyeLOpen": 0 }`・`{ "MouthOpenY": 1 }` などを描いて確かめ、`save_project` で保存する

同じ流れ (胸から上の実際の PSD) を `tests/stdio.rs` の `agent_driven_auto_rig_from_psd` が確かめている。

### 着せ替えのモジュールの自動リギング

衣装の PSD から、本体のスロットに付けるモジュールのプロジェクトを作ってリギングする
([docs/ai-integration.md#モジュールの自動リギング](../../docs/ai-integration.md#モジュールの自動リギング))。モジュールは
パラメータを足せず本体を変えられないので、`get_rig_guide`・`propose_rig` の代わりに `get_module_rig_guide`・
`propose_module_rig` を使う (モジュールのプロジェクトで `propose_rig` を呼ぶと、モジュールのツールへ案内するエラーになる)。

1. `import_psd { "path": "ribbon.ugoeproj", "psd": "ribbon.psd", "base": "character.ugoeproj", "slot": "Hair" }`
   (本体は `.ugoeproj` か `.ugoe`。スロットは本体の `get_model_summary` の `slots`)。衣装のレイヤーはエディタと
   同じく描画順をスロットの範囲に写して取り付け先の下に入る。結果の `module` に本体とスロット、`meshes` は衣装の
   メッシュの数。エディタで作ったモジュールのプロジェクトなら、`open_project` して `get_psd_layers` に `psd` を渡す
2. `get_psd_layers` → `set_layer_labels` (上と同じ)。同じ部品の別の絵 (一度に 1 つだけ見せる袖 A / B・同じ所にかぶる
   帽子とキャップ) を `variant_sets` で組にすると、**その組はポーズになる** (名前の末尾の A / B や、PSD で片方を
   隠した重なる絵で見つからない組も拾う)。結果の `rig.tool` が `propose_module_rig`
3. `get_module_rig_guide` → 値 (`sway_scale` 揺れの大きさ・`sway_length` 振り子の長さ) の説明と範囲・衣装の
   メッシュごとのラベルと位置・従う本体のワープ・揺れるか (`layout`)・組み方の既定 (`options`)・差分の組・衣装の原画
4. 値を選んで `propose_module_rig` に渡す。組み方 (`follow`: `warp` / `mesh`、`colliders`、`poses`) は利用者が
   選ぶもので、省くと既定 (`warp`・衝突体に当てる・ポーズにする):

   ```json
   { "path": "ribbon.ugoeproj",
     "params": { "sway_scale": 1.2, "sway_length": 1.0, "reason": "a light ribbon" } }
   ```

   成功すると提案の ID・理由 (従う本体のワープ・揺れ・当てる衝突体・ポーズ)・差分。パラメータは増えない
5. ユーザーが承認したら `apply_proposal`。`render_png` で本体のパラメータを動かして確かめ、`save_project` で
   保存する (モジュールの部分だけを書く)

同じ流れ (`assets/fixtures/character.psd` を見本の本体のスロット `Headwear` に付け、左右の目を差分の組にする) を
`tests/stdio.rs` の `agent_driven_module_rig_from_psd` が確かめている。

## ツール

| ツール | 種類 | 内容 |
| --- | --- | --- |
| `open_project` | 読む | `.ugoeproj` を開く。`create: true` で新しい空のプロジェクト (保存するまでディスクに書かない) |
| `close_project` | 読む | 閉じる。未保存の変更があれば `discard_changes: true` が要る |
| `get_model_summary` | 読む | パラメータ・パーツ・デフォーマ・メッシュ・グルー・テクスチャ・物理・着せ替えのスロット・モーション・表情と、各格子の軸 (パラメータとキー) |
| `get_element` | 読む | 要素 1 つの JSON (モデルのファイルでの形。キーフォームの中身を含む) |
| `propose_edit` | 読む | `Command` の列を複製したモデルに `Batch` として適用し、検証・`ApplyReport`・変わる要素の ID・コマンドの要約と依存を返す。提案の ID を返す |
| `list_proposals` | 読む | 適用を待っている提案の一覧 (`stale` は提案の後にモデルが変わったもの) |
| `import_psd` | 読む | PSD から新しいプロジェクトを作る (保存するまでディスクに書かない)。PSD を覚える。`base` と `slot` を渡すと着せ替えのモジュールのプロジェクト |
| `get_psd_layers` | 読む | PSD のレイヤーの一覧・今の分類・対応するメッシュと、全体の合成画像・サムネイル (MCP の image)。分類の指示と語彙 |
| `set_layer_labels` | 読む | エージェントが決めた分類と差分の組 (`variant_sets`) を受け取る (モデルは変えない)。メッシュとの対応・差分の組・リグを組めるかを返す |
| `get_rig_guide` | 読む | リグのテンプレートで決められない値の説明・範囲・既定値、顔・目・口の位置、原画 |
| `propose_rig` | 読む | 分類と値からリグ (AngleX/Y/Z・EyeL/ROpen・MouthOpenY) の提案を作る。値が通らなければエラー |
| `get_module_rig_guide` | 読む | モジュールのプロジェクトで、リグのテンプレートで決められない値の説明・範囲・既定値、衣装のメッシュの位置と組み方、差分の組、衣装の原画 |
| `propose_module_rig` | 読む | モジュールのプロジェクトで、分類と値からリグ (追従・揺れ・衝突体・差分の組などのポーズ) の提案を作る。パラメータは足さない。値が通らなければエラー |
| `render_png` | 読む | 今のモデル (未保存の変更を含む) を PNG で描く。`ugoe render` と同じ |
| `apply_proposal` | 編集 | 提案を適用する (1 回の `undo` で戻る)。`exclude` でコマンドの一部を外せる。提案の後にモデルが変わっていたら拒否する |
| `undo` / `redo` | 編集 | 取り消し・やり直し |
| `save_project` | 書く | ディスクに保存する。`bundle` で `.ugoe` にも書き出す |

- 「読む」ツールはドキュメントもディスクも変えない (`readOnlyHint`)。「書く」は `destructiveHint` を付ける。
- どのツールもプロジェクトをパスで指す。開いていないプロジェクトを指すと、ディスクから開く。`--root` があれば、
  パスはその下に限る。
  サーバは開いたプロジェクトをパスごとにメモリに持ち、`save_project` まで書かない。
- 失敗 (検証を通らない提案、未知の ID、引数の誤り) はツールの実行エラー (`isError: true`) で返し、
  エージェントが読んで直せるようにする。ツールや持ち主の不具合で panic したときは、サーバを落とさずに JSON-RPC の
  `-32603` (internal error) で答える (stdio のサーバごと落ちて未保存の変更を失わないように)。
- PSD (`import_psd`・`get_psd_layers` の `psd`) は通常のファイルだけを読む (`/dev/zero` のような装置・FIFO・ディレクトリは
  エラー。終わらない読み込みでメモリを使い果たさないように)。
- 提案はエディタと同じ `ugoe_editor_core::Proposals` に置く。提案は作ったときのモデルの状態に対するもので、
  モデルが変わると適用できない (Undo で同じ状態に戻れば、また使える)。
- 自動リギングのツール (`src/autorig.rs`) の入力の型 (分類の語彙と左右、リグの値) は `ugoe_ai` の JSON Schema
  (`classify::output_schema`・`rig::rig_output_schema`) から作る。画像の大きさの上限は `ClassifyOptions` の既定値。
  PSD と分類はプロジェクトごとにメモリに持つ (プロジェクトには保存しない)。

### 入力の型

`propose_edit` の `commands` の型は `ugoe_editor_core::codegen::command_schema()` (schemars) から起動時に作る
(`src/schema.rs`)。手で写さないので、`Command` を変えても追従する。MCP のツールの入力は最上位が
`type: object` でなければならないので、`Command` のスキーマを `$defs.Command` に移し、`Batch` の中の自己参照
(`{"$ref": "#"}`) を `#/$defs/Command` に書き換える。

`Command` のスキーマは大きい (`tools/list` 全体で約 67 KB) ので、`Command` を受け取るツールは `propose_edit`
だけにし、適用は提案の ID だけを受け取る。

### 描画

`render_png` は `ugoe render` と同じ `ugoe_headless` の関数を、このプロセスの中で呼ぶ (`src/render.rs`)。
描画の規則は CLI と同じで、`ugoe` (ugoe-cli) の実行ファイルは要らない。今のドキュメント (未保存の変更を含む) を
メモリの上で `.ugoe` に書き出して読み直したもの (書き出した `.ugoe` を `ugoe render` で描くのと同じ中身) を描き、
結果は MCP の画像 (`image/png`、base64) で返す。`log` には警告・時間を進めた回数・大きさとアダプタが入る。

長く動くサーバなので、GPU のドライバの不調で落ちないように次の手当てをする。

- GPU は最初の `render_png` で用意し、以後使い回す。破棄しない (WSL2 の dzn はデバイスの破棄時に SIGSEGV を
  起こすことがある)。
- アダプタやデバイスを用意できなければツールのエラーにし、次の呼び出しでもう一度試す。
- 描画中の wgpu のエラー (検証・メモリ不足) はエラーとして返す (`ugoe_headless` のエラースコープ)。
- それでも panic したら受け止めてツールのエラーにし、その GPU を捨てて次の呼び出しで作り直す。
- ドライバの中の SIGSEGV のようにプロセスごと落ちる不調は防げない。そのときは MCP クライアントがサーバを
  起動し直す (未保存の変更は失われる)。ドライバが不安定な環境では `--backend gl` や Mesa の lavapipe
  (`VK_DRIVER_FILES`) を選ぶ (`docs/testing.md` の GPU の節)。

## プロトコル

- 版: `initialize` の握手をする版 (`2025-11-25`・`2025-06-18`・`2025-03-26`・`2024-11-05`) と、要求ごとに
  `_meta` で版を示す版 (`2026-07-28`、`server/discover`) の両方に応じる。
- メソッド: `initialize`・`server/discover`・`ping`・`tools/list`・`tools/call`。通知は受け取って無視する。
- HTTP (`src/http.rs`): [Streamable HTTP](https://modelcontextprotocol.io/specification/draft/basic/transports/streamable-http)
  のエンドポイント 1 つ (`/mcp`) で、要求を 1 つずつ POST で受け、応答は JSON 1 つ (`application/json`) で返す。
  ツールは途中の通知を送らないので SSE は使わない。通知には `202`、GET と DELETE には `405` を返し、
  セッション (`Mcp-Session-Id`) は発行しない (状態はプロセスに 1 つ)。要求ごとに版を示す版では
  `MCP-Protocol-Version`・`Mcp-Method`・`Mcp-Name` (`=?base64?...?=` の形も解く) のヘッダが本文と一致しなければ
  `400` と `HeaderMismatch` (-32020)、知らないメソッドは `404`。握手をする版では `MCP-Protocol-Version` が無ければ
  2025-03-26 とみなし、応じられない版なら `400`。
- 依存: 既存の Rust の MCP SDK は非同期ランタイム (tokio) などを連れてくるが、stdio で要求に順に答えるだけなら
  JSON-RPC の処理は数百行で書けるので、`serde_json` だけで書いた。画像の base64 はワークスペースの `base64` を使う。
  HTTP も、要求に 1 つずつ JSON で答えるだけなので、HTTP のサーバのクレートを入れず std の `TcpListener` の上に
  書いた (接続ごとのスレッド、keep-alive、`Content-Length` と `chunked` の本文、`Expect: 100-continue`)。
  トークンの乱数は `getrandom` (OS の乱数。MIT/Apache-2.0、ワークスペースではすでに推移的に入っている)。
  要求は接続ごとのスレッドで同時に処理する (エディタで確認を待つ適用が、ほかの要求を止めないように)。文書は持ち主の
  錠の中で扱い、描画は GPU を同時に使わないよう `Mutex` で 1 つずつ行う。
- `--editor` の中継 (`src/relay.rs`) はエディタへ HTTP で送る。クライアントはワークスペースの `ureq` (ugoe-ai が使う
  もの) で、要求ごとに新しい接続にし、プロキシを使わない。答えの時間は限らない (エディタが確認の時間切れを決める)。

## テスト

```sh
cargo test -p ugoe-mcp   # サーバを stdio で起動して、握手・ツールの一覧・提案と適用・Undo・保存、PSD からの自動リギングを確かめる
                         # tests/http.rs は --http で起動して、HTTP での握手・ツールの呼び出しと、トークン・Origin・Host の守り・--root の要求を確かめる
                         # --root で外に出るパス (相対・..・絶対・シンボリックリンク) を拒否することは tests/stdio.rs と src/root.rs
                         # tests/editor.rs は偽のエディタ (持ち主) を EditorBridge で待ち受け、--editor の中継でツールを一通り動かす
                         # パスの変換 (WSL と Windows) は src/wsl.rs と src/relay.rs の単体テスト
                         # tests/editor.rs は --mcpb の束も展開し、manifest の検査と、中の ugoe-mcp --editor が偽のエディタにつながることを確かめる
```

Windows 向けのビルドは `cargo xwin clippy --target x86_64-pc-windows-msvc -p ugoe-mcp --tests` で確かめられる。WSL の中なら、
`cargo xwin test --target x86_64-pc-windows-msvc -p ugoe-mcp --no-run` で作った単体テストの .exe を相互運用で動かせる
(`tests/*.rs` の子プロセスを起動するテストは、実行ファイルのパスが Linux のパスになるので動かない)。

```sh
cargo test -p ugoe-mcp --features gpu-tests   # render_png で実際に描く (GPU か lavapipe などが要る。just test-gpu に含まれる)
```

GPU テストは `PATH` を空にしてサーバを起動し、`ugoe` の実行ファイルなしで PNG が返ること、2 回目の描画で GPU を
使い回せること、標準入力を閉じるとサーバが正常に終了すること (終了時に GPU を破棄しない) を確かめる。
GPU を使わないテストでは、GPU に触る前に引数の誤り (未知のパラメータ・色) を返すことを確かめる。

## 関連タスク

- T0508 (`docs/tasks/T0508-mcp-server.md`)
- T0513 (`docs/tasks/T0513-mcp-agent-auto-rig.md`): 自動リギングの判断をエージェントに任せるツール
- T0514 (`docs/tasks/T0514-mcp-http-transport.md`): HTTP のトランスポート
- T0515 (`docs/tasks/T0515-mcp-root-dir.md`): ツールが触れるパスを `--root` の下に限る
- T0516 (`docs/tasks/T0516-mcp-document-host.md`): 文書の持ち主の抽象化・エディタに組み込む API・`--editor` の中継と WSL
- T0643 (`docs/tasks/T0643-editor-claude-desktop-mcpb.md`): Claude Desktop 用の MCP Bundle (`--mcpb`・`src/mcpb.rs`)
