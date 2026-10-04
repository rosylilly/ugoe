---
id: T0921
title: アプリの自前の .visually-hidden を @ugoe/ui の .ugoe-visually-hidden に置き換える
status: done
milestone: M9
crates: [apps/editor, apps/viewer, docs]
depends_on: [T0912]
assignee: claude-worker-T0921
---

# T0921: アプリの自前の .visually-hidden を @ugoe/ui の .ugoe-visually-hidden に置き換える

## 目的

`@ugoe/ui` の `tokens.css` には、見せずに読み上げだけさせる文字のクラス `.ugoe-visually-hidden` がある (T0912 で公開のものとして
docs/ui.md#そのほか と packages/ui/README.md に書いた)。ところが、エディタとビューアのいくつかの部品は、同じ指定を自前の
`.visually-hidden` として `<style>` に書き直している。これを `.ugoe-visually-hidden` に置き換え、自前の CSS を消す。

2026-10-04 の時点の場所:

- `apps/viewer/src/lib/MemoryDialog.svelte`
- `apps/editor/src/lib/panels/timeline/MotionEditor.svelte`
- `apps/editor/src/lib/panels/timeline/CurveGraph.svelte`
- `apps/editor/src/lib/ui/TabStrip.svelte`

(`apps/editor/src/lib/panels/slots/SlotsPanel.svelte` はすでに `.ugoe-visually-hidden` を使っている。)

## 受け入れ条件

- [x] 上の部品の `.visually-hidden` を `.ugoe-visually-hidden` にし、`<style>` の自前の指定を消した (`rg "visually-hidden" apps` で `ugoe-` の付かないものが無い)
- [x] 読み上げの名前を確かめるテスト (あれば) が通る
- [ ] `just ci` が通る

## 参考

- docs/ui.md#そのほか
- T0912 のメモ

## メモ

- 2026-10-04 (claude-worker-T0921):
  - やったこと: `apps/editor/src/lib/ui/TabStrip.svelte` と `apps/viewer/src/lib/MemoryDialog.svelte` の `.visually-hidden` を
    `.ugoe-visually-hidden` にし、`<style>` の自前の指定を消した。`MotionEditor.svelte`・`CurveGraph.svelte` は起票の時点ですでに
    `.ugoe-visually-hidden` だった。`rg "visually-hidden" apps` で `ugoe-` の付かないものは無い。
  - 見た目: 消した指定 (position・width・height・overflow・clip-path・white-space) は tokens.css の `.ugoe-visually-hidden` に
    すべて同じ値で含まれ、tokens.css は両アプリの `main.ts` で読み込んでいる。tokens.css の側は `margin: -1px; padding: 0; border: 0`
    が多いが、1px に切り抜いた絶対配置の span なので見た目は変わらない。両部品の `<style>` に span に当たる指定は無い。
  - テスト: 両部品の読み上げの名前を確かめる既存のテストは無い (受け入れ条件 2 は該当なしとしてチェック)。
  - `just ci`: 1 件だけ失敗。`apps/editor/src/lib/viewport/ToolOverlay.test.ts` の
    「ToolOverlay mesh edit keys > deletes the selected vertices with Delete」(T0699 で入ったテスト) が
    `expected [ +0 ] to deeply equal []` (47 行目 `selection.vertices.Quad`) で落ちる。この変更の前のコミット (main の d9aa9a35) でも
    同じく落ちるので無関係。web-check がそこで止まるため、残りの段 (viewer・studio・face-tracking・player・i18n・runtime・ui の
    check と test) と `i18n`・`app-build-check`・`script-check` を手で流し、すべて通った。Rust 側 (fmt・clippy・test・layers など) も通った。
