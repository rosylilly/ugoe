# ugoe ドキュメント

| 文書 | 内容 |
| --- | --- |
| [vision.md](vision.md) | 目標・非目標・機能互換の範囲 |
| [architecture.md](architecture.md) | クレート構成・レイヤー・データフロー・依存ルール |
| [glossary.md](glossary.md) | 用語集 (Live2D 用語との対応) |
| [spec/model.md](spec/model.md) | モデルのデータ構造 |
| [spec/evaluation.md](spec/evaluation.md) | キーフォーム補間とデフォーマ変形の数式 |
| [spec/rendering.md](spec/rendering.md) | 描画・合成・マスク |
| [spec/physics.md](spec/physics.md) | 物理演算 |
| [spec/motion.md](spec/motion.md) | モーション・表情・常時エフェクト |
| [spec/file-formats.md](spec/file-formats.md) | ファイル形式とバージョニング |
| [editor.md](editor.md) | エディタの設計 |
| [ai-integration.md](ai-integration.md) | AI 連携の設計 |
| [portability.md](portability.md) | ゲームエンジン・プラットフォームへの移植 |
| [testing.md](testing.md) | テスト方針 |
| [roadmap.md](roadmap.md) | マイルストーン |
| [workflow/multi-agent.md](workflow/multi-agent.md) | マルチエージェントでの作業手順 |
| [tasks/](tasks/README.md) | タスクボード |
| [adr/](adr/README.md) | 設計判断の記録 |

## 文書の書き方

- **仕様 (`spec/`) は「何をするか」を書く。** 実装の詳細ではなく、入力と出力の関係を書く。ランタイムを別言語で再実装する人が、コードを読まずに同じ結果を出せることを目標にする。
- **ADR は「なぜそう決めたか」を書く。** 一度 `Accepted` になった ADR は書き換えない。覆すときは新しい ADR を書く。
- 未決の論点は `> 未決:` で始まる引用ブロックで書く。決まったら消すか ADR にする。
