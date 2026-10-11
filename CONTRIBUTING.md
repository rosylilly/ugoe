# ugoe への貢献

> **English:** The development documents of ugoe (this file, [AGENTS.md](AGENTS.md) and [docs/](docs/README.md)) are written in Japanese.
> Issues and pull requests in English are welcome. Commit messages must be in English.
> The public specification ([spec/](spec/README.md)) is in English. To report a vulnerability, see [SECURITY.md](SECURITY.md).

ugoe に興味を持ってくださってありがとうございます。この文書は、人の貢献者が最初に読む短い案内です。
規約の正は [AGENTS.md](AGENTS.md) と [docs/](docs/README.md) にあり、ここでは要点だけを書いてリンクします。食い違ったら AGENTS.md が正です。

> ugoe は開発中 (未リリース) です。1.0 より前は API とファイル形式が予告なく変わります。

## 始める前に

- **行動規範:** 参加するすべての人は [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) に従います。
- **脆弱性:** 公開の Issue に書かず、[SECURITY.md](SECURITY.md) の手順で報告してください。
- **不具合と要望:** Issue のテンプレートから書いてください。大きな変更は、PR の前に Issue で相談してください。
- **作業の単位:** 作業は [docs/tasks/](docs/tasks/README.md) のタスク単位で行います (1 タスク = 1 ブランチ = 1 PR)。
  対応するタスクが無い変更は、先に Issue で相談してください (メンテナがタスクを起票します)。誤字の修正などの小さな変更は、タスクが無くてもかまいません。
  手順の詳細は [docs/workflow/multi-agent.md](docs/workflow/multi-agent.md) にあります (人にも同じ手順を使います)。

## 開発の始め方

必要なものは [README.md の「始め方」](README.md#始め方) にあります (Rust・Node.js・pnpm・just・Tauri の前提パッケージなど)。

```sh
just setup      # pnpm install (lefthook の Git フックもここで入る)
just            # レシピの一覧
just crate <name>  # 1 クレートだけ clippy とテスト (反復中)
just ci         # すべての検査 (PR の前に必ず通す)
```

最初に [AGENTS.md](AGENTS.md) と [docs/architecture.md](docs/architecture.md)、触るクレートの `README.md` を読んでください。
作業の内容ごとに読むものは [AGENTS.md の表](AGENTS.md#最初に読むもの-作業内容別) にあります。

## PR を出す前に

- **`just ci` を通す。** fmt・clippy (`-D warnings`)・テスト・依存方向・ライセンス・フロントエンドの検査をまとめて行います。
  CI でも同じ検査を行います。テストの書き方は [docs/testing.md](docs/testing.md)。
- **規約を守る。** 依存方向・`unsafe` の禁止・エラー型・コメントの言語などは [AGENTS.md の「守るべきルール」](AGENTS.md#守るべきルール) にあります。
- **仕様とコードをそろえる。** モデルの型やファイル形式を変えたら、同じ PR で仕様書を直します ([AGENTS.md の「仕様とコードの同期」](AGENTS.md#仕様とコードの同期))。
- **振る舞いを変えたらテストを書く。**

## コミット

- [Conventional Commits](https://www.conventionalcommits.org/) の形で、**すべて英語** で書きます (例: `fix(runtime): clamp keyform weights (T0123)`)。
- scope は省略できます。書くなら `commitlint.config.mjs` の `SCOPES` から選びます。タスクに関するコミットは件名の末尾にタスク ID を付けます。
- Git のフック (lefthook) が整形・検査・commitlint を行います。**`--no-verify` でフックを飛ばさないでください。** 失敗したら原因を直します。
- 詳細は [AGENTS.md の「コミット」](AGENTS.md#コミット)。メッセージは `just commitlint "<メッセージ>"` で事前に検査できます。

## ライセンス

貢献したものは、置く場所のライセンスで公開されます ([ADR 0018](docs/adr/0018-licensing-and-1.0-scope.md))。

| 場所 | ライセンス |
| --- | --- |
| `crates/`・`bindings/`・`docs/` | MIT |
| `apps/editor/`・`apps/viewer/`・`apps/studio/`・`packages/` | GPL-3.0-only |
| `spec/` (公開仕様、英語) | CC BY 4.0 |

- MIT の場所 (`crates/`・`bindings/`) に、GPL のコードを写したり、コピーレフト (MPL・LGPL・GPL など) のライブラリを依存させたりしません。`just licenses` が検査します。
- **Live2D Cubism SDK などの外部 SDK のコードを写さず、リポジトリに置きません。** 挙動を調べて分かったことは、自分の言葉で仕様書に書き、出典を示します。
- 新しい外部の依存を入れるときは、PR に理由を書きます。

詳しい方針は [AGENTS.md の「外部のコードとライセンス」](AGENTS.md#外部のコードとライセンス) にあります。
