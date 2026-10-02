# 翻訳の手順

ugoe のエディタ・ビューア・スタジオの画面の文言を訳す人と、文言を足す開発者のための手順です。
仕組みは [`@ugoe/i18n`](../packages/i18n/README.md)、決めた経緯は [ADR 0029](adr/0029-shared-ui-and-i18n.md)、文の書き方 (キーの名前・文体) は
[docs/ui.md の「文言の書き方」](ui.md#文言の書き方) にあります。翻訳サービス (Weblate・Crowdin など) は、まだ導入していません (ADR 0029 の 7)。
導入するまでは、JSON のファイルを直接編集して PR を送ってください。

## カタログの置き場所

| 場所 | 名前空間 | 中身 |
| --- | --- | --- |
| `packages/face-tracking/locales/<言語>.json` | `face-tracking` | フェイストラッキングのパネル |
| `packages/<名前>/locales/<言語>.json` | パッケージの短い名前 | 共有の部品 (`ui`・`player` など。できたものから) |
| `apps/<名前>/locales/<言語>.json` | アプリの名前 | アプリの画面 (`editor`・`viewer`・`studio`。移行したものから) |

- 形式は [i18next JSON v4](https://www.i18next.com/misc/json-format) です。キーは入れ子のオブジェクトで、値は文字列です。
- **原文は英語 (`en.json`)** です。日本語 (`ja.json`) は常に欠けのない訳にします。
- 対応する言語の一覧は [`packages/i18n/src/languages.json`](../packages/i18n/src/languages.json) です。
- 操作マニュアル (`docs/manual/`) は日本語だけで、いまは訳しません (ADR 0029 の 6)。

## 訳すときの約束

- **キー (左側) は変えず、値 (右側) だけを訳します。**
- `{{name}}` は実行時に値が入る引数です。名前を変えたり、訳さずに消したりしないでください (語順は変えてかまいません)。
  `{{value, number}}` の `, number` は書式の指定なので、そのまま残します。
- `_one`・`_other` などで終わるキーは複数形です。言語ごとに必要な形が違います ([CLDR の複数形の区分](https://www.unicode.org/cldr/charts/latest/supplemental/language_plural_rules.html))。
  英語は `_one` と `_other`、日本語は `_other` だけです。その言語に要る形をすべて書き、要らない形は書きません。
- `<link>…</link>` のような印は、文の一部をリンクなどにするためのものです。印の名前は変えず、囲む語句を訳に合わせて選びます。
- 製品名・形式の名前 (ugoe・MediaPipe・`.ugoe`・OBS) と、`` `pnpm mediapipe` `` のようなコマンドは訳しません。
- 訳が分からないキーは、値を空の文字列 (`""`) にするかキーを書かずにおけば、英語が出ます (日本語を除く。日本語では `just i18n` が欠けとして止めます)。

## 新しい言語を足す

1. `packages/i18n/src/languages.json` の `languages` に `{ "tag": "<言語>", "name": "<その言語自身での名前>" }` を足します。
   タグは BCP 47 の言語の部分 (`fr`・`ko` など) です。常に欠けのない訳にするなら `"complete": true` を付けます。
2. `packages/i18n/src/languages.ts` の `Locale` の型に同じタグを足します。
3. 各カタログの置き場所に `<言語>.json` を作ります (訳した分だけでかまいません)。
4. `just i18n` で検査します。3 つ目の言語を足すときは、複数形の区分と文字の向きを確かめます (ADR 0029 の「見直すきっかけ」)。

## 文言を足す・変える (開発者)

1. 原文 (`en.json`) にキーを足します。キーの名前は `<画面または部品>.<要素>.<種類>` ([docs/ui.md](ui.md#キーの名前))。
2. **同じ PR で `ja.json` の訳も足します。**
3. コードでは、そのパッケージの `t()` にキーを文字列のまま渡します (`t("calibration.calibrate.label")`)。キーを組み立てると、
   未使用のキーの検査が頭の一致 (`source.`) でしか見られなくなるので、キーを並べた表を書くほうがよいです。
4. 英語の文を直すときはキーを変えません。意味が変わったら、新しいキーにします (古い訳が残らないように)。

## 訳を確かめる

- `just i18n`: キーの過不足・複数形・引数・空の訳・未使用のキーと、画面の直書きの文言 (下) を検査します (`just ci` に入っています)。
- `pnpm --filter <パッケージ> check`: コードの中のキーが原文にあるかを型で確かめます。
- 画面で確かめる: いまは言語を切り替える画面がまだ無いため、アプリの `src/main.ts` の `setLocale("...")` を一時的に変えて
  `just editor` / `just viewer` で開きます (言語の切り替えの画面は後続のタスクで作ります)。英語は日本語より長くなりやすいので、
  ボタンや見出しの文字が切れていないかを見ます。

## 直書きの文言の検査

カタログを通さずに画面に出す文言を、`scripts/check-hardcoded-strings.mjs` (`just i18n`) が見つけます。
`apps/*` と `packages/*` の `.svelte` と `.ts` を Svelte の公式のパーサ (`svelte/compiler` の `parse`) で読みます。
テスト (`*.test.ts`・`test/`)・設定 (`*.config.ts`)・生成物 (`generated/`)・`src-tauri/`・`@ugoe/ui` の部品の見本 (`gallery/`) は見ません。

**見つけるもの:**

- マークアップのテキストノード (`<button>Save</button>`)。
- 表示に使う属性の値: `title`・`alt`・`placeholder`・`aria-label`・`aria-description`・`aria-placeholder`・
  `aria-roledescription`・`aria-valuetext` と、部品の props の `label`・`tooltip`・`description`・`hint`・`heading`・`caption`・`message`・`text`。
- `{...}` で表示する式の、値になる位置の文字列: `{busy ? "Saving" : "Save"}` の 2 つ、`{name ?? "Untitled"}` の右。
  `t("key")` の引数のように、関数に渡す文字列は値ではないので見ません。
- スクリプト (`<script>` と `.ts`) の文字列のうち、次のもの。
  - 表示に使う場所にあるもの: オブジェクトの `label:`・`title:`・`message:` など (上の props と同じ名前)、`el.textContent = ...`・
    `el.title = ...` などの代入、`setAttribute("title", ...)`、`alert`・`confirm`・`prompt` と Tauri の dialog の `message`・`ask` の引数。
  - ASCII でない文字 (日本語など) を含むもの。
  - 英語の文に見えるもの (大文字で始まる語の後に、空白で区切った語が続く。`Warp deformer`・`` `Loaded ${n} files` ``)。

**見つけないもの (誤検知を減らす約束):**

- 文字を含まないもの (記号・数値・空の文字列。`—`・`42 %`・`×`)。
- ID・キー・クラス名・単位・値の名前のような、小文字の識別子 (`dialog-title`・`editor.save.label`・`image/png`・`px`・`yaw`)。
  テキストノードと表示の属性でも同じです。
- 表示に使わない属性 (`class`・`id`・`href`・`type` など) の固定の値と、表示に使わない場所の英語の 1 語 (`kind: "Warp"`)。
- 比較 (`e.key === "Escape"`)・`case`・型 (`type Mode = "Edit"`)・`import` の文字列・プロパティのキー。
- 開発者向けの出力: `console.*` の引数と、例外の文 (`throw ...`・`new Error(...)`・`new TypeError(...)` など、名前が `Error` で終わるもの)。
- `<style>`・`<script>` の要素の中のテキスト。
- **訳さない文字列** (製品名・コマンドの例など) は、その行か前の行のコメントに `i18n-ignore` と理由を書きます
  (`<!-- i18n-ignore: 製品名 -->`・`// i18n-ignore: シェルのコマンド`)。コメントの行と次の行が対象の外になります。

**移行の途中の許す一覧:** まだカタログに移していないファイルは、`scripts/hardcoded-strings-allowlist.json` に
アプリ・パッケージごとにファイルと件数を持ちます (載っていないファイルは 0 件)。

- 一覧の件数より **増えたら** 失敗し、そのファイルで見つけた文言を行番号付きで出します。
- 一覧の件数より **減ったら**、一覧を縮めるまで失敗します。`node scripts/check-hardcoded-strings.mjs --update` で、減ったファイルの件数を
  書き戻します (増えた分は書きません)。一覧は減る向きにしか動かないので、手で件数を増やさないでください。
- 移行の作業では `node scripts/check-hardcoded-strings.mjs --list [パス...]` で、一覧に載っている分を含めてすべて出せます。
- 新しいアプリ・パッケージは、一覧に載せずに (0 件で) 始めます。
