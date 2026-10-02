# 0029: 共有の UI 部品と翻訳 (@ugoe/ui・@ugoe/i18n・@ugoe/player)。翻訳は i18next、部品の土台は Bits UI

- 状態: Accepted (2026-10-02、利用者の確認済み)
- 日付: 2026-10-02
- 関連: T0900、T0901〜T0908、T0663〜T0666、T0723、ADR 0018・0019 (ライセンス)、ADR 0025 (`packages/` と `bindings/` の境界)、ADR 0028 (スタジオ)

## 背景

M8 で、エディタ・ビューア・新しいスタジオ (ADR 0028) が UI の部品と翻訳を共有し、日本語と英語を切り替えられるようにする
([roadmap.md の M8](../roadmap.md#m8-スタジオと-ui-の基盤))。いまの状態は次のとおり。

- **文言:** エディタは英語、ビューアは日本語で、どちらも `.svelte` に直書きしている。共有部品の `@ugoe/face-tracking` だけは
  `strings.ts` に `ja` / `en` の TypeScript のオブジェクト (文字列と、引数を取る関数) を持ち、アプリがどちらかを渡す。
- **見た目:** 両方のアプリの `app.css` が `--bg`・`--panel`・`--accent` などの CSS 変数を持つが、値が違う
  (強調色はエディタが紫 `#6d28d9`、ビューアが青 `#2b6cb0`。文字の大きさは 13px と 15px)。
  ボタン・スライダー・ダイアログは各アプリが別々に作っている (ダイアログはどちらも `<dialog>` を使う)。
- **エラー:** editor-core と Tauri のコマンドは、英語の文 (`thiserror` の `Display`) を返し、画面はそれをそのまま出す。

制約:

- **翻訳の協力者が使える形にする** (利用者の判断)。独自の形式ではなく、翻訳サービス (Weblate・Crowdin など) がそのまま読める、
  広く使われている形式と実装に乗る。サービスの選定と導入は後で行う。
- 対応する言語は日本語と英語から始め、既定は OS の言語にする (利用者の判断)。
- 共有パッケージは `packages/` に置き、GPL-3.0-only にする (ADR 0018・0019・0025。利用者の判断)。依存は GPL-3.0 と両立するライセンスであればよい。
- フロントエンドは Svelte 5 (runes) で、実行するのは Tauri の WebView (Windows は WebView2、macOS は WKWebView、Linux は WebKitGTK)。
  サーバでの描画 (SSR) は無い。
- MCP サーバ・CLI・ログの文言は、エージェントや利用者のスクリプトが読むので英語のままにする。

## 決定

### 1. パッケージの分け方

| パッケージ | 置き場所 | 中身 | 内部の依存 |
| --- | --- | --- | --- |
| `@ugoe/i18n` | `packages/i18n` | 翻訳の実行 (i18next を包む)・言語の検出と切り替え・数値と日付の書式・カタログの型・キーの検査のスクリプト | なし |
| `@ugoe/ui` | `packages/ui` | デザイントークン (CSS 変数)・部品 (Svelte 5)・部品の見本の画面 | `@ugoe/i18n` |
| `@ugoe/player` | `packages/player` | ビューアのモデル再生の層 (読み込み・モーション・表情・トラッキング・ライティング・着せ替え・覚えた設定)。wasm はアプリが渡す | `@ugoe/i18n`・`@ugoe/face-tracking` (パネルを部品に移した後は `@ugoe/ui` も) |

- どれも `private: true`・`"license": "GPL-3.0-only"`・pnpm のワークスペース (`workspace:*`) で、`@ugoe/face-tracking` と同じく
  ビルドせずに `src/index.ts` を `exports` に出す。Svelte は `peerDependencies`。
- 依存の向きは `apps/* → @ugoe/player → @ugoe/ui → @ugoe/i18n` の一方向 (`@ugoe/face-tracking` は `@ugoe/i18n` と `@ugoe/ui` を使ってよい)。`packages/` はアプリを import しない。
  アプリ同士も import しない (ADR 0019)。`@ugoe/face-tracking` も文言を `@ugoe/i18n` に移す (T0901)。
- MIT の `bindings/` (ADR 0025) はこれらを import しない。

### 2. 翻訳の仕組み: i18next と、i18next JSON v4 のカタログ

- **実行は [i18next](https://www.i18next.com/) を `@ugoe/i18n` で包む** (利用者の判断)。 アプリと部品は i18next を直接 import せず、`@ugoe/i18n` の
  `t()`・`setLocale()`・`formatNumber()` などだけを使う。包むのは、Svelte 5 で再描画させるためと、将来ほかの実装に替えられるようにするため。
- **カタログは [i18next JSON v4](https://www.i18next.com/misc/json-format) の形式にする。** 複数形は CLDR の区分の接尾辞 (`_one`・`_other` など)、
  引数は `{{name}}`。これは Weblate (形式 `i18nextv4`) と Crowdin (`i18next_json`) が複数形を含めてそのまま読める。
  ICU MessageFormat の構文 (`i18next-icu`) は使わない。
- **カタログの置き場所:** 各パッケージ・アプリが自分の文言を持つ。`<パッケージ>/locales/<言語>.json` に置き、i18next の名前空間は
  パッケージの短い名前 (`ui`・`player`・`face-tracking`・`editor`・`viewer`・`studio`) にする。全言語をバンドルに含め、HTTP では読まない。
- **再描画:** `@ugoe/i18n` が今の言語を `$state` で持ち、`t()` がそれを読む。言語を切り替えると、`t()` を使う式がすべて再評価される。
- **型:** 原文の言語のカタログから、i18next の `CustomTypeOptions` (型の拡張) でキーと引数を検査する。キーが数千になって
  型の検査が遅くなったら、`enableSelector: "optimize"` を使う ([TypeScript](https://www.i18next.com/overview/typescript))。
- **検査 (`just i18n`、just ci に入れる):** 言語の間のキーの過不足・未使用のキー・引数の食い違いを、自前の小さなスクリプトで検査する (T0901)。
  画面の直書きの文言を見つける検査も入れる (T0908)。公式の `i18next-cli` は依存が大きい (`@swc/core`・`react-i18next` など) ので、いまは使わない。
- **数値・日付・一覧の書式**は `Intl` (`Intl.NumberFormat`・`Intl.DateTimeFormat`・`Intl.ListFormat`) を今の言語で使う。`@ugoe/i18n` が関数を出し、
  カタログの中では i18next の組み込みの書式 (`{{value, number}}`) を使ってよい ([Formatting](https://www.i18next.com/translation-function/formatting))。
- **原文の言語は英語** にする (利用者の判断)。日本語は「常に欠けのない訳」として、`just i18n` で過不足を検査する。英語以外の言語で訳が欠けたら英語を出す。

### 3. 対応する言語

- 日本語 (`ja`) と英語 (`en`)。言語のタグは BCP 47 の言語の部分だけを使う (`ja-JP` → `ja`)。
- **既定は OS の言語:** OS の優先する言語の一覧を前から見て、最初に対応している言語を使う。どれも対応していなければ英語。
  WebView の `navigator.languages` を既定にし、アプリが OS の言語を渡せる口を `@ugoe/i18n` に置く
  (WebView によっては OS の言語と違う値を返すため。取り方は T0901 で確かめる)。
- **設定で上書きできる:** 「OS に合わせる・日本語・English」。言語の名前は、その言語自身で書く。設定はアプリごとに覚える。
  スタジオの出力ウィンドウ・ブラウザソースに文言を出すときは、操作ウィンドウの言語に従う (スタジオの設計は ADR 0028)。
- 言語を足すときは、カタログを足して対応する言語の一覧に加えるだけで済むようにする (コードの分岐を作らない)。

### 4. Rust 側のメッセージ

- **Rust はコードと引数を返し、文は画面で訳す。** 画面に出るエラー・警告・問題 (editor-core の `ProjectError`・`ModuleProblem`、
  Tauri のコマンドのエラー、wasm の読み込みの警告など) は、次の形で渡す。

  ```json
  { "code": "module.base_missing", "args": { "path": "body.ugoeproj" }, "message": "the base project is missing: body.ugoeproj" }
  ```

  - `code`: 安定した識別子 (小文字の `snake_case` を `.` でつなぐ)。**公開の約束として扱い、名前を変えない。** 意味が変わるなら新しいコードを作る。
  - `args`: 文に差し込む値 (文字列・数値)。パスや名前はそのまま渡し、単位や書式は画面で付ける。
  - `message`: 英語の文。ログ・MCP・CLI・開発者向けの表示で使う。
- 画面はカタログの `errors.<code>` を `args` で引く。カタログに無いコード (新しいバックエンドと古い画面の組み合わせなど) は `message` を出す。
- **MCP・CLI・ログの文言は英語のまま**変えない。Rust のクレートに翻訳のカタログは置かない。
- 実装は T0666 (エディタ)。ランタイムの層 (wasm) の警告も、ビューアとスタジオの移行の中で同じ形にそろえる。

### 5. UI の部品は Bits UI を土台にし、見た目は @ugoe/ui が付ける

- **ふるまいの土台は [Bits UI](https://bits-ui.com/) (ヘッドレスの部品集、Svelte 5) にする** (利用者の判断)。キーボード操作・フォーカスの管理・
  WAI-ARIA の属性は Bits UI に任せ、`@ugoe/ui` は Bits UI の部品を包んで、見た目 (デザイントークンの CSS 変数) と ugoe の約束
  (文言を `@ugoe/i18n` から渡す・密度・ショートカットの表記) を付ける。アプリは Bits UI を直接 import せず、`@ugoe/ui` だけを使う
  (Bits UI の破壊的な変更を `@ugoe/ui` の中で吸収するため)。
- **Bits UI に無い部品は、Bits UI の上か HTML の標準の要素の上で自作する。** リセット付きのスライダーと数値の欄 (Bits UI の `Slider` に
  数値の欄とリセットを組み合わせる)・数値の欄・色の選択・ファイルのドロップ領域・知らせ (Notice / Toast)・ショートカットの表記など。
  自作の部品のふるまいは [WAI-ARIA Authoring Practices Guide (APG)](https://www.w3.org/WAI/ARIA/apg/) のパターンに従う。
- 部品の中に固定の文言を持たない。読み上げの名前・閉じるボタンの名前などは `@ugoe/ui` が `@ugoe/i18n` から引いて Bits UI に渡す。
  日付を扱う部品を使うときは、`locale` に今の言語を渡す。
- 見た目は `@ugoe/ui` のデザイントークンで決め、CSS のフレームワーク (Tailwind など) は使わない。Bits UI の部品は `class` と `data-*` 属性で装う。
- **強調色は 3 つのアプリでエディタの紫 (`#6d28d9`) にそろえる** (利用者の判断)。スタジオも紫にし、ビューアの青 (`#2b6cb0`) はやめる。
  ダークのテーマの値とコントラスト比は docs/ui.md の色の表にある。
- 部品の一覧 (どれが Bits UI を元にするか)・トークン・アクセシビリティの基準・文言の書き方は [docs/ui.md](../ui.md) に書く。

**依存とライセンス** (2026-10-02 に `npm view` と tarball で確かめた。すべて GPL-3.0 と両立する):

| パッケージ | 版 | ライセンス | 入る経路 |
| --- | --- | --- | --- |
| `bits-ui` | 2.19.4 | MIT | `@ugoe/ui` の依存 |
| `@floating-ui/dom`・`@floating-ui/core`・`@floating-ui/utils` | 1.8.0・1.8.0・0.2.12 | MIT | bits-ui |
| `tabbable` | 6.5.0 | MIT | bits-ui |
| `runed` (と、その依存の `dequal`・`esm-env`・`lz-string`) | 0.35 系 (bits-ui の範囲。最新は 0.37.1) | MIT | bits-ui・svelte-toolbelt |
| `svelte-toolbelt` (と、その依存の `clsx`・`style-to-object`) | 0.10.6 | MIT (同梱の LICENSE。下の注意) | bits-ui |
| `@internationalized/date` (と、その依存の `@swc/helpers`) | 3.12.4・0.5.23 | Apache-2.0 | bits-ui の peer。日付の部品を使わなくても入れる |

- Apache-2.0 は GPL-3.0 と両立する (GPL-2.0 とは両立しない)。`packages/`・`apps/` は GPL-3.0-only なので問題ない。
- **`svelte-toolbelt` は package.json に `license` の欄が無い** (npm は「Proprietary」と表示する)。tarball の `LICENSE` は MIT
  (Hunter Johnston・Thomas G. Lopes)。`just licenses` の扱いは、**理由と確かめた版を書いた例外の一覧** (パッケージ名・版・
  実際のライセンス・確かめた方法) に入れることを推奨する (クレートの検査にある「SPDX の式を持たない依存 (名前 -> 理由)」の一覧と同じ考え方)。LICENSE のファイルを読んで推し量る方式は、ほかのパッケージで誤って通す
  おそれがあるため、既定の扱いにはしない。版が変わったら例外が外れ、確かめ直すようにする。実装は T0901 (npm の依存の検査を
  `scripts/check-licenses.mjs` に足すとき) に任せる。
- `packages/`・`apps/` の npm の実行時の依存は、GPL-3.0 と両立するライセンス (クレートの検査と同じ `GPL_COMPATIBLE`。Apache-2.0 を含む) に限ることを、
  `just licenses` で検査する (T0901)。

**版の固定と更新:**

- Bits UI は 0.x (Svelte 4) から、2025-02 の 1.x (Svelte 5 での作り直し、1.1.0) を経て、2025-05 の 2.x (2.1.0) と、
  4 か月足らずで 2 回の破壊的な変更があった。2.x の中では 2026-10 まで小さな版が続いている。
- そのため **`bits-ui` は `@ugoe/ui` の `package.json` で版を完全に固定する** (`^` や `~` を付けない)。`pnpm-lock.yaml` もコミットする。
  bits-ui が `^` で受ける依存 (`runed`・`svelte-toolbelt` など) はロックファイルで固定される。
- 更新は Renovate などに任せず、人が行う。小さな版の更新は、変更の記録を読み、部品の Vitest と見本の画面のスクリーンショットの検査
  (T0902・T0903) を通してから入れる。大きな版の更新は、1 つのタスクにして `@ugoe/ui` の中だけを直す (アプリには影響させない)。
- Bits UI の保守が止まったら、`@ugoe/ui` の中で自作か別の部品集に置き換える。アプリが Bits UI を直接使わないので、置き換えは `@ugoe/ui` で閉じる。

### 6. マニュアルの翻訳は M8 の範囲に入れない

- 操作マニュアル (`docs/manual/`) は日本語だけのままにする (利用者の判断)。M8 で翻訳するのは、画面の文言とアプリの中の短い説明 (ツールチップ・空の状態の案内など)。
- 将来の英訳の置き場所は `docs/manual/en/` と決めておく (同じファイル名)。アプリの「ヘルプ」は、今の言語のマニュアルがあればそれを、無ければ日本語を開く。

### 7. 翻訳サービス

- サービスの選定と導入は後で行う (利用者の判断)。条件は「i18next JSON v4 を複数形を含めて読めること」で、
  [Weblate](https://docs.weblate.org/en/latest/formats/i18next.html) と [Crowdin](https://store.crowdin.com/i18next-json) はどちらも満たす。
- カタログは `<パッケージ>/locales/<言語>.json` という決まった場所に置くので、どちらのサービスでも「パッケージごとに 1 つの部品 (component)」として登録できる。

## 検討した代替案

### 翻訳の仕組み

版とライセンスは 2026-10-02 に `npm view` で確かめた。大きさは、最小の使い方を esbuild で minify し gzip した値。

| 案 | 利点 | 欠点 |
| --- | --- | --- |
| **i18next** 26.4.2 (MIT、実行時の依存なし、2026-09 更新) と i18next JSON v4 (採用) | 利用者が最も多い。Weblate (`i18nextv4`) と Crowdin (`i18next_json`) が複数形を含めてそのまま読む。フレームワークに依存しない。型の拡張でキーを検査できる。書式に `Intl` を使う | 約 14 KB と、ほかより大きい。Svelte 用の結び付けは自分で書く (`$state` で数十行)。型の拡張はキーが多いと検査が遅い |
| [Paraglide JS](https://paraglidejs.com/) 2.25.4 (MIT) | メッセージを型付きの関数にコンパイルするので、型と大きさは最良 | `setLocale()` は既定でページを読み直す (`reload: false` で避けられるが、メモリの状態の扱いに注意が要る)。既定の inlang の形式は Weblate や Crowdin がそのまま読めず、結局 i18next の形式のプラグインを使うことになる。コンパイルの手順と依存 (`@inlang/sdk` など) が増える |
| [Fluent](https://projectfluent.org/) (`@fluent/bundle` 0.19.1、Apache-2.0) | 言語ごとに文を自由に分岐でき、表現力が高い。約 4 KB | Weblate は `.ftl` を読める (4.8 から) が、複数形を Weblate の複数形として扱えない。最後のリリースが 2025-04 で、Svelte 用の部品は無い。協力者が Fluent の構文を覚える必要がある |
| ICU MessageFormat ([intl-messageformat](https://formatjs.github.io/docs/intl-messageformat/) 12.1.2、BSD-3-Clause) | Unicode の標準の構文。約 10 KB | Weblate の JSON の形式は ICU の複数形を複数形として扱わない (協力者が `{count, plural, ...}` を直接書く)。Svelte 用の結び付けは自分で書く |
| [svelte-i18n](https://github.com/kaisermann/svelte-i18n) 4.0.1 (MIT、ICU) | Svelte 向けで、すぐ使える | 最後のリリースが 2024-10。runes ではなく store で作られ、作者が作り直しを予告している。古い `intl-messageformat` 10 に依存する |
| いまの `strings.ts` の形 (TypeScript のオブジェクト) を広げる | 依存が無く、型が最も強い | 翻訳サービスが読めず、協力者が TypeScript を書く必要がある (利用者の条件に反する) |

### UI の部品

| 案 | 利点 | 欠点 |
| --- | --- | --- |
| 自前 (標準の要素と APG のパターン) | 依存が無く、版の上げ下げに振り回されない。いまのコード (`<dialog>`) をそのまま育てられる | メニュー・コンボボックス・ツールチップなどのキーボード操作とフォーカスの管理を自分で作り、テストし続ける必要がある |
| **[Bits UI](https://bits-ui.com/) 2.19.4** (MIT、ヘッドレス、Svelte 5。採用) | アクセシビリティ (WAI-ARIA・キーボード・フォーカス) を任せられる。見た目は自由で、CSS 変数のトークンで装える。活発に保守されている | 依存 (`@floating-ui/dom`・`tabbable`・`runed`・`svelte-toolbelt`、peer の `@internationalized/date`) が増える。2025 年に 1.x・2.x と破壊的な変更が続いた (版を固定して吸収する)。数値の欄・色の選択などは無く、自作する |
| [Ark UI](https://ark-ui.com/) / Zag (MIT) | 状態機械による作りで、フレームワークをまたいで使われている | 部品ごとに数十のパッケージに依存する。Svelte 版は React 版より新しい |
| 見た目付きの部品集 (shadcn-svelte・Skeleton・Flowbite Svelte) | すぐに画面が作れる | Tailwind が前提で、CSS 変数のトークンと合わない。見た目がほかのアプリと同じになり、密度の調整が難しい |

### マニュアルの翻訳

| 案 | 利点 | 欠点 |
| --- | --- | --- |
| **M8 では翻訳しない。置き場所の約束だけ決める** (採用) | M8 の範囲が膨らまない。マニュアルを書き換えるたびの二重の手間が無い | 英語の利用者は、画面は英語でもマニュアルは日本語になる |
| M8 で英訳する | 英語の利用者がすぐに使える | エディタとスタジオのマニュアルは量が多く、画面の変更のたびに両方を直す必要がある |

## 結果

- 3 つのアプリが同じ部品と同じ翻訳の仕組みを使い、日本語と英語を切り替えられる。新しい言語は、カタログを足すだけで加えられる。
- 翻訳の協力者は、TypeScript を書かずに翻訳サービス (または JSON) で訳を足せる。サービスの導入は、カタログの場所を登録するだけで済む。
- 画面の文言を足す開発者は、英語のカタログにキーを足し、日本語の訳も同じ PR で足す (`just i18n` が欠けを見つける)。
- バックエンドのエラーはコードで画面に渡るので、訳が付けられる。コードは公開の約束になり、名前を変えにくくなる。
- 込み入った部品のアクセシビリティは Bits UI に任せられる。自作の部品と、組み合わせた画面のアクセシビリティは自分で保つ。基準は docs/ui.md に置き、T0907 で 3 つのアプリを見直す。
- Bits UI の更新は人が行い、`@ugoe/ui` の中で閉じる。版を固定するので、Bits UI の不具合の修正を受け取るのが遅れることがある。
- `scripts/check-licenses.mjs` は、いまは `packages/` とアプリの npm の依存を検査していない。i18next などを入れるときに、
  `packages/`・`apps/` の実行時の依存が GPL-3.0 と両立することも検査に加える (T0901。`svelte-toolbelt` の例外を含む)。

見直すきっかけ:

- i18next の保守が止まる、または翻訳サービスが i18next JSON v4 を読めなくなったとき。
- Unicode MessageFormat 2 (MF2) が翻訳サービスと実装で広く使えるようになったとき (カタログの形式を移すか考える)。
- Bits UI の保守が止まる、または大きな版の更新で `@ugoe/ui` の書き直しが大きくなりすぎるとき (自作か別の部品集に替える)。
- 3 つ目の言語を足すとき (複数形の区分・文字の向きの確認)。
