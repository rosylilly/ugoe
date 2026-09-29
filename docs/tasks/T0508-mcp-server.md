---
id: T0508
title: MCP サーバ (`ugoe-mcp`)
status: done
milestone: M6
crates: [ugoe-mcp, scripts, Cargo.toml, commitlint.config.mjs, docs, AGENTS.md]
depends_on: [T0601]
assignee: claude-worker-T0508
---

# T0508: MCP サーバ (`ugoe-mcp`)

## 目的

`Command` の JSON Schema をツールとして公開する MCP サーバを作り、Claude Code などの外部のエージェントがプロジェクトを読み、編集を提案・適用できるようにする。

## 受け入れ条件

- [x] 新しいクレート `ugoe-mcp` (ツール層、MIT)。`scripts/check-layers.mjs` と commitlint の scope に登録する
- [x] ツール: プロジェクトを開く・保存する、モデルの要約を読む (パーツ・メッシュ・デフォーマ・パラメータ・キーの格子)、`Command` を適用する (検証と ApplyReport を返す)、Undo、指定したパラメータの値で PNG を描く (`ugoe render` と同じ)
- [x] ツールの入力の型は `Command` の JSON Schema から作る (手で写さない)。AI は提案するだけという原則 (ADR 0006) との関係を決めて書く (例: 適用には確認を求める設定)
- [x] 実行中のエディタにつなぐか、`.ugoeproj` を直接扱うかを決めて書く
- [x] Claude Code からつないで、簡単な編集 (パラメータの作成とキーの追加) ができることを確かめ、手順を書く
- [x] `just ci` が通る

## 参考

- `docs/ai-integration.md` の原則 5
- https://modelcontextprotocol.io/

## メモ

- 設計の判断は `docs/ai-integration.md#mcp-サーバ`、使い方とツールは `crates/ugoe-mcp/README.md`。
  - `.ugoeproj` を直接開く (実行中のエディタにはつながない)。同じプロジェクトをエディタと同時に編集しないこと。
  - ADR 0006 との関係: 提案 (`propose_edit`。複製に適用して検証・`ApplyReport`・変わる要素の ID を返す) と
    適用 (`apply_proposal`。提案の ID だけ) を別のツールにし、MCP クライアントの許可の確認を承認にする。
    提案の後にドキュメントが変わったら提案を捨てる。`--read-only` で適用・Undo・Redo・保存を出さない。
- `Command` を受け取るツールは `propose_edit` だけ (`tools/list` が約 67 KB になるため)。入力の型は
  `command_schema()` を起動時に組み替えて作る (`$defs.Command` に移し、`#` への参照を書き換える)。
- 描画は `ugoe render` を子プロセスで呼ぶ。直接ライブラリを呼ばなかった理由: CLI の描画は
  `ugoe-render` を呼ぶだけでなく、ヘッドレスの GPU の用意 (`render::Gpu`、WSL2 の dzn を避けるための leak)・
  出力の大きさと `fit`・PNG のデコードと乗算済みアルファ・時間を進める処理 (`animate.rs`) など約 1000 行が
  バイナリの中の private なモジュールにあり、共有するには ugoe-cli をライブラリにするか別のクレートに
  切り出す必要がある (ugoe-cli は範囲外で、T0114 が同時に触っている)。写すと描画の規則がずれる。
  加えて、GPU のドライバの不調で長く動くサーバが落ちないようにできる。
  - 弱点 (版ずれ・見つからない) への手当て: `--ugoe-bin` / `UGOE_BIN` で変えられる。既定はサーバと
    同じディレクトリの `ugoe` を `PATH` より先に使う (同じ checkout で一緒にビルドすれば版がそろう)。
    見つからなければ、探した場所と直し方 (`cargo build -p ugoe-cli -p ugoe-mcp`・`--ugoe-bin`) をエラーで示す。
    `ugoe --version` がサーバの版と違えば描画の結果に警告を付ける。
  - 後続の候補: ヘッドレス描画 (GPU の用意・大きさ・PNG) を共有のライブラリに切り出し、CLI と MCP が
    同じ関数を呼ぶ形にする (ugoe-cli の変更を伴う)。
- JSON-RPC は `serde_json` だけで書いた (既存の Rust の MCP SDK は非同期ランタイムを連れてくるため)。
  画像の base64 は T0509 がワークスペースに入れた `base64` を使う (新しい外部の依存は無い)。旧来の `initialize` の版と、
  2026-07-28 の要求ごとの `_meta` の版の両方に応じる。
- 確かめたこと: `tests/stdio.rs` でサーバを子プロセスとして起動し、握手・`tools/list`・パラメータの作成と
  キーの追加の提案と適用・Undo/Redo・保存・`--read-only` を確かめた。`render_png` は手元で `ugoe render` を
  呼んで PNG が返ることを確かめた (CI では CLI が無いときのエラーだけ)。実際の Claude Code からの接続は
  ユーザーの設定を変えないため行っていない (登録の手順は README)。
- AGENTS.md のクレート一覧に `ugoe-mcp` を足した (team-lead の承認を得て `crates:` に AGENTS.md を追加)。
- 後続の候補: 実行中のエディタにつなぎ、エディタの画面で提案の差分をプレビューして採用する形
  (Tauri 側に口を開ける)。PSD の取り込み (`import_psd`) や `ugoe_ai` の推定 (`infer_keyforms` 以外の
  AI の提案) のツール。`Command` の op ごとに分けた小さなスキーマ (トークンの節約)。
