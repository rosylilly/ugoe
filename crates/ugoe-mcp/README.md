# ugoe-mcp

**層:** T4 アプリ (依存してよいクレートは `scripts/check-layers.mjs` を参照)

MCP (Model Context Protocol) サーバ。Claude Code などの外部のエージェントが `.ugoeproj` を開いてモデルを読み、
`ugoe_editor_core::Command` の列で編集を提案・適用できるようにする。PSD からの自動リギングでは、画像を見て
レイヤーを分類し、リグの値を決める判断をエージェントに任せる (ugoe は API を呼ばない。ADR 0022)。設計の判断
(ADR 0006 との関係、`.ugoeproj` を直接扱うこと) は [docs/ai-integration.md#mcp-サーバ](../../docs/ai-integration.md#mcp-サーバ)。

## 動かし方

stdio のサーバ (1 行 1 メッセージの JSON-RPC 2.0)。MCP クライアントが子プロセスとして起動する。

```sh
cargo build --release -p ugoe-mcp   # target/release/ugoe-mcp
# あるいは PATH に入れる
cargo install --path crates/ugoe-mcp
```

| オプション | 内容 |
| --- | --- |
| `--read-only` | 読むだけにする。`apply_proposal`・`undo`・`redo`・`save_project` を出さない (提案と描画はできる) |
| `--backend <NAME>` | 描画に使う wgpu のバックエンド (`vulkan` `metal` `dx12` `gl`、カンマ区切り)。省略時は `WGPU_BACKEND`、それも無ければ wgpu の既定 |

ログは標準エラーに書く (標準出力は MCP のメッセージだけ)。

### Claude Code に登録する

```sh
# このリポジトリの中だけで使う (プロジェクトのスコープ。.mcp.json に書かれる)
claude mcp add --scope project ugoe -- /path/to/ugoe/target/release/ugoe-mcp
# 自分の環境のどこでも使う
claude mcp add --scope user ugoe -- ugoe-mcp
# 読むだけにする。描画のバックエンドを GL に固定する
claude mcp add ugoe-read -- ugoe-mcp --read-only --backend gl
```

`claude mcp list` で `ugoe` が `Connected` になっていればよい。Claude Code の中では `/mcp` で状態とツールを見られる。
相対パスはサーバの作業ディレクトリ (Claude Code を起動したディレクトリ) から解決する。

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
      "mcp__ugoe__get_rig_guide", "mcp__ugoe__propose_rig"
    ]
  }
}
```

`get_psd_layers` と `get_rig_guide` は画像を返す。Claude Code は MCP のツールの結果の大きさに上限 (既定 25,000 トークン。
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
   リグを組めるか (`rig.ready`。顔がないと組めない)
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

## ツール

| ツール | 種類 | 内容 |
| --- | --- | --- |
| `open_project` | 読む | `.ugoeproj` を開く。`create: true` で新しい空のプロジェクト (保存するまでディスクに書かない) |
| `close_project` | 読む | 閉じる。未保存の変更があれば `discard_changes: true` が要る |
| `get_model_summary` | 読む | パラメータ・パーツ・デフォーマ・メッシュ・グルー・テクスチャ・物理・着せ替えのスロット・モーション・表情と、各格子の軸 (パラメータとキー) |
| `get_element` | 読む | 要素 1 つの JSON (モデルのファイルでの形。キーフォームの中身を含む) |
| `propose_edit` | 読む | `Command` の列を複製したモデルに `Batch` として適用し、検証・`ApplyReport`・変わる要素の ID・コマンドの要約と依存を返す。提案の ID を返す |
| `list_proposals` | 読む | 適用を待っている提案の一覧 (`stale` は提案の後にモデルが変わったもの) |
| `import_psd` | 読む | PSD から新しいプロジェクトを作る (保存するまでディスクに書かない)。PSD を覚える |
| `get_psd_layers` | 読む | PSD のレイヤーの一覧・今の分類・対応するメッシュと、全体の合成画像・サムネイル (MCP の image)。分類の指示と語彙 |
| `set_layer_labels` | 読む | エージェントが決めた分類を受け取る (モデルは変えない)。メッシュとの対応とリグを組めるかを返す |
| `get_rig_guide` | 読む | リグのテンプレートで決められない値の説明・範囲・既定値、顔・目・口の位置、原画 |
| `propose_rig` | 読む | 分類と値からリグ (AngleX/Y/Z・EyeL/ROpen・MouthOpenY) の提案を作る。値が通らなければエラー |
| `render_png` | 読む | 今のモデル (未保存の変更を含む) を PNG で描く。`ugoe render` と同じ |
| `apply_proposal` | 編集 | 提案を適用する (1 回の `undo` で戻る)。`exclude` でコマンドの一部を外せる。提案の後にモデルが変わっていたら拒否する |
| `undo` / `redo` | 編集 | 取り消し・やり直し |
| `save_project` | 書く | ディスクに保存する。`bundle` で `.ugoe` にも書き出す |

- 「読む」ツールはドキュメントもディスクも変えない (`readOnlyHint`)。「書く」は `destructiveHint` を付ける。
- どのツールもプロジェクトをパスで指す。開いていないプロジェクトを指すと、ディスクから開く。
  サーバは開いたプロジェクトをパスごとにメモリに持ち、`save_project` まで書かない。
- 失敗 (検証を通らない提案、未知の ID、引数の誤り) はツールの実行エラー (`isError: true`) で返し、
  エージェントが読んで直せるようにする。
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
- 依存: 既存の Rust の MCP SDK は非同期ランタイム (tokio) などを連れてくるが、stdio で要求に順に答えるだけなら
  JSON-RPC の処理は数百行で書けるので、`serde_json` だけで書いた。画像の base64 はワークスペースの `base64` を使う。

## テスト

```sh
cargo test -p ugoe-mcp   # サーバを stdio で起動して、握手・ツールの一覧・提案と適用・Undo・保存、PSD からの自動リギングを確かめる
```

```sh
cargo test -p ugoe-mcp --features gpu-tests   # render_png で実際に描く (GPU か lavapipe などが要る。just test-gpu に含まれる)
```

GPU テストは `PATH` を空にしてサーバを起動し、`ugoe` の実行ファイルなしで PNG が返ること、2 回目の描画で GPU を
使い回せること、標準入力を閉じるとサーバが正常に終了すること (終了時に GPU を破棄しない) を確かめる。
GPU を使わないテストでは、GPU に触る前に引数の誤り (未知のパラメータ・色) を返すことを確かめる。

## 関連タスク

- T0508 (`docs/tasks/T0508-mcp-server.md`)
- T0513 (`docs/tasks/T0513-mcp-agent-auto-rig.md`): 自動リギングの判断をエージェントに任せるツール
