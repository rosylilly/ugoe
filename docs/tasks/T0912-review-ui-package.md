---
id: T0912
title: コードレビュー: @ugoe/ui
status: done
milestone: M9
crates: [packages, docs]
depends_on: []
assignee: claude-worker-T0912
---

# T0912: コードレビュー: @ugoe/ui

## 目的

@ugoe/ui のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `packages/ui` のすべて

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - Bits UI の包み方の一貫性、props の名前と既定値、`mergeAttributes` などの公開の範囲
  - 部品のテスト (happy-dom) の穴、docs/ui.md の部品の一覧との一致 (見た目の見直しは M10 の T0916・T0917)
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0920〜T0924 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

読んだもの: `packages/ui` の src (部品 24・関数のファイル・tokens.css・テストと harness)・locales・gallery・scripts/shots.mjs・設定・README。
Bits UI の包み方 (props は ugoe の言葉、Bits UI の props は外に出さない) は全部品でそろっていた。docs/ui.md の部品の一覧と実装は一致
(`Toaster`・`TooltipProvider`・`AlertDialog` は一覧の行の中で触れている)。

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 中 | `src/components/Toaster.svelte` | ポインタとフォーカスで止める仕組みが 1 つの旗を共有していて、フォーカスが中にあるままポインタが離れると (またはその逆で) 時間が進み、知らせが消えた (WCAG 2.2.1) | 直した (ポインタとフォーカスを別に覚え、どちらかがあれば止める。テストを先に追加) |
| 中 | `src/number.ts` `formatEditable` | 1e21 以上の値で toFixed が返す指数の表記の末尾の 0 を削り、`1.2e+30` を `1.2e+3` にした (矢印で増減すると 1200 から動く)。負の `decimals` で RangeError | 直した (指数の表記は削らない。桁数は roundTo と同じく 0〜15 に収める。テストを先に追加) |
| 中 | `src/shortcut.ts` `ariaKeyShortcuts` | "+" のキーを `Control++` と書いた。aria-keyshortcuts では区切りと区別するため `Plus` と書く | 直した (テストを先に追加) |
| 低 | `src/components/NumberField.svelte` | 値が NaN のときに矢印で増減すると `onchange(NaN)` を呼んだ | 直した (範囲に収めた 0 から動かす。テストを先に追加) |
| 中 | `src/*.test.ts` | テストの穴: `Toggle` の `tooltip` と `describedby` の重ね方、`Menu` のサブメニュー、`Popover` の `onclose`、`DropZone` のドラッグの数え方 (子の要素への出入り・ファイルでないドラッグ) と `disabled` | テストを足した (どれも今の実装で通る) |
| 低 | `README.md` | 部品の既定の文言を「`slider.reset.aria` だけ」と書いていた (実際は閉じる・キャンセル・種類の言葉・ColorPicker・DropZone・ThemeSetting も) | 直した |
| 低 | `src/props.ts`・`README.md` | `mergeAttributes` を「内部用」と書いていたが、`index.ts` から出していて、エディタの `TipButton` が使っている | 直した (自前のトリガーを作るときに使う公開の関数と書いた) |
| 低 | `src/tokens.css`・docs/ui.md | `.ugoe-visually-hidden` が「部品が使う補助のクラス」とだけ書かれ、アプリ向けに書かれていない。エディタとビューアの 4 か所が同じ CSS を `.visually-hidden` として書き直している (並行のレビューの指摘) | `@ugoe/ui` 側は直した (公開のクラスとして docs/ui.md#そのほか と README に書いた)。アプリの置き換えは T0921 を起票 |
| 低 | docs/ui.md の部品の一覧 | `Notice` を「閉じるボタン付き」と書いていたが、`ondismiss` を渡したときだけ出す | 直した |
| 低 | `Checkbox`・`Toggle`・`NumberField`・`Select`・`SegmentedControl`・`ColorPicker`・`Tabs` と `Button` | 読み上げの props の名前が `describedby`・`labelledby`・`ariaLabel` と `aria-describedby`・`aria-labelledby` に分かれている。トリガーのスニペットも `Popover`・`Menu` は `trigger`、`Tooltip` は `children` | T0920 を起票 (公開 API の変更で、3 つのアプリの書き換えを伴う) |
| 低 | `src/index.ts` | `export *` で部品の内部の関数 (`number.ts`・`files.ts`・`parseShortcut`・テーマの load/save・`toasts`・`dismissToast`) まで出している | T0920 を起票 |
| 低 | `DropZone.svelte` | `disabled` の間は dragover・drop を止めないので、ブラウザの既定 (ファイルを開く) が働く | 見送り (Tauri のアプリは WebView の外でドロップを受けるので起きない。ウィンドウ全体のドロップの扱いはアプリが決めること) |
| 低 | `Slider.svelte` | リセットは `defaultValue` を `min`・`max` に収めずに渡す | 見送り (既定値は呼び手が範囲の中で決めるもの。収めると呼び手の誤りが隠れる) |
| 低 | `ColorPicker.svelte` | `value` が読めない文字 (`"red"`) のとき、黒を選んでも `value` を書き換えない (表示は黒) | 見送り (`#rrggbb` を渡す約束の外。害が小さい) |
| 低 | `Slider.svelte` | `<svelte:document onpointerleave>` は document では来ない (pointerup・pointercancel で足りている) | 見送り (害が無い) |

### 横断

- **WAI-ARIA の書き方の細部:** aria-keyshortcuts の `Plus` のように、属性の値の書き方の決まりはテストで確かめにくい。アプリで `aria-keyshortcuts` を直書きしている所 (あれば) も同じ誤りがありうる。
- **ポインタとフォーカスの 2 つの入口で同じ状態を切り替える処理** (止める・開く・見せる) は、1 つの旗を共有すると片方の離脱でもう片方の状態を消す。ほかのパネルの hover/focus の扱いにもありうる。
- **`Number.prototype.toFixed` の指数の表記:** 1e21 以上で指数の文字列を返す。数値を文字列にして末尾を削る処理 (エディタ・スタジオの数値の表示) に同じ誤りがありうる。
- **共有の CSS の書き直し:** `.visually-hidden` のように、`@ugoe/ui` にあるものをアプリが書き直している例がある (T0921)。`.ugoe-*` の公開のクラスは docs/ui.md に一覧があると見つけやすい。

### just ci

`git rebase main` (追従済み) の後に `just ci` を 1 回流した。fmt-check から spec まで (Rust の clippy・テストを含む) は通り、
`web-check` の `@ugoe/editor` の `src/lib/preview/tracking.test.ts` だけが `Hook timed out in 10000ms` (beforeAll の `await import("./preview.svelte")`) で落ちた。
並行のビルドで負荷が高い (load average 33〜81) ための時間切れで、この変更とは関係ない: 同じテストは main (05c82cf) の作業ツリーでも同じく落ち、
`--hookTimeout=120000` を付けるとこのブランチで 7 件とも通る。`web-check` の残り (viewer・studio・face-tracking・player・i18n・runtime・ui の check と test、editor の check) と
`i18n`・`app-build-check` は個別に流して通った。
