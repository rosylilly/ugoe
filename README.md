# ugoe

Live2D Cubism と**機能的に互換**な 2D リギング・アニメーションのエンジンとツールチェインです。Rust で書いています。

> 状態: 設計初期 (M0)。API とファイル形式はまだ頻繁に変わります。

- **ランタイム**: Rust 製。CPU で決定的に変形を評価し、描画は wgpu (WebGPU API) で行います。ネイティブ・Web・ゲームエンジンに組み込めます。
- **エディタ**: Tauri v2 + Svelte 5 で作ります。ビューポートは同じランタイムを wasm にして描画します。
- **非互換**: Live2D の `.moc3` などのファイル形式や内部機構とは互換性がありません。データ形式は独自で、Live2D と同じことができるようにします。
- **AI 連携 (予定)**: PSD の取り込み → レイヤーの意味分類 → 自動メッシュ → 自動パラメータ設定。

## 始め方

必要なもの: Rust (`rust-toolchain.toml` で自動導入)、Node.js 26+ (`.tool-versions`)、pnpm (`package.json` の `packageManager` で固定。Node 26 は corepack を同梱しないため `npm i -g corepack && corepack enable` で導入)、[just](https://github.com/casey/just)、各 OS の [Tauri 前提パッケージ](https://v2.tauri.app/start/prerequisites/)。

```sh
just setup      # pnpm install など
just ci         # すべての検査
just editor     # エディタを起動
just cli fixture single-quad > /tmp/quad.json && just cli info /tmp/quad.json
```

## ドキュメント

- [docs/manual/](docs/manual/README.md) — エディタの操作マニュアル (リガー向け。PSD から動くモデルを作るチュートリアル)
- [AGENTS.md](AGENTS.md) — コントリビュータ (人間・AI エージェント) 向けの規約
- [docs/](docs/README.md) — ビジョン・アーキテクチャ・仕様・ADR・ロードマップ・タスク
- [spec/](spec/README.md) — 公開仕様 (英語、草案)。ugoe 以外の実装がモデルとバンドルを読み書きするための仕様

## ライセンス

- ランタイム・ライブラリ・ツール (`crates/`)・文書 (`docs/`): [MIT](LICENSE)
- エディタ (`apps/editor/`): [GPL-3.0-only](apps/editor/LICENSE)
- ビューア (`apps/viewer/`): [GPL-3.0-only](apps/viewer/LICENSE)
- 公開仕様 (`spec/`、英語): [CC BY 4.0](spec/LICENSE)

詳細と依存ライブラリの方針は [ADR 0018](docs/adr/0018-licensing-and-1.0-scope.md)・[ADR 0019](docs/adr/0019-viewer-license.md)・[ADR 0021](docs/adr/0021-public-specification.md) にあります。
