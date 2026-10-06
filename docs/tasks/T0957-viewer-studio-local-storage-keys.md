---
id: T0957
title: "ビューア・スタジオ: localStorage のキーの名前を ugoe.viewer.*・ugoe.studio.* にそろえる"
status: todo
milestone: M9
crates: [apps/viewer, apps/studio, packages/player, docs]
depends_on: []
assignee:
---

# T0957: ビューア・スタジオ: localStorage のキーの名前を ugoe.viewer.*・ugoe.studio.* にそろえる

## 目的

T1633 でエディタのキーを `ugoe.editor.*` にそろえたが、ビューアは `ugoe-viewer.locale`・`ugoe-viewer.theme`、スタジオは `ugoe-studio.*`、
`@ugoe/player` の覚えるものの接頭辞は `ugoe-viewer` (`DEFAULT_STORAGE_PREFIX`)・`ugoe-studio.player` と、形が違う (T1633 の候補)。
リリース前なので、移行 (古いキーの読み替え) はしない (利用者の方針)。

## 受け入れ条件

- [ ] ビューアのキーが `ugoe.viewer.*`、スタジオのキーが `ugoe.studio.*` になった。`@ugoe/player` の既定の接頭辞と、スタジオが渡す接頭辞も同じ形にした。各アプリの `index.html` のテーマのキーも同じ名前を読む
- [ ] キーの名前を 1 か所 (アプリごとの一覧) にまとめた (T1633 のエディタと同じ形)
- [ ] 古いキーを引き継ぐ・消すための処理 (スタジオの `LEGACY_STORAGE_KEYS`・`LEGACY_PLAYER_STORAGE_PREFIX` など) を外した (リリース前なので移行は要らない)
- [ ] キーの名前を書いた文書 (docs/studio.md・各アプリの README など。`ugoe-viewer`・`ugoe-studio.` で探す) を今の形に直した
- [ ] `just e2e --only viewer`・`just e2e --only studio` (キーを読み書きしていれば) と `just ci` が通る

## 参考

- T1633 のメモ (やったこと・起票の候補)
- apps/viewer/src/lib/settings.ts、apps/studio/src/lib/settings.ts、packages/player/src (`storage.ts`・`player.svelte.ts`)、各アプリの index.html

## メモ

- T0043 で起票した (元の候補: T1633 (ビューア・スタジオのキー))。T1633 のもう 1 つの候補 (generated の wasm があるときの Vitest の失敗) は T1641 で直った。
