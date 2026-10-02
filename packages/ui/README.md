# packages/ui

エディタ・ビューア・スタジオで共有する UI の部品とデザイントークン (npm パッケージ `@ugoe/ui`、pnpm のワークスペース)。
方針は [ADR 0029](../../docs/adr/0029-shared-ui-and-i18n.md)、トークンの値・部品の一覧・アクセシビリティの基準は
[docs/ui.md](../../docs/ui.md) にある。T0902 で 1 段目の部品を作った (2 段目は T0903)。

**ライセンスは GPL-3.0-only** (使うアプリと同じ)。ふるまいの土台の [Bits UI](https://bits-ui.com/) とその依存は MIT・Apache-2.0
(`just licenses` が推移的に検査する)。

```
src/tokens.css          デザイントークン (CSS 変数)。ライト・ダーク (data-theme)、compact・comfortable (data-density)
src/index.ts            公開する部品と関数
src/components/         部品 (Button・IconButton・Checkbox・Toggle・Slider・NumberField・Select・SegmentedControl・
                        Tabs・Section・Toolbar)
src/number.ts           数値の欄の解釈・書式・増減 (純粋な関数)
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
- **アプリと、ほかのパッケージは `bits-ui` を直接 import しない。** 部品の props は ugoe の言葉で決めてあり、Bits UI の props は外に出さない
  (Bits UI の破壊的な変更をこのパッケージの中で吸収するため)。
- **見えるラベル・読み上げの名前・選択肢の名前は、翻訳を通した文言を props で渡す。** 部品が既定で出す文言
  (いまは `Slider` のリセットの名前 `slider.reset.aria` だけ) は、このパッケージのカタログ `locales/<言語>.json`
  (@ugoe/i18n の名前空間 `ui`) から引き、props で上書きできる。言語はアプリが @ugoe/i18n の `setLocale()` で決める。
- `IconButton` の `label` は必須で、空だとコンソールにエラーを出す。`title` 属性は付けない (ツールチップは T0903)。
- `Select` の一覧は body に出す (Portal) ので、`data-theme`・`data-density` は `<html>` に付ける。
- 見た目はトークンだけで決める。部品の CSS は `.ugoe-*` のクラス (`:global`) で、Bits UI の `data-state` などの属性に当てている。

## Bits UI の版

`bits-ui` は `package.json` で版を完全に固定している (`^` を付けない)。更新は人が行う (ADR 0029 の 5):

1. 変更の記録を読む。
2. `package.json` の版を書き換えて `pnpm install`。
3. `pnpm --filter @ugoe/ui test`・`pnpm --filter @ugoe/ui check`・`just licenses` を通す。
   `svelte-toolbelt` の版が変わったら、tarball の `LICENSE` を読んで `scripts/check-licenses.mjs` の `NPM_EXCEPTIONS` を書き換える。
4. `just ui-shots <出力先>` で更新前と後を撮り、見比べる。

## コマンド

```sh
pnpm --filter @ugoe/ui test           # Vitest
pnpm --filter @ugoe/ui check          # svelte-check
pnpm --filter @ugoe/ui gallery        # 見本の画面 (http://localhost:1430/?theme=dark&density=comfortable&lang=ja)
just ui-shots /tmp/ugoe-ui-shots      # 見本の画面を 9 通り撮って検査する
```

`test` と `check` は `just web-check` (`just ci`) に入っている。`ui-shots` は Chrome が要るので CI に入れていない。
撮るのはテーマ (light・dark) × 密度 (compact・comfortable) × 言語 (en・ja) の 8 通りと、狭い幅 (360px) の 1 枚。
それぞれで、描き終わったこと・スクリプトのエラーが無いこと・`<html>` の属性・操作できる部品のすべてに読み上げの名前があることを確かめる。
Chrome は環境変数 `CHROME` で指定できる (無ければ PATH の `google-chrome` / `chromium`)。
