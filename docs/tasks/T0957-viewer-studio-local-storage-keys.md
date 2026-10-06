---
id: T0957
title: "ビューア・スタジオ: localStorage のキーの名前を ugoe.viewer.*・ugoe.studio.* にそろえる"
status: done
milestone: M9
crates: [apps/viewer, apps/studio, packages/player, docs]
depends_on: []
assignee: claude-worker-T0957
---

# T0957: ビューア・スタジオ: localStorage のキーの名前を ugoe.viewer.*・ugoe.studio.* にそろえる

## 目的

T1633 でエディタのキーを `ugoe.editor.*` にそろえたが、ビューアは `ugoe-viewer.locale`・`ugoe-viewer.theme`、スタジオは `ugoe-studio.*`、
`@ugoe/player` の覚えるものの接頭辞は `ugoe-viewer` (`DEFAULT_STORAGE_PREFIX`)・`ugoe-studio.player` と、形が違う (T1633 の候補)。
リリース前なので、移行 (古いキーの読み替え) はしない (利用者の方針)。

## 受け入れ条件

- [x] ビューアのキーが `ugoe.viewer.*`、スタジオのキーが `ugoe.studio.*` になった。`@ugoe/player` の既定の接頭辞と、スタジオが渡す接頭辞も同じ形にした。各アプリの `index.html` のテーマのキーも同じ名前を読む
- [x] キーの名前を 1 か所 (アプリごとの一覧) にまとめた (T1633 のエディタと同じ形)
- [x] 古いキーを引き継ぐ・消すための処理 (スタジオの `LEGACY_STORAGE_KEYS`・`LEGACY_PLAYER_STORAGE_PREFIX` など) を外した (リリース前なので移行は要らない)
- [x] キーの名前を書いた文書 (docs/studio.md・各アプリの README など。`ugoe-viewer`・`ugoe-studio.` で探す) を今の形に直した
- [x] `just e2e --only viewer`・`just e2e --only studio` (キーを読み書きしていれば) と `just ci` が通る

## 参考

- T1633 のメモ (やったこと・起票の候補)
- apps/viewer/src/lib/settings.ts、apps/studio/src/lib/settings.ts、packages/player/src (`storage.ts`・`player.svelte.ts`)、各アプリの index.html

## メモ

- T0043 で起票した (元の候補: T1633 (ビューア・スタジオのキー))。T1633 のもう 1 つの候補 (generated の wasm があるときの Vitest の失敗) は T1641 で直った。
- T0957 (claude-worker-T0957):
  - やったこと: ビューアの `src/lib/settings.ts` を `storageKeys.ts` (`STORAGE_PREFIX` = `ugoe.viewer`、`STORAGE_KEYS` の `locale`・`theme`) に置き換え、スタジオに `src/lib/storageKeys.ts` (`ugoe.studio`、`locale`・`theme`、`PLAYER_STORAGE_PREFIX` = `ugoe.studio.player`) を作った。`@ugoe/player` の `DEFAULT_STORAGE_PREFIX` を `ugoe.viewer` にした (Player の名前 `STORAGE_NAMES` (`lighting.models` など) はそのまま)。両アプリの `index.html` のテーマのキーも直した。
  - 古いキーの片付けを外した: スタジオの `LEGACY_STORAGE_KEYS`・`LEGACY_PLAYER_STORAGE_PREFIX`・`forgetLegacySettings` (と `Control.svelte` の呼び出し)、それにしか使っていなかった `PlayerStorage.clear()`。スタジオの `settings.ts` はプロファイルの値 (出力の大きさ・刻みの速さ) の読み方だけになった。
  - テスト: 各アプリの `storageKeys.test.ts` で形 (`ugoe.<アプリ>.<名前>`)・Player のキーとの重なりが無いこと・ビューアの接頭辞が Player の既定と同じこと・`index.html` が読むキーがテーマのキーと一致することを検査する (エディタの T1633 と同じ形)。古いキーを読めることを確かめていたテスト (T0925・T0937) と `clear()` のテストは消した。`packages/player` のテストのキーは新しい名前にした。
  - 文書: apps/viewer/README.md に「覚える値」(全キーの表) を足し、キーの記述を直した。docs/studio.md に「覚える値」を足し、T0827 の「localStorage からの移し替え」を「localStorage に持たない」に書き直した。apps/studio/README.md・packages/player/README.md・packages/i18n/README.md・packages/ui/README.md・docs/ui.md と、コメント (`packages/i18n/src/preference.ts`・`packages/ui/src/theme.svelte.ts`) のキーも直した。
  - 範囲の外の小さな変更: `packages/i18n` と `packages/ui` の README とコメントの例のキー、docs/ui.md。
  - `just e2e --only viewer` (6 件)・`just e2e --only studio` (13 件) はすべて通った。`just ci` は通った。
  - やらなかったこと: Player の名前 (`dress-up` など kebab-case) をエディタの camelCase にそろえることはしていない (受け入れ条件は接頭辞の形だけ)。
  - 起票の候補: (1) `@ugoe/player` の `STORAGE_NAMES` の名前 (`lighting.models`・`dress-up.auto-restore` など) を、エディタ・アプリのキー (camelCase) と同じ形にそろえるか決める。
