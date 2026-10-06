---
id: T0969
title: "マニュアル: AI と MCP の画像 (ai-proposal・ai-revision・ai-consent・ai-settings など・mcp-*) を偽の AI のプロバイダで撮り直す"
status: done
milestone: M10
crates: [tests, docs]
depends_on: [T0050, T0968]
assignee: claude-worker-T0969
---

# T0969: マニュアル: AI と MCP の画像 (ai-proposal・ai-revision・ai-consent・ai-settings など・mcp-*) を偽の AI のプロバイダで撮り直す

## 目的

AI のタブの画像は T0676・T1643 より前のまま (提案のカードに「Adopting changes:」の行も今のボタンの並びも無い。T1643)。
ai-settings.png は T0919 で撮り直したが、鍵の保管庫が無くて API キーの欄が写っていない。MCP の画像も撮っていない (T0919)。
`just e2e --manual` で、偽の AI のプロバイダと鍵の保管庫 (T0050) を使って撮り直す。

## 受け入れ条件

- [x] `just e2e --manual` の筋書きが、ネットワークに出ない偽の AI のプロバイダ (決まった提案・改訂・法線マップを返す) でエディタを動かす。
      偽物をアプリに入れる口が要るなら、e2e のビルドだけで有効になる形にし、その理由をメモに書いた
- [x] `ai-proposal`・`ai-consent`・`ai-instructions`・`ai-revision`・`ai-normal-maps`・`ai-settings` (API キーの欄が写る) を撮った
- [x] `mcp-*` の画像を撮った (MCP サーバと承認の流れ。撮れないものは理由と撮り方をメモと docs/manual-check.md に書いた)
- [x] docs/manual/ai.md・mcp.md の文と画像の説明が合っている
- [x] tests/e2e/README.md#マニュアルの画像 の一覧を直した
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T0919 のメモ (やらなかったこと・起票の候補)、T1643 のメモ (ai-revision.png)、T0050 (鍵の保管庫)
- tests/e2e/README.md#マニュアルの画像、docs/manual/ai.md、docs/manual/mcp.md

## メモ

- T0046 で起票した (元の候補: T0919 (AI と MCP の画像)・T1643 (ai-revision.png の撮り直し))。
- 同じ `editor.manual.mjs` を触るので T0968 の後にした。

### T0969 の作業 (やったこと)

- `just e2e --manual` の筋書き (`specs/editor.manual.mjs`) の最後に AI と MCP の手順を足し、`ai-proposal`・`ai-consent`・`ai-instructions`・
  `ai-revision`・`ai-normal-maps`・`mcp-connect`・`mcp-approval`・`mcp-claude-desktop` を撮り直した (`ai-settings` は前の画像と同じだった)。
  `character.psd` から「New from PSD…」で作って保存したプロジェクトで撮る。法線マップは `ugoe sample avatar` (法線マップの無いもの)。
  右の列は区切りをキーで動かして広げる (アプリの既定は変えない)。
- **偽物をアプリに入れる口は作っていない (アプリのコードは変えていない)。** AI は T0029 の偽の AI のプロバイダ (`lib/fake-ai.mjs`。
  ループバックの OpenAI 互換のサーバ) に設定の画面から向ける。注文 (`<user_instructions>`) を読んで、リグの値では変えられない文に
  `Not applied:` を答えるようにした (`ai-revision`)。答えの理由の決まった文を、偽の答えの決め方そのもの
  ("Each value is the middle of its allowed range.") にした (マニュアルに写るため)。
- 送る前の確認 (`ai-consent`) は、Claude を選んで偽のキーを使い捨ての鍵の保管庫 (T0050) に置き、名前で分類してから「Propose rig」で出し、
  撮って「Cancel」で閉じる (何も送らない)。鍵の保管庫が無ければ、この手順 (と `ai-instructions`) は飛ばす。
