---
id: T0930
title: "@ugoe/player: localStorage のキーの接頭辞をアプリが渡せるようにする"
status: done
milestone: M9
crates: [packages, apps/viewer, apps/studio, docs]
depends_on: [T0914]
assignee: claude-worker-T0930
---

# T0930: @ugoe/player: localStorage のキーの接頭辞をアプリが渡せるようにする

## 目的

T0914 のレビューで見つけた。@ugoe/player が覚えるもの (光源・モデルごとの光源・着せ替えの組・待機モーション・
トラッキングのままにするパラメータ・自動で戻す・自動で流す・光源の印・覚えたものの並び) の localStorage のキーは
`ugoe-viewer.*` に固定されている。README は「スタジオで使うときに、キーの接頭辞をアプリが渡せるようにする」と書くが、
まだできていない。また、`loadToggle`・`loadLightMarkers`・`loadAutoRestore`・`loadRememberedOrder` などの
「try の中で localStorage を引き、読めなければ既定」という小さな関数が、ファイルごとに少しずつ違う形で重複している。

## 受け入れ条件

- [x] `PlayerHost` (か `Player` の引数) でキーの接頭辞を渡せる。省くと今の `ugoe-viewer` (ビューアの覚えたものを失わない)
- [x] 覚える関数が接頭辞と `Storage` を受け取る 1 つの小さな入れ物にまとまり、各ファイルの重複が無くなる
- [x] スタジオの出力ウィンドウ (受け手) が覚える必要の無いもの (待機モーションの選択を最近使ったものにする書き込みなど) を書かない
- [x] README とテストを合わせる
- [x] `just ci` が通る

## 参考

- packages/player/README.md
- docs/tasks/T0914-review-player.md の所見の表

## メモ

やったこと:

- `packages/player/src/storage.ts` に `PlayerStorage` (接頭辞と `Storage` の組) を足した。キーは「接頭辞 + `.` + 名前」、名前の一覧は `STORAGE_NAMES`。
  読み書きはすべて try の中で行い (localStorage を引くだけで例外になる環境も)、読めなければ「覚えていない」、書けなければ何もしない。
  `get`・`set`・`remove`・`getJson`・`setJson`・`getToggle`・`setToggle`・`clear` (その接頭辞の Player のキーをすべて消す)。
- `PlayerHost.storagePrefix` (省くと `ugoe-viewer`) と `PlayerHost.storage` (覚える先の差し替え) を足し、`Player.storage` で公開した。
  キーの名前の部分は前と同じなので、ビューア (接頭辞を渡さない) は前から覚えたものをそのまま読む (テストでキーの一覧を固定した)。
- 重複していた `loadToggle`・`saveToggle`・`loadLightMarkers`・`saveLightMarkers`・各ファイルの `*_STORAGE_KEY` を除き、
  `ModelMemory` (`new ModelMemory(store, name, parse, { limit })`)・`loadLighting`・`loadTrackingKept`・`loadAutoRestore`・`loadRememberedOrder` などは
  `PlayerStorage` を受け取って読み書きする。`autoRestore` の値は前からの `"true"`・`"false"` のまま (ほかのオン・オフは `"on"`・`"off"`)。
- 受け手のモードで `#recallIdleMotion` が覚えた待機モーションの選択を書き直していたのを直した (受け手は読まず書かない。先に失敗するテストを足した)。
  受け手のモードで読み込み・フレームの適用・`commitLighting` をしても localStorage に何も書かないことをテストで確かめる。
- スタジオ: 出力ウィンドウの Player の接頭辞を `ugoe-studio.player` (`apps/studio/src/lib/settings.ts` の `PLAYER_STORAGE_PREFIX`) にした。
  T0930 より前に出力ウィンドウが `ugoe-viewer.*` に覚えたもの (実際には待機モーションの選択くらい) は、引き継がずに `forgetLegacySettings`
  (操作ウィンドウを開いたとき。T0827 と同じ) で消す。スタジオとビューアは別のアプリ (Tauri の identifier が違い、開発時のポートも違う) なので、
  ビューアの覚えたものは消えない。
- ビューア: 「覚えたもの」の一覧の並びを `player.storage` で読み書きする (`MemoryDialog.svelte`)。
- README (packages/player・apps/viewer・apps/studio) を合わせた。

やらなかったこと:

- OBS のブラウザソース (Output.svelte をスタジオの待ち受けから読むページ) の localStorage に前の `ugoe-viewer.*` が残っていても消さない
  (操作ウィンドウとは別の保存先。受け手は読まないので害は無い)。
- アプリ側の同じ形の小さな関数 (`apps/viewer/src/lib/settings.ts`・`apps/studio/src/lib/settings.ts` の言語の設定の読み書き) は範囲の外なのでそのまま。

起票の候補:

- ビューアとスタジオの `settings.ts` (言語・テーマの localStorage の読み書き) も同じ形の入れ物 (例えば `@ugoe/ui` か `@ugoe/i18n` に置く) にまとめる (T0914 の「横断」の最後の項目)。
- 各テストの `MemoryStorage` (player の 6 ファイル・viewer・studio) を共有のテスト用の部品にまとめる。

`just ci`: 通った (rebase main の後)。
