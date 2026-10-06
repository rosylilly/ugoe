# packages/ui

エディタ・ビューア・スタジオで共有する UI の部品とデザイントークン (npm パッケージ `@ugoe/ui`、pnpm のワークスペース)。
方針は [ADR 0029](../../docs/adr/0029-shared-ui-and-i18n.md)、トークンの値・部品の一覧・アクセシビリティの基準は
[docs/ui.md](../../docs/ui.md) にある。T0902 で 1 段目の部品、T0903 で 2 段目の部品 (重ねて出すもの・知らせ・色・ファイル・ショートカット) を作った。

**ライセンスは GPL-3.0-only** (使うアプリと同じ)。ふるまいの土台の [Bits UI](https://bits-ui.com/) とその依存は MIT・Apache-2.0
(`just licenses` が推移的に検査する)。

```
src/tokens.css          デザイントークン (CSS 変数)。ライト・ダーク (data-theme)、compact・comfortable (data-density)。文字の欄の共通の見た目も
src/index.ts            公開する部品と関数 (アプリとほかのパッケージが使うものだけを名前で出す)
src/components/         部品 (Button・IconButton・Checkbox・Toggle・Slider・NumberField・TextField・TextArea・SecretField・Select・SegmentedControl・
                        Tabs・Section・Toolbar・Dialog・AlertDialog・Notice・Toaster・Tooltip・TooltipProvider・
                        Menu・ContextMenu・Popover・ColorPicker・DropZone・Kbd・ThemeSetting・Icon)
src/number.ts           数値の欄の解釈・書式・増減 (純粋な関数)
src/commit.ts           数値の欄の確定の印 (numberFieldCommit。同じ欄の続けての確定を受け手がまとめるため)
src/shortcut.ts         ショートカットの表記 (Mod → ⌘ / Ctrl)・aria-keyshortcuts
src/scrollGap.ts        一覧が送るときだけ行末をスクロールバーの分空ける印 (markScrolling。Select・Menu・ContextMenu)
src/color.ts            色 (#rrggbb) と RGB
src/files.ts            DropZone の accept による選り分け
src/toast.svelte.ts     Toast の列 (showToast を出す。列と閉じる関数は Toaster が使う)
src/announce.svelte.ts  読み上げだけの知らせ (announce を出す。見えない live region は Toaster が描く。docs/ui.md#状態の変化の知らせ)
src/theme.svelte.ts     テーマの設定 (initTheme・currentTheme を出す。設定の読み書きは ThemeSetting が使う)
src/props.ts            Bits UI のトリガーの属性と ugoe の props を重ねる (mergeAttributes。自前のトリガーを作るときに使う)
src/i18n.ts             部品の既定の文言 (@ugoe/i18n の名前空間 `ui`。カタログは locales/)
src/context.ts          部品の間の context (Toolbar の中の Button を Bits UI の Toolbar.Button で出す)
src/*.test.ts           Vitest (happy-dom の上でキーボード操作と aria を検査する)
gallery/                部品の見本の画面 (vite)
scripts/shots.mjs       見本の画面をヘッドレスの Chrome で撮って検査する
```

## 使い方

```ts
// main.ts で 1 回だけ
import "@ugoe/ui/tokens.css";
```

```svelte
<script lang="ts">
  import { Button, IconButton, Slider } from "@ugoe/ui";
</script>

<Button variant="primary" onclick={save}>{t("file.save.label")}</Button>
<IconButton label={t("view.reset.aria")} onclick={reset}><svg>…</svg></IconButton>
<Slider label={t("tracking.smoothing.label")} bind:value={smoothing} min={0} max={1} step={0.05}
        defaultValue={0.5} />
```

- テーマと密度は `<html data-theme="light|dark" data-density="compact|comfortable">` で決める。省くと OS のテーマと `compact`。
- `Slider` と `NumberField` の `onchange` は利用者の操作のときだけ呼ばれる。外から渡した値が刻みに乗らなくても呼ばれない ([docs/ui.md](../../docs/ui.md#数値の部品と-onchange))。
  `NumberField` の `onchange` の中では `numberFieldCommit()` が欄ごとの印を返す (同じ欄の続けての確定をまとめるため)。
- **アプリと、ほかのパッケージは `bits-ui` を直接 import しない。** 部品の props は ugoe の言葉で決めてあり、Bits UI の props は外に出さない
  (Bits UI の破壊的な変更をこのパッケージの中で吸収するため)。
- **見えるラベル・読み上げの名前・選択肢の名前は、翻訳を通した文言を props で渡す。** 部品が既定で出す文言
  (閉じる・キャンセル・`Slider` のリセットの名前・`Notice` と Toast の種類の言葉・`ColorPicker` の欄の名前・`DropZone` の案内とボタン・
  `ThemeSetting` の選択肢) は、このパッケージのカタログ `locales/<言語>.json` (@ugoe/i18n の名前空間 `ui`) から引く。
  名前の props (`closeLabel`・`cancelLabel`・`resetLabel`・`prompt`・`buttonLabel` など) があるものは上書きできる。
  言語はアプリが @ugoe/i18n の `setLocale()` で決める。
- **見せずに読み上げだけさせる文字** には `class="ugoe-visually-hidden"` を付ける (tokens.css にある。アプリで同じ CSS を書き直さない)。
  部品の中の文字を見せなくするときは、部品の口を使う (`Slider` の `hideLabel`・`ColorPicker` の `hideValue`)。どちらも部品の中を位置の基準にするので、
  送る枠の外へ出ない ([docs/ui.md](../../docs/ui.md#レイアウト) の「送り」)。
- `IconButton` の `label` は必須で、空だとコンソールにエラーを出す。`title` 属性は付けず、同じ文言を `Tooltip` で出す
  (`tooltip={false}` で消せる。`shortcut="Mod+Z"` でツールチップにショートカットを出し、`aria-keyshortcuts` を付ける)。
- `Select` の一覧は body に出す (Portal) ので、`data-theme`・`data-density` は `<html>` に付ける。
- `Select`・`Menu`・`ContextMenu` の一覧は、送るとき (中身が枠より高いとき) だけ `data-scrolling` を付けて行末を `--ugoe-scrollbar-gap` 空ける
  (`src/scrollGap.ts`。重ねて描くスクロールバーが行末に掛からないように。[docs/ui.md](../../docs/ui.md#レイアウト) の「送り」)。
  送らないときの幅は変わらない。`Select` は Bits UI が隠す中の送り (viewport) を使わず、一覧の枠を送ってスクロールバーを見せる。
- **文字の欄は `TextField`:** 見えるラベル (`label`)・補足 (`hint`)・誤りの文 (`error`) を渡すと欄と結ぶ (`aria-describedby`・`aria-invalid`・
  `aria-errormessage`)。行の左に名前を置くときは `id` と外の `<label for>`。`oninput` は打つたび、`onchange` は確定 (Enter・フォーカスを外す) で
  値が変わったときだけ呼ぶ (外から渡した値では呼ばない)。Esc で確定前の値に戻す。
- **欄の説明は `tooltip`:** `Toggle`・`Slider` (つまみ)・`NumberField`・`Select`・`TextField` は `tooltip` で、フォーカスを受ける要素に
  Tooltip を付ける (`title` 属性の代わり。渡した `aria-describedby` と並べて読む)。`Menu` の項目は `description` で名前の下に説明の行を出す。

  ```svelte
  <TextField label={t("rig.name.label")} bind:value={name} error={nameError} />
  <Select label={t("rig.frame.label")} {options} bind:value={frame} tooltip={t("rig.frame.tooltip")} />
  ```
- **読み上げの props は HTML の属性の名前で渡す:** `aria-label` (見えるラベルを置けないとき)・`aria-labelledby` (見えるラベルが別の要素に
  あるとき)・`aria-describedby` (補足の説明)。`Button` と同じ書き方で、`Checkbox`・`Toggle`・`Slider`・`NumberField`・`TextField`・`Select`・
  `SegmentedControl`・`ColorPicker`・`Tabs` も同じ名前で受ける。

  ```svelte
  <span id="{ids}-gravity">{t("physics.gravity.label")}</span>
  <NumberField aria-labelledby="{ids}-gravity" bind:value={gravity} />
  <Select aria-label={t("camera.select.aria")} {options} bind:value={camera} />
  <Toggle label={t("view.mirror.label")} aria-describedby="{ids}-mirror-note" bind:checked={mirror} />
  ```
- **公開の範囲:** `index.ts` は部品と、アプリやほかのパッケージが使う関数 (`initTheme`・`currentTheme`・`showToast`・`announce`・`numberFieldCommit`・
  `mergeAttributes`) と型だけを出す。部品の内部の関数 (`number.ts`・`files.ts`・`shortcut.ts` など) は出さない。アプリで要るものになったら名前を足す。
## 重ねて出す部品 (T0903)

```svelte
<script lang="ts">
  import { AlertDialog, Dialog, Menu, Notice, showToast, Toaster, TooltipProvider } from "@ugoe/ui";
</script>

<!-- アプリの根元に 1 回だけ -->
<TooltipProvider>
  <App />
  <Toaster />
</TooltipProvider>

<Dialog bind:open title={t("rename.title")} description={t("rename.description")} onclose={later}>
  …中身…
  {#snippet actions()}<Button onclick={…}>…</Button><Button variant="primary">…</Button>{/snippet}
</Dialog>
<AlertDialog bind:open title={…} description={…} confirmLabel={t("memory.forgetAll.label")} onconfirm={forgetAll} />
<Menu label={t("menu.file.label")} entries={[{ label: t("menu.save.label"), shortcut: "Mod+S", onselect: save }]} />
<Notice tone="warning" title={…} ondismiss={hide}>…</Notice>
```

```ts
showToast(t("file.saved"), { tone: "success" }); // 既定で 5 秒。ポインタやフォーカスが乗っている間は止まる
```

- **Dialog:** `onclose` は利用者が閉じたとき (Esc・外のクリック・閉じるボタン) だけ呼ぶ。`dismissible={false}` なら中のボタンでしか閉じない。
  閉じるボタンは並びの最後に置くので、開くと最初の入力欄やボタンにフォーカスが行く。別の要素にしたいときは、その部品に `id` を渡し、
  同じ id を `initialFocus` に渡す (不意に出る確認で「後で」に置くなど。見つからなければ最初の要素)。その要素の Tooltip は
  開いたときには出さない (出すと最初の Esc が説明を消すだけになり、ダイアログが閉じない)。
  決定のボタンを `actions` に置いて本文の `<form>` を送るときは、`<Button type="submit" form={formId}>` (フォームの外から送れる)。
- **AlertDialog:** 取り消せない操作の確認。開くとフォーカスは取り消し側。決定は既定で `danger`。
- **Notice** は残る知らせ (`role="status"`、`danger` は `role="alert"`)、**Toast** はすぐ消えてよい知らせ。種類は印と読み上げの言葉でも示す。
- **Tooltip・Popover・Menu の `trigger`:** 自前の要素をトリガーにするときは、`trigger` のスニペットが受け取る属性をフォーカスを受ける要素に広げる
  (3 つとも同じ名前。Menu は省くと ugoe の `Button` を出す)。ugoe の `Button` に広げてよい (`<Button {...attrs as ButtonBaseProps}>`)。

  ```svelte
  <Tooltip text={t("view.fit.tooltip")}>
    {#snippet trigger(attrs)}<Button {...attrs as ButtonBaseProps} onclick={fit}>{t("view.fit.label")}</Button>{/snippet}
  </Tooltip>
  ```
- **ContextMenu** の `label` は一覧の読み上げの名前。右クリックでしかできない操作は作らない。
- **DropZone:** 同じことができる「ファイルを選ぶ…」のボタンを必ず出す。Tauri のように WebView の外でドロップを受けるアプリは、
  `active` で見た目を変え、`onbrowse` で自前のダイアログを開く。
- **Kbd:** `keys="Mod+Shift+Z"`。macOS は ⌘⇧Z、ほかは Ctrl+Shift+Z。
- **Icon:** `<Icon name="close" />`。線のアイコン (大きさと線の太さは tokens.css の `.ugoe-icon`)。ボタンの中の印に文字の記号 (×・▲・▶) を使わない ([docs/ui.md](../../docs/ui.md#アイコン))。
- Bits UI 2.19.4 が付けないので、`@ugoe/ui` で足しているもの: Tooltip の `role="tooltip"`、Menu・ContextMenu の一覧の `aria-label`。

## テーマ (T0909)

設定の「OS に合わせる・ライト・ダーク」。選んだテーマは `<html data-theme>` に付け、「OS に合わせる」なら属性を外す
(tokens.css が `prefers-color-scheme` で切り替えるので、OS のテーマの変化にもそのまま追従する)。

```ts
// main.ts で mount の前に 1 回。キーは `<アプリ>.theme` (言語の `<アプリ>.locale` と同じ流儀)
import { initTheme } from "@ugoe/ui";
initTheme("ugoe.viewer.theme");
```

```svelte
<ThemeSetting />  <!-- 設定の画面に置く。選ぶとすぐに切り替えて覚える -->
```

- **ちらつき:** module のスクリプトは最初の描画の後に走ることがあるので、アプリの `index.html` の `<head>` に、同じキーを読んで
  `data-theme` を付ける小さなスクリプトを置く (3 つのアプリの `index.html` を見本に)。
- CSS の変数を JS で読むところ (canvas の背景など) は、`$effect` の中で `currentTheme()` を読み、テーマ (設定・OS) が変わったら読み直す。
- 透過の出力など、テーマを効かせない画面では `initTheme()` を呼ばず tokens.css も読まない。

## 文字の選択 (T0911)

- tokens.css を読むと、画面の文字は既定で選択できなくなり、画像とリンクはドラッグで持ち出せなくなる。入力の欄の中は選択できる。
- 利用者に写させたい文字 (ライセンスの全文・エラーの詳細・URL など) にだけ `class="ugoe-selectable"` を付ける。
  `Notice` と Toast のエラー (`danger`) の文は部品の側で選択できる。許している所の一覧は docs/ui.md の「文字の選択」。

## 見た目

- 見た目はトークンだけで決める。部品の CSS は `.ugoe-*` のクラス (`:global`) で、Bits UI の `data-state` などの属性に当てている。

## Bits UI の版

`bits-ui` は `package.json` で版を完全に固定している (`^` を付けない)。更新は人が行う (ADR 0029 の 5):

1. 変更の記録を読む。
2. `package.json` の版を書き換えて `pnpm install`。
3. `pnpm --filter @ugoe/ui test`・`pnpm --filter @ugoe/ui check`・`just licenses` を通す。
   `svelte-toolbelt` の版が変わったら、tarball の `LICENSE` を読んで `scripts/lib/npm-license-exceptions.mjs` の版を書き換える (`scripts/check-licenses.mjs` と `scripts/third-party-notices.mjs` の両方が読む)。
4. `just ui-shots <出力先>` で更新前と後を撮り、見比べる。

## コマンド

```sh
pnpm --filter @ugoe/ui test           # Vitest
pnpm --filter @ugoe/ui check          # svelte-check
pnpm --filter @ugoe/ui gallery        # 見本の画面 (http://localhost:1430/?theme=dark&density=comfortable&lang=ja)
just ui-shots /tmp/ugoe-ui-shots      # 見本の画面を 21 通り撮って検査する
```

`test` と `check` は `just web-check` (`just ci`) に入っている。`ui-shots` は Chrome が要るので CI に入れていない。
部品を使うアプリの画面 (3 つのアプリ) は `just ui-audit` で撮って検査する ([docs/testing.md](../../docs/testing.md#ui-の監査))。部品の見た目を変えたら、両方を流す。
撮るのはテーマ (light・dark) × 密度 (compact・comfortable) × 言語 (en・ja) の 8 通りと、狭い幅 (360px) の 1 枚、
重ねて出す部品 (ダイアログ・確かめのダイアログ・メニュー・Popover・Toast・ツールチップ) を見本の画面の `?open=` で開いた 12 枚 (2 通りずつ)。
動きの途中を撮らないよう、Chrome に `--force-prefers-reduced-motion` を渡す。
それぞれで、描き終わったこと・スクリプトのエラーが無いこと・`<html>` の属性・操作できる部品のすべてに読み上げの名前があること・
描いた形の問題が無いことを確かめる。描いた形は見本の画面が描き終えたときに測る (`gallery/main.ts`。`SegmentedControl` の名前と選択肢が
字の途中で折れていないか、狭い所の `SegmentedControl` がはみ出さずに次の行へ送られたか、`position` を持たない送る枠の中の
`Notice` の見せずに読ませる文字が枠の外を位置の基準にしていないか)。
Chrome は環境変数 `CHROME` で指定できる (無ければ PATH の `google-chrome` / `chromium`)。
