# apps/editor

ugoe のエディタ (Tauri v2 + Svelte 5)。設計は [docs/editor.md](../../docs/editor.md)。

```
src/              フロントエンド (Svelte 5 + TS)
  lib/ipc.ts        バックエンドとの通信はすべてここを通す
src-tauri/        バックエンド (Rust クレート `ugoe-editor`)
  src/commands.rs   IPC コマンド。薄く保ち、編集ロジックは ugoe-editor-core に置く
```

## 守ること

- 編集ロジックを書かない。ユーザーの入力を `Command` に変換して `apply_command` を呼ぶ。
- IPC コマンドを変えたら、同じ PR で `src/lib/ipc.ts` を更新する。
- Svelte 5 の runes を使う。状態管理のライブラリは入れない。

## コマンド

```sh
just editor         # 開発モードで起動 (vite + tauri dev)
just editor-build   # リリースビルド
just web-check      # Biome + svelte-check
```
