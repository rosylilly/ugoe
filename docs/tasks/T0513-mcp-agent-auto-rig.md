---
id: T0513
title: MCP: 自動リギングの判断を外部のエージェントに任せるツール
status: done
milestone: M6
crates: [ugoe-mcp, ugoe-ai, docs]
depends_on: [T0508, T0503, T0504]
assignee: claude-worker-T0513
---

# T0513: MCP: 自動リギングの判断を外部のエージェントに任せるツール

## 目的

ADR 0022 の「外部のエージェントに判断を任せる経路」を作る。ugoe は API を呼ばず、MCP クライアント側のエージェント (Claude Code・OpenCode・Codex CLI など) が、画像を見て分類し、パラメータを選ぶ。

## 受け入れ条件

- [x] PSD からプロジェクトを作る (または開く) → レイヤーの一覧と画像 (合成画像・レイヤーの縮小画像。MCP の image コンテンツ) を返す → エージェントが分類の結果 (T0503 の語彙) を渡す → 提案 (T0504 の `propose_rig` など) を作って検証の結果と差分を返す、までのツール
- [x] エージェントが `RigParams` など判断の値を渡せる。検証に通らなければ、エラーをツールの結果として返し、エージェントが直して呼び直せる
- [x] 分類の語彙・出力の形は `ugoe_ai` の JSON Schema から作り、手で写さない
- [x] 提案の適用は既存の `apply_proposal` と同じく、利用者の確認 (MCP クライアントの許可) を通す (ADR 0006)
- [x] stdio でサーバを起動し、JSON-RPC を流すテストで一通りの流れを確かめる。Claude Code・OpenCode から使う手順を `crates/ugoe-mcp/README.md` に書く
- [x] `just ci` が通る

## 参考

- `docs/adr/0022-multiple-ai-providers-and-agents.md`
- `docs/tasks/T0508-mcp-server.md`・`docs/tasks/T0504-auto-rig.md` のメモ

## メモ

- やったこと
  - `ugoe-mcp` に `import_psd`・`get_psd_layers`・`set_layer_labels`・`get_rig_guide`・`propose_rig`・`list_proposals` を足した
    (`src/autorig.rs`。トランスポートに依らない `Tools` の中。T0514 の HTTP はこれをそのまま使える)。
    - `get_psd_layers` はレイヤーの一覧 (番号・名前・外接矩形・対応するメッシュ・今の分類) と、全体の合成画像 (最初のページ)・
      サムネイルを MCP の image で返す。各サムネイルの直前に `layer N: 名前` の文を置いて対応を示す。最初のページに分類の指示と語彙。
      名前の規則の分類 (`classify_by_name`) を既定の案として持ち、エージェントは直すレイヤーだけを渡せばよい (`set_by`)。
    - 画像の上限は `ClassifyOptions` の既定 (1024・256 画素、1 回 32 枚)。Claude Code は画像も含めてツールの結果を既定 25,000 トークンに
      制限するので、既定では 8 枚ずつにした (`count` で 32 まで)。胸から上の PSD の最初の 24 枚 + 合成画像は約 1.6 MB だった。
    - 値が範囲の外・語彙にないラベル・番号の誤り・検証を通らないリグは、ツールのエラー (`isError`) で返し、エージェントが直して呼び直す。
  - ugoe-ai (lib.rs は触っていない): `classify::{layers, LayerInfo, layer_thumbnail_png, classification_instructions, apply_answer}`・
    `Label::description`・`rig::params_from_answer` を足した。`classify_layers` と `propose_rig_with_ai` はこの関数を使うように直した
    (動作は同じ)。MCP の入力の型は `output_schema`・`rig_output_schema` から作り、指示の文は API に送るシステムプロンプトと同じ。
  - 提案の置き場を MCP 独自の表から `ugoe_editor_core::Proposals` (T0637) に替えた。提案の結果にコマンドの要約と依存 (`requires`) を付け、
    `apply_proposal` に `exclude` を足した。挙動の変化: 適用・Undo の後も提案は捨てず stale になる (Undo で戻れば使える)。
    適用済みの提案をもう一度適用したときのエラーの文が「not found」から「the model changed ...」に変わった (テストを直した)。
  - 文書: `docs/ai-integration.md#エージェントに任せる自動リギング`、同じ文書の提案と適用の節、`crates/ugoe-mcp/README.md` (流れの例・
    Claude Code の許可の設定・OpenCode の登録)、サーバの `instructions`。
- 確かめたこと
  - `tests/stdio.rs` の `agent_driven_auto_rig_from_psd`: サーバを stdio で起動し、胸から上の実際の PSD で import → レイヤーと画像
    (PNG の大きさの上限) → 誤った分類がエラーで何も変えない → 分類を渡す → ガイド → 範囲の外の値・形の違う値がエラー → 直した値で提案 →
    モデルは変わらない → `exclude` の誤り → 適用 → 保存 (`validate` が空、パラメータ 6 つ) → 開き直すと PSD を渡し直す。
  - `just ci`・`cargo test -p ugoe-mcp --features gpu-tests` が通る。手元で MCP 経由で適用したリグを `render_png`
    (AngleX 30・EyeLOpen 0・MouthOpenY 1) で描き、振り向き・ウインク・開いた口を目で見た (scratchpad の `T0513/rig-angle30.png`)。
- 確かめていないこと
  - Claude Code・OpenCode・Codex CLI から実際につなぐこと (ユーザーの設定を変えないため)。Claude Code が画像を何トークンと数えるか。
    OpenCode の MCP のツールの許可の書き方 (`ugoe_apply_proposal` の名前) は文書で確かめきれず、README にそう書いた。
- 範囲外の変更: ルートの `Cargo.lock` (ugoe-mcp が ugoe-ai・ugoe-psd に依存したため。外部の依存は増えていない)。
- 後続の候補
  - 分類をプロジェクトに保存する (開き直すたびに PSD を渡し直さなくて済む。エディタと共有できる)。
  - サムネイルを 1 枚の一覧画像 (コンタクトシート) にまとめ、画像の枚数を減らす (クライアントが画像ごとに数える場合に効く)。
  - 深度推定器を MCP から使う・`propose_rig` に `RigOptions` (頭のワープの分割数) を出す。法線マップ (T0506) の提案のツール。
  - MCP の提案を実行中のエディタの提案の一覧に出す。
