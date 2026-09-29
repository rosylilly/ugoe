# ugoe-mcp

**層:** T4 アプリ (依存してよいクレートは `scripts/check-layers.mjs` を参照)

MCP (Model Context Protocol) サーバ。Claude Code などの外部のエージェントが `.ugoeproj` を開いてモデルを読み、
`ugoe_editor_core::Command` の列で編集を提案・適用できるようにする。設計の判断 (ADR 0006 との関係、
`.ugoeproj` を直接扱うこと) は [docs/ai-integration.md#mcp-サーバ](../../docs/ai-integration.md#mcp-サーバ)。

## 動かし方

stdio のサーバ (1 行 1 メッセージの JSON-RPC 2.0)。MCP クライアントが子プロセスとして起動する。

```sh
cargo build --release -p ugoe-mcp -p ugoe-cli   # target/release/ugoe-mcp と ugoe (描画に使う)
# あるいは PATH に入れる
cargo install --path crates/ugoe-mcp
cargo install --path crates/ugoe-cli
```

| オプション | 内容 |
| --- | --- |
| `--read-only` | 読むだけにする。`apply_proposal`・`undo`・`redo`・`save_project` を出さない (提案と描画はできる) |
| `--ugoe-bin <PATH>` | 描画に使う `ugoe` (ugoe-cli)。省略時は `UGOE_BIN`、このサーバと同じディレクトリの `ugoe`、`PATH` の順に探す |

ログは標準エラーに書く (標準出力は MCP のメッセージだけ)。

### Claude Code に登録する

```sh
# このリポジトリの中だけで使う (プロジェクトのスコープ。.mcp.json に書かれる)
claude mcp add --scope project ugoe -- /path/to/ugoe/target/release/ugoe-mcp
# 自分の環境のどこでも使う
claude mcp add --scope user ugoe -- ugoe-mcp
# 読むだけにする。描画に使う ugoe の場所を渡す
claude mcp add ugoe-read -e UGOE_BIN=/path/to/ugoe -- ugoe-mcp --read-only
```

`claude mcp list` で `ugoe` が `Connected` になっていればよい。Claude Code の中では `/mcp` で状態とツールを見られる。
相対パスはサーバの作業ディレクトリ (Claude Code を起動したディレクトリ) から解決する。

読むだけのツールを確認なしで呼ばせるには、Claude Code の設定 (`.claude/settings.json` など) の `permissions.allow` に
`mcp__ugoe__open_project`・`mcp__ugoe__get_model_summary`・`mcp__ugoe__get_element`・`mcp__ugoe__propose_edit`・
`mcp__ugoe__render_png` を足す。`mcp__ugoe__apply_proposal` と `mcp__ugoe__save_project` は足さず、呼ぶたびに確認させる
(提案の承認にあたる。[docs/ai-integration.md#提案と適用-adr-0006-との関係](../../docs/ai-integration.md#提案と適用-adr-0006-との関係))。

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

   結果は提案の ID (`p1`)・`report` (警告)・`changes` (`{"parameters": {"added": ["AngleX"]}, "meshes": {"changed": ["Face"]}}`)。
   モデルはまだ変わらない。
3. ユーザーが差分を見て承認すると `apply_proposal { "proposal": "p1" }` (Claude Code は呼ぶ前に許可を求める)
4. キーのセルの形を変える (`set_mesh_keyform` など) 提案を同じように出し、`render_png { "params": { "AngleX": 30 } }` で見た目を確かめる
5. `save_project` で保存する (`bundle` を渡すと `.ugoe` にも書き出す)

同じ流れを `tests/stdio.rs` が、サーバを子プロセスとして起動して確かめている。

## ツール

| ツール | 種類 | 内容 |
| --- | --- | --- |
| `open_project` | 読む | `.ugoeproj` を開く。`create: true` で新しい空のプロジェクト (保存するまでディスクに書かない) |
| `close_project` | 読む | 閉じる。未保存の変更があれば `discard_changes: true` が要る |
| `get_model_summary` | 読む | パラメータ・パーツ・デフォーマ・メッシュ・グルー・テクスチャ・物理・着せ替えのスロット・モーション・表情と、各格子の軸 (パラメータとキー) |
| `get_element` | 読む | 要素 1 つの JSON (モデルのファイルでの形。キーフォームの中身を含む) |
| `propose_edit` | 読む | `Command` の列を複製したモデルに `Batch` として適用し、検証・`ApplyReport`・変わる要素の ID を返す。提案の ID を返す |
| `render_png` | 読む | 今のモデル (未保存の変更を含む) を PNG で描く。`ugoe render` と同じ |
| `apply_proposal` | 編集 | 提案を適用する (1 回の `undo` で戻る)。提案の後にモデルが変わっていたら拒否する |
| `undo` / `redo` | 編集 | 取り消し・やり直し |
| `save_project` | 書く | ディスクに保存する。`bundle` で `.ugoe` にも書き出す |

- 「読む」ツールはドキュメントもディスクも変えない (`readOnlyHint`)。「書く」は `destructiveHint` を付ける。
- どのツールもプロジェクトをパスで指す。開いていないプロジェクトを指すと、ディスクから開く。
  サーバは開いたプロジェクトをパスごとにメモリに持ち、`save_project` まで書かない。
- 失敗 (検証を通らない提案、未知の ID、引数の誤り) はツールの実行エラー (`isError: true`) で返し、
  エージェントが読んで直せるようにする。

### 入力の型

`propose_edit` の `commands` の型は `ugoe_editor_core::codegen::command_schema()` (schemars) から起動時に作る
(`src/schema.rs`)。手で写さないので、`Command` を変えても追従する。MCP のツールの入力は最上位が
`type: object` でなければならないので、`Command` のスキーマを `$defs.Command` に移し、`Batch` の中の自己参照
(`{"$ref": "#"}`) を `#/$defs/Command` に書き換える。

`Command` のスキーマは大きい (`tools/list` 全体で約 67 KB) ので、`Command` を受け取るツールは `propose_edit`
だけにし、適用は提案の ID だけを受け取る。

### 描画

`render_png` は今のドキュメントを一時的な `.ugoe` に書き出し、`ugoe render` を子プロセスとして呼ぶ (`src/render.rs`)。
描画の規則を CLI と完全に同じにし、GPU のデバイスをサーバのプロセスに持たないため。結果は MCP の画像
(`image/png`、base64) で返す。

- `ugoe` は `--ugoe-bin`、`UGOE_BIN`、このサーバと同じディレクトリ、`PATH` の順に探す。同じ checkout から
  一緒にビルド (`cargo build -p ugoe-cli -p ugoe-mcp`) すれば隣に置かれ、版がそろう。
- 見つからなければ、探した場所と直し方をツールのエラーで返す。`ugoe --version` がサーバの版と違えば、結果の
  `log` に警告を付ける。

## プロトコル

- 版: `initialize` の握手をする版 (`2025-11-25`・`2025-06-18`・`2025-03-26`・`2024-11-05`) と、要求ごとに
  `_meta` で版を示す版 (`2026-07-28`、`server/discover`) の両方に応じる。
- メソッド: `initialize`・`server/discover`・`ping`・`tools/list`・`tools/call`。通知は受け取って無視する。
- 依存: 既存の Rust の MCP SDK は非同期ランタイム (tokio) などを連れてくるが、stdio で要求に順に答えるだけなら
  JSON-RPC の処理は数百行で書けるので、`serde_json` だけで書いた。画像の base64 はワークスペースの `base64` を使う。

## テスト

```sh
cargo test -p ugoe-mcp   # サーバを stdio で起動して、握手・ツールの一覧・提案と適用・Undo・保存を確かめる
```

`render_png` の実際の描画は `ugoe render` のテスト (`crates/ugoe-cli`) が確かめる。ここでは CLI が見つからないときの
エラーだけを確かめる。

## 関連タスク

- T0508 (`docs/tasks/T0508-mcp-server.md`)