- MCP は偽のエージェント (`lib/fake-agent.mjs`。エディタのブリッジに Streamable HTTP でつなぐ小さなクライアント) で撮る。
  `clientInfo` で `claude-code`・`claude-ai` と名乗り、`get_psd_layers`・`set_layer_labels`・`propose_rig`・`apply_proposal` で
  適用の確認を出す。待ち受けを入れ直して、つないだエージェントの一覧を画像ごとに 1 つにする。
- `run.mjs`: エディタの隣に `ugoe-mcp` を `cargo build -p ugoe-mcp` で作って写す (「Connect agents」の登録のコマンドと `.mcpb` の保存に要る。
  mbx の成果物は読むだけの権限なので、消してから写す)。`--manual` ではアプリに `WSL_DISTRO_NAME` を渡さない (WSL の案内を写さない)。
- `run.mjs`: エディタの外への HTTP(S) を見張る止める口 (`ALL_PROXY`・`HTTPS_PROXY`・`HTTP_PROXY` を手元のプロキシの形の口に向け、
  ループバックは `NO_PROXY` で直に通す)。外へ出ようとしたら止めて、行き先を出してそのアプリを失敗にする。送り先を
  `https://example.invalid/v1` にして「Test connection」で送らせる一時的な手順で、`CONNECT example.invalid:443` が止まって失敗に
  なることを確かめた (コミットしていない。外には何も出ない)。
- `lib/webdriver.mjs` の `Keys` に `ArrowLeft` を足した。
- (レビューの指摘) 画面に出るパスに流した機械の場所 (ユーザー名・worktree・mbx の置き場のハッシュ) が写っていたので、`--manual` では
  run.mjs が作業の場所を `/tmp/ugoe-manual/work/`、エディタと `ugoe-mcp` を `/tmp/ugoe-manual/bin/` に写して起動するようにした
  (同時に流せないよう `/tmp/ugoe-manual/lock` で止める)。mcp-connect・mcp-claude-desktop・mcp-approval を撮り直し、パスが
  `/tmp/ugoe-manual/…` だけであることを画像で確かめた。T0968 の `module-new` も一時ディレクトリ (`/tmp/ugoe-manual-<乱数>`) を
  写していたので、見本の本体をそのまま選ぶようにして撮り直した。ai-* の画像にはパスが写っていない (PSD のファイル名だけ)。
- 文書: docs/manual/ai.md (画像の説明。リグの提案は呼吸と髪揺れの物理も作ること、「制限」の古い文 (分類を画面で直せない・分類が残らない)
  を今の形に)、docs/manual/mcp.md (画像の説明)、tests/e2e/README.md (「外に出さない」「偽のエージェント」「マニュアルの画像」・
  偽の AI のプロバイダの注文)、docs/manual-check.md (A01〜A03 に、偽物で撮った画像と本物の画面を見比べることを足した)。
- `just e2e --only editor` と `just e2e --manual --only editor` (と `--update-images`) が通った。`just ci` が通った。

### やらなかったこと・確かめていないこと

- ai-* と mcp-* の外の画像は、撮り直した結果をコミットしていない (動きの途中の差だけで、このタスクの範囲の外)。
- 本物の Claude Code・Claude Desktop が名乗る名前 (`claude-code`・`claude-ai`) は前の画像に合わせただけで、本物では確かめていない
  (docs/manual-check.md の A02・A03 に書いた)。
- 「Install in Claude Desktop…」(OS に開かせる) は撮っていない (Linux の e2e では Claude Desktop が無い)。「Save .mcpb…」で撮った。

### 起票の候補

- エディタ: モジュールのプロジェクトを本体の問題で開けなかったときの知らせ (`ModuleProblemBar`) が、「New from PSD…」で別のプロジェクトを
  作った後も残る (`fileMenu.svelte.ts` の `newFromPsd` は `this.problem` を消さない。`open`・`openSample` は消す)。e2e のマニュアルの筋書きは
  知らせを閉じてから進める。
- 偽の AI のプロバイダが、注文に従った値を返す (今は値を変えずに理由の文だけを変えるので、作り直した提案の値が前と同じ)。
