# 0001: 技術スタック (Rust / wgpu / Tauri / Svelte)

- 状態: Accepted
- 日付: 2026-09-28

## 背景

Live2D と機能的に互換なエンジンとエディタを作る。ランタイムはネイティブ・Web・ゲームエンジンに移植でき、エディタはデスクトップで動く必要がある。

## 決定

- 実装言語は **Rust** (edition 2024)。toolchain は `rust-toolchain.toml` で固定する。
- 描画は **wgpu** (WebGPU API)。ネイティブの Vulkan / Metal / DX12 / GL と、ブラウザの WebGPU / WebGL2 を 1 つのコードで扱える。
- エディタは **Tauri v2**。フロントエンドは **Svelte 5 + TypeScript + Vite**。
- タスクランナーは **just**、JS のパッケージマネージャは **pnpm**、JS/TS の lint と整形は **Biome**。

## 検討した代替案

| 案 | 利点 | 欠点 |
| --- | --- | --- |
| エディタを egui などネイティブ UI で作る | Rust だけで完結し、ビューポートの統合が容易 | ツリー・タイムライン・インスペクタなど複雑な UI を作る手間が大きい。Web の UI 資産を使えない |
| フロントエンドを React にする | エコシステムが最大 | ランタイムが重い。細かい更新が多いエディタでは、Svelte のリアクティビティのほうが単純に書ける |
| 描画を OpenGL (glow) で直接書く | 対応環境が最も広い | Metal や WebGPU への移行が必要になる。wgpu の GL バックエンドで十分に代替できる |

## 結果

- Rust のコアを、エディタ・CLI・wasm・FFI で共有できる。
- wgpu の破壊的変更への追従が定期的に必要になる。wgpu への依存は `ugoe-render` だけに閉じ込める。
