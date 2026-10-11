# ugoe

Live2D Cubism と**機能的に互換**な 2D リギング・アニメーションのエンジンとツールチェインです。Rust で書いています。

> 状態: 開発中 (未リリース)。1.0 の範囲 (M1〜M11。[docs/roadmap.md](docs/roadmap.md#10-の範囲)) のマイルストーンは完了し、残りの後続のタスクと公開の準備を進めています。
> 1.0 より前は API とファイル形式が予告なく変わります。

- **ランタイム**: Rust 製。CPU で決定的に変形を評価し、描画は wgpu (WebGPU API) で行います。ネイティブ・Web・ゲームエンジンに組み込めます。
- **アプリ**: エディタ (リギング)・ビューア (確かめる)・スタジオ (配信) を Tauri v2 + Svelte 5 で作ります。描画は同じランタイムを wasm にして行います。
- **非互換**: Live2D の `.moc3` などのファイル形式や内部機構とは互換性がありません。データ形式は独自で、Live2D と同じことができるようにします。
- **AI 支援**: PSD の取り込み → レイヤーの意味分類 → 自動メッシュ → 自動パラメータ設定を、編集の提案として出します (適用は人が承認する)。MCP サーバから外部のエージェントも使えます。

## 始め方

必要なもの: Rust (`rust-toolchain.toml` で自動導入)、Node.js 26+ (`.tool-versions`)、pnpm (`package.json` の `packageManager` で固定。Node 26 は corepack を同梱しないため `npm i -g corepack && corepack enable` で導入)、[just](https://github.com/casey/just)、各 OS の [Tauri 前提パッケージ](https://v2.tauri.app/start/prerequisites/)、`just ci` のライセンスの表記の検査に cargo-about (`just cargo-about` で決めた版を入れる)。

```sh
just setup      # pnpm install など
just ci         # すべての検査
just editor     # エディタを起動
just cli fixture single-quad > /tmp/quad.json && just cli info /tmp/quad.json
```

## ドキュメント

- [docs/manual/](docs/manual/README.md) — エディタの操作マニュアル (リガー向け。PSD から動くモデルを作るチュートリアル)
- [AGENTS.md](AGENTS.md) — コントリビュータ (人間・AI エージェント) 向けの規約
- [CONTRIBUTING.md](CONTRIBUTING.md) — 貢献の手順 (人の貢献者向けの短い案内。English summary at the top)
- [SECURITY.md](SECURITY.md) — 脆弱性の報告先 (公開の Issue に書かない。日本語と英語)
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) — 行動規範 ([Contributor Covenant](https://www.contributor-covenant.org/version/2/1/code_of_conduct/) 2.1 をそのまま使う。英語)
- [docs/](docs/README.md) — ビジョン・アーキテクチャ・仕様・ADR・ロードマップ・タスク
- [spec/](spec/README.md) — 公開仕様 (英語、草案)。ugoe 以外の実装がモデルとバンドルを読み書きするための仕様

## ライセンス

- ランタイム・ライブラリ・ツール (`crates/`)・バインディング (`bindings/`)・文書 (`docs/`): [MIT](LICENSE)
- エディタ (`apps/editor/`): [GPL-3.0-only](apps/editor/LICENSE)
- ビューア (`apps/viewer/`): [GPL-3.0-only](apps/viewer/LICENSE)
- スタジオ (`apps/studio/`): [GPL-3.0-only](apps/studio/LICENSE)
- アプリで共有するフロントエンドの部品 (`packages/`): GPL-3.0-only (各パッケージの LICENSE)
- 公開仕様 (`spec/`、英語): [CC BY 4.0](spec/LICENSE)
- 行動規範 (`CODE_OF_CONDUCT.md`): [Contributor Covenant](https://www.contributor-covenant.org/version/2/1/code_of_conduct/) 2.1 の文のまま ([CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)。2022 年の [Contributor Covenant のリポジトリのライセンス](https://github.com/EthicalSource/contributor_covenant/blob/91532d80ea5f39b6685aff4892bc6854bf2f6d6f/LICENSE.md))

詳細と依存ライブラリの方針は [ADR 0018](docs/adr/0018-licensing-and-1.0-scope.md)・[ADR 0019](docs/adr/0019-viewer-license.md)・[ADR 0021](docs/adr/0021-public-specification.md)・[ADR 0025](docs/adr/0025-web-runtime-package.md)・[ADR 0028](docs/adr/0028-studio.md) にあります。
