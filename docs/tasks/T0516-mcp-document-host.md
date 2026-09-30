---
id: T0516
title: MCP: 文書の持ち主の抽象化と、エディタへの中継 (--editor)
status: in-progress
milestone: M6
crates: [ugoe-mcp, docs]
depends_on: [T0515]
assignee: claude-worker-T0516
---

# T0516: MCP: 文書の持ち主の抽象化と、エディタへの中継 (--editor)

## 目的

ADR 0023 の 1・3・4 の `ugoe-mcp` 側。ツールをエディタの中でも使えるようにし、`ugoe-mcp --editor` で stdio のクライアントを実行中のエディタへ中継する。

## 受け入れ条件

- [ ] ツールの文書の持ち主を抽象化する (例: `DocumentHost` トレイト。ファイルを自分で開く持ち主 = 今の振る舞い、外から借りる持ち主)。適用 (`apply_proposal`・`undo`・`redo`・`save_project`)・プロジェクトの開閉は持ち主が許すかを決められる (エディタでは確認を待つ・拒否する)。ツールの形と結果は変えない。既存のテストが通る
- [ ] エディタが組み込める API: 外から渡した持ち主でツールを動かし、HTTP (T0514) で待ち受ける関数 (ループバック・トークン必須・`--root` 相当の制限の扱いを書く)。発見用のファイル (ポート・トークン・PID・版) の形と置き場所 (OS ごと) を決め、読み書きの関数を置く
- [ ] `ugoe-mcp --editor`: 発見用のファイルを読んで実行中のエディタにつなぎ、stdio のメッセージを中継する。エディタが無い・止まった・版が合わないときの分かりやすいエラー
- [ ] WSL: Linux 版の `--editor` は Windows 側の発見用のファイルを探せる (`/mnt/c` の下。Windows の利用者のディレクトリの見つけ方を書く)。ミラーでないネットワークでつながらないときの案内。WSL のパスと Windows のパスの相互の変換 (中継の中で。`/mnt/c/...` ↔ `C:\...`、`/home/...` ↔ `\\wsl.localhost\<distro>\...`) とテスト
- [ ] 偽のエディタ (テストの中で持ち主を実装し HTTP で待ち受ける) につないで、中継で一通りのツールが動くテスト
- [ ] README と `docs/ai-integration.md` を更新し、`just ci` が通る

## 参考

- `docs/adr/0023-editor-mcp-bridge.md`
- `docs/tasks/T0514-mcp-http-transport.md`・`docs/tasks/T0515-mcp-root-dir.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
