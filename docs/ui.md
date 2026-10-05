# UI と文言のガイドライン

エディタ・ビューア・スタジオで共有する UI の部品 (`@ugoe/ui`) と翻訳 (`@ugoe/i18n`) の約束です。
なぜこの形にしたかは [ADR 0029](adr/0029-shared-ui-and-i18n.md) にあります。部品とトークンの実装は T0902・T0903、翻訳の実装は T0901 で行います。
この文書と実装が食い違ったら、気づいた人がどちらかを直してください。

## パッケージ

| パッケージ | 置き場所 | 使う場面 |
| --- | --- | --- |
| `@ugoe/i18n` | `packages/i18n` | 画面に出す文言・数値・日付のすべて |
| `@ugoe/ui` | `packages/ui` | トークン (`tokens.css`) と部品。アプリの `main.ts` で `tokens.css` を 1 回だけ読む |
| `@ugoe/player` | `packages/player` | モデルの再生の層 (ビューアとスタジオ。WebPuppet の寿命 (`PuppetSurface`) とスクリーンショットの PNG への変換はエディタも使う) |

- 依存の向きは `apps/* → @ugoe/player → @ugoe/ui → @ugoe/i18n`。どれも GPL-3.0-only。
- アプリの中にしか無い部品 (エディタのパネルの見出しなど) はアプリに置く。2 つ以上のアプリで要るものになったら `@ugoe/ui` へ移す。

### WebPuppet の寿命

WebPuppet (ugoe-wasm) と、それを描く canvas の寿命 (作る・作り直す・捨てる) は、`@ugoe/player` の `PuppetSurface`
(`packages/player/src/surface.ts`) にまとめ、Player (ビューア・スタジオ) とエディタのビューポート (`ViewportController`) が使う (T0932)。
T0672・T0914 で同じ誤りが両方に入り、別々に直していたため。

- **置き場所:** `@ugoe/player` (GPL-3.0-only)。エディタは T0730 から `@ugoe/player` に依存している。MIT の `bindings/web` の
  `<ugoe-avatar>` は GPL のコードを使えないので、同じ振る舞いを自分で持つ (下の「`<ugoe-avatar>` との関係」)。
- **型の受け方:** WebPuppet の型 (wasm-bindgen の生成物) はアプリごとに違うので、`PuppetSurface<P extends SurfacePuppet>` は使うメソッド
  (`free`・`is_lost`・`resize`) だけの構造の型で受け、`created(puppet)` にはアプリの型のまま渡す。wasm の読み込みも `loadWasm` で受ける。
- **canvas:** `PuppetSurface` が作り、`attach(container)` の要素の先頭に置く。エディタは T0672 まで `generation` を増やして Svelte の
  `{#key}` で canvas を作り直していたが、canvas を作るのを `PuppetSurface` に移したので `generation` は除いた (Viewport.svelte は
  canvas を置く要素を渡すだけ。canvas の見た目は `:global(canvas)`・`canvasClass` で決める)。
- **WebGL2 への切り替え:** `"auto"` で作れなければ新しい canvas で `"webgl2"` を試す (1 つの canvas は WebGPU と WebGL2 を併用できない)。
  一度 WebGL2 に落ちたら、作り直しでも WebGL2 から始める。
- **古い作成の破棄:** `attach`・`detach` のたびに番号を進め、待った後で変わっていたら、遅れてできた WebPuppet と canvas を捨てる
  (`attach` の結果は `"superseded"`)。作り直しの途中で `detach` されたときも同じ。
- **GPU の喪失:** WebGPU のデバイスの喪失と WebGL2 のコンテキストの喪失は、どちらも `WebPuppet.is_lost()` で分かる (WebGL2 は
  ugoe-wasm が canvas の `webglcontextlost` で印を立てる。T0697 を T0932 にまとめた)。描く前に `checkLost()` で確かめ、真なら描かない。
  - **WebGPU:** `checkLost()` が気づいたらすぐ canvas ごと作り直す。
  - **WebGL2 (T0935):** `PuppetSurface` が canvas の `webglcontextlost` を `preventDefault` して、復元 (`webglcontextrestored`) を待ってから
    新しい canvas で作り直す (wgpu の資源は戻らないため)。待っている間は `checkLost()` が真 (描かない) で、作り直しは始めない。
    本物の GPU のリセットでは、復元の前に新しいコンテキストを作れないことがある (T0932 までのすぐ作り直す方式では「描画を始められない」に
    なり得た) ため、`<ugoe-avatar>` (T0425・T0439) に揃えた。復元はイベントで始めるので、要求されたときだけ描くエディタも次に操作するまで
    待たない。
  - 作り直しは一度に 1 つで、捨てた WebPuppet の印と古い canvas のイベントは見ないので、2 つの知らせが重なっても作り直しは 1 回。
  - **続けて失ったとき (T0935):** 前の作り直しから 1 分 (`RESTART_WINDOW_MS`) 以内に失うことが続き、作り直しが `MAX_RESTARTS` (3) 回を
    超えたら (4 回目)、作り直さずに `failed(GpuLostRepeatedlyError)` で知らせる (数と時間は `<ugoe-avatar>` と同じ)。GPU が戻らない間に
    作り直しを繰り返して資源と時間を使い続けないため。次の `attach` で数え直す。画面の知らせは、Player は `status` を `unavailable` にして
    `error` にカタログの `errors.gpuLostRepeatedly` を、エディタはビューポートの `status` を `error` にして `viewport.gpuLostRepeatedly` を出す
    (どちらも「起動し直す (ブラウザでは読み込み直す)」ことを案内する。作り直しのボタンは置かない)。
- **devicePixelRatio:** ResizeObserver は倍率だけの変化を知らせないので、`PuppetSurface` が `matchMedia("(resolution: …dppx)")` で聞き、
  描画バッファの大きさを合わせ直して `pixelRatioChanged` で知らせる。CSS ピクセルと描画バッファの画素の換算には `pixelRatio` を使う。
- **作り直した後の状態:** 背景・光源・モデル・パラメータの値を渡し直すのは使う側 (`created(puppet, recovered)`。値は `lost` で控える)。

`<ugoe-avatar>` との関係: GPU の喪失は同じく `is_lost()` で分かり、devicePixelRatio も同じく `matchMedia` で聞く。WebGL2 の
コンテキストの喪失で復元を待つことと、続けて失ったときに諦める数と時間も同じ (T0935 で揃えた)。違うのは知らせ方で、要素は `error` を送り
`ready` を reject し、`restart()` で作り直せる。

## デザイントークン

見た目の値は、すべて `@ugoe/ui` の CSS 変数 (トークン) から取ります。**部品とアプリの CSS に色の値 (`#...`・`rgb()`) や余白の数値を直に書きません。**
例外は、描画の中身 (ビューポートの格子・ギズモ・光源の印など、モデルの上に重ねる図) の色で、これはトークンの外で決めてよいです。

### 名前と切り替え

- 名前は `--ugoe-<分類>-<名前>` (例: `--ugoe-color-text`・`--ugoe-space-3`)。アプリの古い変数 (`--bg` など) と衝突させないため、接頭辞を付けます。
- **ライトとダーク:** 既定は OS の設定 (`prefers-color-scheme`) に従います。`<html data-theme="light">` / `"dark"` で上書きできます (設定の「OS に合わせる・ライト・ダーク」)。
  3 つのアプリの設定に `@ugoe/ui` の `ThemeSetting` を置き、選んだテーマは `localStorage` の `<アプリ>.theme`
  (`ugoe-editor.theme`・`ugoe-viewer.theme`・`ugoe-studio.theme`) に覚えます (T0909)。
  言語も同じ流儀で、`@ugoe/i18n` の `initLocale(key, OS の言語を取る関数)` に `<アプリ>.locale` のキーを渡し、
  設定の画面は `setLocalePreference()` で切り替えて覚えます (T0925。[packages/i18n/README.md の「言語」](../packages/i18n/README.md#言語))。
  localStorage の読み書きは、テーマ・言語・`@ugoe/player` の覚えるものとも `@ugoe/i18n` の `readStoredItem` などを通し、
  使えない localStorage でも例外を出しません (T0937。[packages/i18n/README.md](../packages/i18n/README.md#localstorage-の読み書き-t0937))。
  起動時のちらつきを避けるため、各アプリの `index.html` の `<head>` で同じキーを読んで先に `data-theme` を付けます。
  CSS の変数を JS で読むところ (エディタのビューポートの背景) は `currentTheme()` で変化を知ります。スタジオの出力ウィンドウには効かせません (透過のまま)。
  `color-scheme` も合わせて切り替え、スクロールバーやフォームの標準の部品の色もそろえます。
- **密度:** `<html data-density="compact">` (エディタ。基準の文字 13px。属性を省いたときもこれ) と `"comfortable"` (ビューア・スタジオ。基準の文字 15px)。
  余白と文字の大きさのトークンは密度によって変わり、部品は密度を意識せずにトークンを使います。

### 色

コントラスト比は WCAG 2.x の式で計算した値です (`bg` / `surface` / `sunken` に対して)。

| トークン | 用途 | ライト | ダーク | 比 (ライト) | 比 (ダーク) |
| --- | --- | --- | --- | --- | --- |
| `--ugoe-color-bg` | 画面の地 | `#f4f4f5` | `#18181b` | | |
| `--ugoe-color-surface` | パネル・ダイアログ・入力欄 | `#ffffff` | `#232327` | | |
| `--ugoe-color-sunken` | ビューポートの地・くぼんだ領域 | `#e4e4e7` | `#111113` | | |
| `--ugoe-color-border` | 区切りの線 (飾り。コントラストの基準の外) | `#d4d4d8` | `#3f3f46` | | |
| `--ugoe-color-control-border` | 入力欄・チェックボックスなどの枠 (部品の境界) | `#7a7a83` | `#7c7c86` | 3.9 / 4.3 / 3.4 | 4.3 / 3.8 / 4.6 |
| `--ugoe-color-text` | 本文 | `#18181b` | `#e4e4e7` | 16.1 / 17.7 / 14.0 | 14.0 / 12.3 / 14.9 |
| `--ugoe-color-text-muted` | 補足・単位・無効でない控えめな文字 | `#5f5f68` | `#a1a1aa` | 5.8 / 6.3 / 5.0 | 6.9 / 6.1 / 7.4 |
| `--ugoe-color-accent` | 主な操作・選択・スライダー・リンク | `#6d28d9` | `#a78bfa` | 6.5 / 7.1 / 5.6 | 6.5 / 5.8 / 6.9 |
| `--ugoe-color-on-accent` | 強調色の上の文字 | `#ffffff` | `#18181b` | 7.1 (強調色に対して) | 6.5 (強調色に対して) |
| `--ugoe-color-focus` | フォーカスの輪 | 強調色と同じ | 強調色と同じ | | |
| `--ugoe-color-danger` | エラー・取り消せない操作 | `#c81e1e` | `#f87171` | 5.2 / 5.7 / 4.5 | 6.4 / 5.7 / 6.8 |
| `--ugoe-color-warning` | 警告 | `#a14a06` | `#fbbf24` | 5.5 / 6.0 / 4.7 | 10.6 / 9.4 / 11.3 |
| `--ugoe-color-success` | 成功・接続中 | `#126c34` | `#4ade80` | 5.9 / 6.5 / 5.1 | 10.2 / 9.0 / 10.8 |
| `--ugoe-color-backdrop` | モーダルの後ろ | `rgb(0 0 0 / 40%)` | `rgb(0 0 0 / 55%)` | | |

- 状態の色 (`danger`・`warning`・`success`) は、文字と印 (アイコン) に使います。地に敷くときは、それぞれの `-soft` (地の色に 12% 混ぜた色。`color-mix()`) を使い、文字は `--ugoe-color-text` のままにします。
- 強調色は 3 つのアプリ (エディタ・ビューア・スタジオ) でエディタの紫にそろえます。ビューアの青は使いません (利用者の判断。ADR 0029 の 5)。
- 前のエディタの `--muted` (`#71717a`) は、`--bg` の上で 4.4 で基準 (4.5) に届かなかったので、`--ugoe-color-text-muted` に置き換えました。
- 文字を薄く見せるのに `opacity` を使いません (地との比が下がって 4.5 に届かない)。控えめにするなら `--ugoe-color-text-muted` にします (T0907)。

### 余白

4px を単位にします。`compact` と `comfortable` で同じ名前のまま値が変わります。

| トークン | compact | comfortable | 主な用途 |
| --- | --- | --- | --- |
| `--ugoe-space-1` | 2px | 2px | アイコンと文字の間 |
| `--ugoe-space-2` | 4px | 6px | 部品の中の詰めた余白・行の間 |
| `--ugoe-space-3` | 8px | 10px | ボタンの左右・部品の間 |
| `--ugoe-space-4` | 12px | 16px | パネルの内側・段落の間 |
| `--ugoe-space-5` | 16px | 20px | ダイアログの内側 |
| `--ugoe-space-6` | 24px | 32px | 大きな区切り |
| `--ugoe-control-height` | 24px | 30px | ボタン・入力欄・Select の高さ |

### 文字

| トークン | compact | comfortable | 用途 |
| --- | --- | --- | --- |
| `--ugoe-font-size-xs` | 11px | 12px | 見出しの小さな大文字・単位 |
| `--ugoe-font-size-sm` | 12px | 13px | 補足 |
| `--ugoe-font-size-md` | 13px | 15px | 本文・部品 (基準) |
| `--ugoe-font-size-lg` | 15px | 17px | ダイアログの見出し |
| `--ugoe-font-size-xl` | 18px | 21px | 画面の見出し |

- `--ugoe-font-sans`: `system-ui, -apple-system, "Segoe UI", "Hiragino Sans", "Noto Sans JP", sans-serif`
- `--ugoe-font-mono`: `ui-monospace, "Cascadia Mono", "SF Mono", "Noto Sans Mono", monospace` (数値の欄・パス・コード)
- `--ugoe-line-height`: 1.4 (本文)。部品の 1 行の中は 1.2。
- 太さは 400 と 600 の 2 つだけ使います。数値が並ぶ欄には `font-variant-numeric: tabular-nums` を付けます。
- 日本語では `text-transform: uppercase` と字間の指定が効かない・不自然になるので、見出しの装飾は言語に依らない形 (色と大きさ) で付けます。

### 角丸・影・重なり・動き

| トークン | 値 | 用途 |
| --- | --- | --- |
| `--ugoe-radius-sm` | 3px | チェックボックス・小さなボタン |
| `--ugoe-radius-md` | 4px | ボタン・入力欄・Select |
| `--ugoe-radius-lg` | 8px | ダイアログ・メニュー・知らせ |
| `--ugoe-radius-full` | 999px | トグル・バッジ |
| `--ugoe-shadow-overlay` | `0 4px 16px rgb(0 0 0 / 20%)` (ダークは 50%) | メニュー・ツールチップ・ダイアログ |
| `--ugoe-z-popover` / `--ugoe-z-toast` | 100 / 200 | `popover` と `<dialog>` は最上位の層 (top layer) に出るので、z-index を使うのは知らせの帯などだけ |
| `--ugoe-duration-fast` / `--ugoe-duration-normal` | 100ms / 200ms | 開閉・ホバー。`prefers-reduced-motion: reduce` では 0ms |
| `--ugoe-focus-ring` | `2px solid var(--ugoe-color-focus)`、`outline-offset: 2px` | `:focus-visible` |

### いまの変数からの移し方

| いまの変数 (エディタ・ビューア) | トークン |
| --- | --- |
| `--bg` | `--ugoe-color-bg` |
| `--panel` | `--ugoe-color-surface` |
| `--viewport-bg` | `--ugoe-color-sunken` |
| `--border` | `--ugoe-color-border` (部品の枠なら `--ugoe-color-control-border`) |
| `--text` / `--muted` | `--ugoe-color-text` / `--ugoe-color-text-muted` |
| `--accent` | `--ugoe-color-accent` |
| `--danger` / `--warning`、直書きの `#dc2626` など | `--ugoe-color-danger` / `--ugoe-color-warning` |

移行の途中は、アプリの `app.css` で古い変数をトークンへの別名 (`--bg: var(--ugoe-color-bg)`) にしていました。T0907 で 3 つのアプリと `@ugoe/face-tracking` を
トークンへ移し、別名を消しました。新しいコードは古い変数を使いません。

## 部品の一覧

`@ugoe/ui` の部品は、ふるまいの土台を [Bits UI](https://bits-ui.com/) 2.x (ヘッドレス) に任せ、見た目をトークンで付けます ([ADR 0029](adr/0029-shared-ui-and-i18n.md) の 5)。
**アプリと、`@ugoe/ui` 以外のパッケージは Bits UI を直接 import せず、`@ugoe/ui` の部品を使います。** Bits UI に無い部品は、Bits UI の部品の組み合わせか
HTML の標準の要素の上に自作し、ふるまいは [WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/) のパターンに従います。

土台の列は元にするもの (Bits UI の部品名、または自作の元にする要素)。キーボードの列は部品にフォーカスがあるときの操作で、Bits UI の部品はその実装に従います。
段は実装するタスク (T0902 = 1 段目、T0903 = 2 段目)。

| 部品 | 使う場面 | 土台 | キーボード | 段 |
| --- | --- | --- | --- | --- |
| `Button` | 操作を 1 つ実行する。種類は `primary` (画面に 1 つまで)・`secondary` (既定)・`danger`・`ghost` (ツールバーの中) | Bits UI `Button` | Enter・Space | 1 |
| `IconButton` | 文字を出す場所が無いときの操作。**`label` (読み上げとツールチップに使う) を必ず渡す** | Bits UI `Button` と `Tooltip` | Enter・Space | 1 |
| `Checkbox` | 決定 (保存・OK) を押すまで効かない選択肢、または一覧の中の選択 | Bits UI `Checkbox` | Space | 1 |
| `Toggle` | すぐに効く入・切 (「鏡像で表示」など)。説明は `tooltip` (スイッチに Tooltip を付ける) | Bits UI `Switch` | Space・Enter | 1 |
| `Slider` | 範囲のある数値。`NumberField` とリセット (既定値に戻す) を並べて出せる (この組み合わせは自作) | Bits UI `Slider` | 矢印・PageUp/PageDown・Home/End | 1 |
| `NumberField` | 数値を正確に入れる。上下の矢印で増減、Shift で 10 倍。単位を後ろに出す | 自作 (`<input type="text" inputmode="decimal">`、APG の Spinbutton) | 上下の矢印・Enter で確定・Esc で戻す | 1 |
| `Select` | 5 個以上の選択肢から 1 つ | Bits UI `Select` | 上下の矢印・Enter・頭文字で移る | 1 |
| `SegmentedControl` | 2〜4 個の選択肢から 1 つ (すべて見せたいとき) | Bits UI `RadioGroup` (または `ToggleGroup` の単一選択) | 矢印で移り、選ぶ | 1 |
| `Tabs` | 同じ場所の中身を切り替える (中身が別の画面のとき) | Bits UI `Tabs` | 左右の矢印・Home/End | 1 |
| `Section` | 折りたためるまとまり (パネルの中) | Bits UI `Collapsible` | Enter・Space | 1 |
| `Toolbar` | ボタンの並び。中は 1 つのタブ止まりにする | Bits UI `Toolbar` | 左右の矢印 (roving tabindex) | 1 |
| `Dialog` | 作業を止めて確認・入力させる (モーダル)。取り消せない操作の確認は `AlertDialog` | Bits UI `Dialog`・`AlertDialog` | Esc で閉じる・中でフォーカスが回る・閉じたら開いた元にフォーカスを戻す | 2 |
| `Notice` | 画面に残る知らせ (読み込みの警告など)。`ondismiss` を渡すと閉じるボタンを出す | 自作 (`role="status"`。エラーは `role="alert"`) | 閉じるボタンに届く | 2 |
| `Toast` | すぐ消えてよい知らせ (「保存しました」)。操作を含むなら `Notice` にする。`showToast()` で出し、アプリの根元の `Toaster` が描く | 自作 (`role="status"`、`popover="manual"`) | フォーカスを奪わない。ポインタやフォーカスが乗っている間は消えない | 2 |
| `Tooltip` | ボタンやアイコンの短い説明。`IconButton` は `label` を既定で出す。根元に `TooltipProvider` を置くと、続けて出すときに待たない | Bits UI `Tooltip` | フォーカスでも出る・Esc で消える | 2 |
| `Menu` / `ContextMenu` | 操作の一覧 (ファイルのメニュー・「その他」のボタン / 右クリック)。選択肢を選ぶなら `Select`。項目にショートカットを出せる (`aria-keyshortcuts` も付く) | Bits UI `DropdownMenu`・`ContextMenu` (メニューバー (`Menubar`) は使う画面ができたときに足す) | 上下の矢印・Enter・Esc・頭文字で移る | 2 |
| `Popover` | 部品の近くに出す小さな編集の枠 (色の選択など) | Bits UI `Popover` | Esc で閉じ、開いた元に戻る | 2 |
| `ColorPicker` | 光源などの色 | 自作 (`Popover` の中に `<input type="color">` と `NumberField`) | 標準の操作 | 2 |
| `DropZone` | ファイルを落とす領域。**同じことができるボタン (「ファイル…」) を必ず並べる** | 自作 (`<div>` と `Button`) | ボタンで開く | 2 |
| `Kbd` | ショートカットの表記 (macOS は ⌘、ほかは Ctrl) | 自作 (`<kbd>`) | | 2 |
| `ThemeSetting` | テーマの設定 (OS に合わせる・ライト・ダーク)。選ぶとすぐに切り替えて覚える | 自作 (`SegmentedControl`) | 矢印で移り、選ぶ | T0909 |

Bits UI を包むときの約束:

- Bits UI の部品の `class` と `data-*` 属性 (`data-state`・`data-disabled` など) にトークンを当てて装います。Bits UI の既定の見た目には頼りません (持たない)。
- 読み上げの名前・閉じるボタンの名前など、部品の中の文言は `@ugoe/i18n` から引いて渡します。日付を扱う部品を使うときは `locale` に今の言語を渡します。
- `@ugoe/ui` の部品の props は ugoe の言葉で決め、Bits UI の props をそのまま外に出しません (Bits UI の破壊的な変更を `@ugoe/ui` の中で吸収するため)。
- 読み上げの名前と説明の props は、HTML の属性と同じ名前 (`aria-label`・`aria-labelledby`・`aria-describedby`) で受けます。
  `Button`・`IconButton` はトリガーの属性を広げて受けるので、ほかの部品もこれに合わせます (T0920。`ariaLabel`・`labelledby`・`describedby` とは書かない)。
- トリガーを呼び手が描く部品 (`Tooltip`・`Popover`・`Menu`) は、そのスニペットを `trigger` と呼び、受け取った属性をフォーカスを受ける要素に広げます。
- `index.ts` は部品と、アプリやほかのパッケージが使う関数・型だけを名前で出します (`export *` は使わない)。部品の内部の関数
  (数値の欄の解釈・ファイルの選り分け・テーマの読み書き・Toast の列など) は出さず、アプリで要るものになったら名前を足します。
- Bits UI の版は `@ugoe/ui` で完全に固定し、人が更新します (ADR 0029 の 5)。

使い分けの約束:

- **`title` 属性で説明を出さない**のを基本にし、`Tooltip` を使います (`title` はキーボードで出せず、タッチでも出ない)。
  T0907 の見直しの後に残している `title` は、フォーカスを受けない要素の補い (省略した名前の全体・ID・一覧の行の印) だけです。
  同じことを見える文や読み上げの名前で伝えていて、`title` が無くても操作に困らないものに限ります。
- **チェックボックスとトグル:** 押した瞬間に効くならトグル、決定のボタンを押すまで効かないならチェックボックス。
- **`Dialog` と `Notice`:** 利用者の判断が要らない知らせにダイアログを使いません。
- **取り消せない操作** (すべて忘れる・上書き保存の前の確認など) は `danger` のボタンにし、確かめのダイアログを出すか、Undo を用意します。確かめのダイアログでは、既定のフォーカスを取り消し側に置きます。
- **無効のボタン**は、なぜ押せないかを近くに書くか、`Tooltip` で出します (無効のボタンはフォーカスを受けないので、説明は `aria-describedby` で別の要素から付ける)。
- パネルの見出し・行のレイアウトなど、エディタの中の共通の形は下の「エディタの中の共通の形」にあります。

## エディタの中の共通の形

エディタのパネルとダイアログを `@ugoe/ui` に移すとき (T0663・T0664・T0665) にそろえる形です。エディタの中にしか無いものは
`apps/editor/src/lib/ui/` と `apps/editor/src/app.css` に置き、2 つ以上のアプリで要るものになったら `@ugoe/ui` へ移します。

| 形 | 使うもの | 約束 |
| --- | --- | --- |
| パネルの見出し | `<h2>` (`app.css`) | `--ugoe-font-size-sm`・太さ 600・`--ugoe-color-text-muted`、上 `--ugoe-space-4`・下 `--ugoe-space-2`。大文字化と字間は使わない (日本語で効かない)。パネルの中のまとまりは `<h3>` (`app.css`。`--ugoe-font-size-md`・600・本文の色)。折りたためるまとまりは `Section` |
| 行 (欄の名前と部品) | `.ed-field` (`app.css`) | 2 列の格子 (名前 `minmax(5em, 2fr)`・部品 `3fr`)、行の間 `--ugoe-space-2`。名前は折り返してよく、固定の幅を付けない (英語は長くなる)。名前は `<label for>` か部品の `label`。単位は部品の後ろ (`NumberField` の `unit`)。名前を見せる部品 (`Slider`・`Checkbox`・`Toggle`・`Select` の `label`) は行に入れずにそのまま置く |
| ボタンの並び | `.ed-buttons` (`app.css`) | 横に並べて折り返す (間 `--ugoe-space-2`)。決定は右端ではなく並びの最後 (パネルの中は左寄せ)。押せない理由は近くに書く |
| 補足・空の状態 | `.ed-hint` (`app.css`) | `--ugoe-font-size-sm`・`--ugoe-color-text-muted`。空の状態は「何が無いか」と「次に何をするか」を 1 文で |
| 説明付きのボタン | `TipButton` (`lib/ui/TipButton.svelte`) | `title` 属性の代わりに `Tooltip`。ショートカットがあれば `shortcut` (`Mod+Z`。実際に効くものだけ) |
| タブの並び | `TabStrip` (`lib/ui/TabStrip.svelte`) / `Tabs` | 中身を部品が持てるなら `Tabs`。右の列のように、タブの状態が別々の表示の状態に分かれているときは `TabStrip` (APG の Tabs。矢印・Home・End、説明は Tooltip、見てほしい変化は `attention` で色・太さ・印と読み上げ) |
| 失敗・知らせ | `Notice` / `showToast` | 操作の失敗はその場所に `Notice` (`danger`。閉じられる)。結果 (「保存しました」) は Toast。ヘッダーの操作の失敗はヘッダーの下の知らせの帯 (`FileMenu` の `.bands`) |
| 本体のもの (モジュールのプロジェクト) | `BaseLock` | 欄をまとめて無効にし、直す場所を `role="note"` の文で示す。`fieldset` の `disabled` は `<input>`・`<button>` にしか効かないので、`Slider` など中にそれらを持たない部品には `useBaseLock()` (`lib/ui/baseLock.ts`) の値を `disabled` に渡す |
| 確かめ | `AlertDialog` | 取り消せない操作・保存していない変更を捨てる操作。同じボタンを 2 度押させる形にしない |
| ダイアログ | `Dialog` | 見出しは `title`、説明は `description`、決定・取り消しは `actions`。Esc で閉じるときの意味 (「後で」など) を `onclose` で決める |
| 文字の入力欄 | `<input type="text">` | `@ugoe/ui` に部品がまだ無い。ダイアログの中は `app.css` の `.ugoe-dialog input[type="text"]` で装う。名前は `<label for>` |
| 要素の名前 | `src/lib/labels.ts` | `顔 (Layer8)` の形 (名前と ID)。ID・パス・パラメータの名前は訳さない |

- **文言の置き場所:** エディタのカタログ (`apps/editor/locales/<言語>.json`、名前空間 `editor`) のキーの最初の段は、画面の部分 (`app`・`fileMenu`・
  `moduleSettings` など) か、パネルの名前 (`inspector`・`physics`・`timeline` など) にします。部品から作る文 (`moduleProject.ts` など) は
  関数にして、呼ぶたびに `t()` を通します (モジュールの読み込み時に 1 度だけ評価した定数は言語の切り替えに追従しない)。
- **`<dialog>` と body に出す部品:** `showModal()` の `<dialog>` は最上位の層 (top layer) に出るので、その中で `Select`・`Popover`・`Menu`
  (body に出す) を使うと一覧が後ろに隠れます。ダイアログを `@ugoe/ui` の `Dialog` に移してから使います。移すまでは `SegmentedControl` などの
  body に出さない部品にします。
- **重なるダイアログ:** `Dialog` の上に `Dialog`・`AlertDialog` を開いてよいです (設定の中の「接続を確かめる」から送る前の確認を開くなど。T0665)。
  Bits UI のフォーカスの閉じ込めと Esc・外のクリックの層は全体で重なった順に積まれるので、上のものだけが反応し、閉じると下に戻ります。
  ただし、開いた元のボタンを待つ間に無効にすると、閉じたときにフォーカスを戻せないので、押せるようになってから戻します (`SettingsDialog` の「接続を確かめる」)。

### 数値の部品と onchange

`Slider` と `NumberField` (と `Slider` の中の数値の欄) は、**利用者の操作のときだけ** `onchange` (と `Slider` の `oncommit`) を呼びます (T0910)。

- 利用者の操作は、つまみのドラッグ・キー (矢印・PageUp / PageDown・Home / End)・欄に入れて確定 (Enter・フォーカスを外す)・リセットのボタンです。
- 外から渡した `value` では呼びません。刻み (`step`) の格子や `decimals` の桁に乗らない値 (指揮役が送る呼吸の値など) を渡すと、
  つまみは格子に寄せて、欄は丸めて**表示だけ**します。渡した値はそのまま残り、寄せた値を書き戻しません。
- 数値の欄は、書き換えていなければ確定しません (フォーカスを外す・Enter だけでは呼ばない)。書き換えていない間は、フォーカスがあっても外からの値に追従し、
  書き換えの途中は外からの値で欄を上書きしません。
- そのため、毎フレーム変わる値を映すだけのスライダーでも、利用側で「変化が小さければ送らない」などの回避は要りません。
- **確定の印:** `NumberField` は矢印キー・PageUp / PageDown の 1 回ごとに `onchange` を呼びます (押し続けると何度も呼ぶ)。
  呼んでいる間だけ、`numberFieldCommit()` が欄ごとに一意な印 (文字列) を返します (それ以外では `null`)。受け手はこれで同じ欄の続けての確定を
  見分けてまとめられます。エディタは 1 つの Undo にまとめます ([editor.md#undo](editor.md#undo)、T0695)。`onchange` の中で `await` した後は
  印が無いので、読むなら同期的に読みます。ビューア・スタジオは印を読まず、振る舞いは変わりません。

## 文字の選択

デスクトップアプリらしく、画面の文字は**既定で選択できません** (T0911)。ドラッグのはずみに文字が選択されて操作の邪魔をしないためです。
`@ugoe/ui` の `tokens.css` の末尾で、`<html>` に `user-select: none` (WebKitGTK・WKWebView 向けの `-webkit-user-select` も) を付けています。
スタジオの出力ウィンドウ (`apps/studio/output.html`) は `tokens.css` を読まないので、同じ指定を `<style>` に書いています。

- **入力の欄の中は選択できます:** 文字の `<input>` (ボタン・チェックボックス・スライダーなどの型を除く)・`<textarea>`・`contenteditable`。
  `NumberField` や `Slider` の数値の欄も中身は `<input>` なので、そのまま選択・コピーできます。
- **選択を許すときは `.ugoe-selectable` を付けます。** 付けた要素とその中が選択できます。CSS に `user-select: text` を直に書かず、このクラスを使います
  (印を探せば例外の一覧が分かるように)。許すのは、利用者が写したい文字だけにします。**エラーの文は選択できるようにします**
  (利用者の判断。報告や検索に写せるように)。新しくエラーの文を出すときは、`Notice` / Toast の `danger` を使うか、`.ugoe-selectable` を付けます。いま許している所:

| アプリ | 場所 | 理由 |
| --- | --- | --- |
| `@ugoe/ui` | `Notice` と Toast のうちエラー (`danger`) の見出しと本文 (部品の側で付ける) | エラーの文を報告に貼る |
| エディタ・ビューア・スタジオ | エラーの文 (`.error`・ビューアの `player.error`・スタジオの `.studio__error` など、失敗を出す文のすべて) | エラーの文を報告に貼る |
| エディタ・ビューア・スタジオ | ライセンスのダイアログの全文 (`LicenseDialog` の `<pre>`) | 引用・転記 |
| エディタ | モジュールの問題の帯の「詳細」(`ModuleProblemBar`)・着せ替えの問題の詳細 (`DressUpSection`)・アバターの検査のエラーの一覧 (`AvatarSection`) | エラーの詳細を報告に貼る |
| エディタ | マニュアルを開けなかったときの知らせ (`HelpButton`) | URL を写してブラウザで開く |
| エディタ | エージェントの接続の手順の設定の例 (`ConnectAgents` の `<pre>`) | 設定のファイルに貼る |
| スタジオ | 操作の API の URL (`ApiPanel`) | 外のツールに貼る (コピーのボタンもある) |
| スタジオ | 仮想カメラの失敗の文と詳細、入れ方のコマンド (`VirtualCameraPanel`) | エラーの詳細を報告に貼る・端末に貼る |
| スタジオ | ホットキーのエラー (`warning` の `Notice` の本文) | エラーの文を報告に貼る |
| ビューア・スタジオ | トラッキングのパネルのエラーの文 (`@ugoe/face-tracking` の `TrackingPanel`) | エラーの文を報告に貼る |

- **画像とリンクはドラッグで持ち出せません** (`-webkit-user-drag: none`)。ファイルのドロップ (`DropZone`)、
  ポインタのイベントで作るドラッグ (ギズモ・スライダー・タイムライン・ツリーとポーズの並べ替え) には影響しません。
  エディタは Tauri のファイルのドロップを有効にしているので、Windows (WebView2) では HTML5 のドラッグ & ドロップ (`draggable`) が
  届きません。要素の並べ替えはポインタのイベントで作ります (T0696)。
- 指定は `:where()` で詳細度を 0 にしているので、部品やアプリの CSS で上書きできます。

## UI の品質の基準

M10 (UI の品質) のタスクが同じ物差しで画面を見直すための約束です (T0915)。上のトークン・部品・「エディタの中の共通の形」は「何を使うか」、
ここは「画面をどう組むか」と「どう確かめるか」です。機械で見つけられるものは `just ui-audit` ([docs/testing.md](testing.md#ui-の監査)) が数え、
残りは撮った画像を人が見ます。各項の **確かめ方** に、どちらで見るかを書きます。

### 配置

- **情報の優先度:** 画面ごとに「いちばんよく使う操作」を決め、それを最も目に入る位置 (左上・各パネルの頭) に置きます。`primary` のボタンは画面に 1 つまで。
  まれな操作 (書き出しの設定・取り消せない操作・開発者向けの情報) は、メニュー・設定・折りたたんだ `Section` の奥に置きます。
  利用者が知っているべき状態 (保存していない・配信中・トラッキング中・エラー) は、送らずに見える位置に、文と印で出します。
  確かめ方: 人 (広い幅と狭い幅の画像で、送らずに見えるものを並べ、優先度と合っているか)。
- **揃え:** 文字は左揃え、数値は右揃えで `tabular-nums`。1 つのパネルの中では、欄の名前の列と部品の列の左端をそろえます (`.ed-field` の格子)。
  ボタンの並びの決定は並びの最後 (「エディタの中の共通の形」)。アイコンと文字は中心の線でそろえます。
  確かめ方: 人 (画像に縦の線を当てて見る)。
- **余白の段:** 余白はトークン (`--ugoe-space-*`) の段だけを使い、関係の近いもの同士ほど狭くします: 部品の中・ラベルと部品は `space-1`〜`space-2`、
  部品の間は `space-2`〜`space-3`、まとまりの間は `space-4`、パネルの内側とダイアログは `space-4`〜`space-5`。まとまりの中の余白を、
  まとまりの間の余白より広くしません (どれとどれが仲間か分からなくなる)。
  確かめ方: 人。直書きの余白は T0916 で探す。
- **グループの見せ方:** まとまりは、まず見出しと余白で示します。枠 (`fieldset`・カード) は、意味のある組 (位置・大きさ・回転のように一緒に変える欄、
  ラジオの組) だけに使い、枠の中に枠を重ねません。区切りの線は、余白だけでは分からない所 (ツールバーと中身の境など) に 1 本だけ引きます。
  折りたためるまとまり (`Section`) は、閉じても何が入っているかが見出しで分かるようにし、閉じた状態の要約 (数・状態) を見出しの右に出してよいです。
  確かめ方: 人。

### レイアウト

- **幅の変化:** 対象の幅は、エディタ 1024px〜、ビューアとスタジオ 360px〜です (「アクセシビリティの基準」の「大きさの変化」)。主な領域 (ビューポート・
  シーンの一覧) を最も広くし、横のパネルには最小と最大の幅を付けます。幅が足りないときは、まず折り返し (ボタンの並び)、次に列ごと縦に送る並び
  (ビューア・スタジオの狭い幅)、最後に省略記号 (名前。全体は Tooltip か `title`) の順で逃がします。ボタン・見出し・欄の名前に固定の幅を付けません
  (英語は日本語より長い)。
  確かめ方: `just ui-audit` (横のはみ出し・文字の重なりと切れ。広い幅と狭い幅) と人。
- **高さの変化:** 低い画面 (エディタ 720px) でも主な操作が見えるようにします。送るのはパネルの中で、パネルの頭 (見出し・タブ・道具の並び) は送らずに残します。
  ビューアとスタジオの狭い幅だけは、画面の全体を縦に送ります。
  確かめ方: 人 (`-bottom` の場面で、送った先が見えること)。
- **送り:** 1 つの領域に縦の送りは 1 つにします (送る領域の中に、また縦に送る領域を入れない)。横の送りは表とタイムラインだけにします。
  送れることが分かるように、領域の端で行を途中で切って見せるか、見出しを固定 (`position: sticky`) にします。選んだもの・フォーカスしたものは、見える所まで送ります。
  確かめ方: 人。Tab で送り先に隠れたまま止まらないことは `just ui-audit` (Tab の止まりの「画面の外」)。
- **分割:** 大きさを変えられる境 (`role="separator"`) は、ポインタとキーボード (矢印・Home・End) の両方で動かせ、フォーカスの印を出し、
  変えた大きさを覚えます。最小の大きさで中身が潰れない (見出しと 1 行が見える) ようにします。
  確かめ方: `just ui-audit` (Tab の止まりの印) と人。

### デザイン

- **トークン:** 色・余白・文字・角丸・影は上の「デザイントークン」から取ります。文字を薄く見せるのに `opacity` を使いません。新しい色はコントラスト比を計算して表に足します。
  確かめ方: `just ui-audit` (コントラスト) と、直書きの検索 (T0916)。
- **アイコン:** アイコンだけのボタンは `IconButton` (読み上げの名前とツールチップ) にします。意味を伝えるアイコンは地に 3:1 以上にし、色だけで違いを伝えません。
  文字の記号 (◆・▾・▶) と絵のアイコンを、同じ並びの中で混ぜません。大きさ・線の太さ・揃えの段は T0916 で決めます。
  確かめ方: `just ui-audit` (名前) と人。
- **状態の見せ方:** 部品の状態は次の形で示し、どれも色だけに頼りません。

  | 状態 | 見せ方 |
  | --- | --- |
  | 通常 | トークンの地と枠 |
  | hover | 地の色を一段変える (枠と文字は変えない)。押せないものには付けない |
  | 押下 | hover よりもう一段濃い地 |
  | フォーカス | `--ugoe-focus-ring` (`:focus-visible`)。外側の枠に出す部品は `:focus-within` |
  | 選択 | 強調色の地か枠に加えて、形 (チェックの印・左の帯・太字) |
  | 無効 | 控えめな文字の色 (コントラストの基準の外)。理由を近くに書くか Tooltip で出す |
  | エラー | 危険の色の枠と印と文 (`aria-invalid`・`aria-describedby` で文を結ぶ) |
  | 読み込み中・実行中 | その場所に文 (「読み込んでいます…」) と、長いものは進み具合と取り消し。押すと二重になる操作は無効にする |

  確かめ方: 見本の画面 (`pnpm --filter @ugoe/ui gallery`・`just ui-shots`) で部品ごとに、アプリの画面は人。
- **空の状態:** 何も無い領域 (プロジェクトなし・パーツなし・モデルなし・シーンなし) には、「何が無いか」と「次に何をするか」を 1 文で書き、
  次の操作のボタンをその場に置きます (文でメニューの場所を教えるだけにしない)。
  確かめ方: `just ui-audit` の `-empty` の場面を人が見る。
- **読み込み中:** 0.5 秒より長くかかりそうな処理は、始めた場所に「何をしているか」を出します。数秒を超えるもの (AI の送信・書き出し・PSD の取り込み) は
  進み具合か経った時間と、取り消しを出します。待っている間も、ほかの領域の操作を止めません (止めるならモーダルで理由を出す)。回る印は
  `prefers-reduced-motion: reduce` で止め、文だけでも分かるようにします。
  確かめ方: 人 (偽のホストでは待ちを作れない。実機か e2e)。
- **エラー:** 失敗は起きた場所に出します (操作の失敗は `Notice` の `danger`、欄の入力の誤りは欄の下の文)。「何が起きたか」と「どうすればよいか」を
  利用者の言葉で書き、内部のメッセージ (英語のコード・パス) は詳細に回します。エラーを Toast だけで出しません (消えて読み返せない)。
  エラーの文は選択できるようにします (「文字の選択」)。
  確かめ方: `just ui-audit` (画面に出たエラーを数える。見本で出るはずのないエラーを見つける) と人。

### アクセシビリティ

基準は下の「アクセシビリティの基準」です。T0915 で、その確かめ方 (スクリーンリーダー・200% の拡大・reduced motion) を [確かめ方](#確かめ方) に足しました。

## アクセシビリティの基準

目標は [WCAG 2.2](https://www.w3.org/TR/WCAG22/) の AA に当たる水準です。ビューポート (モデルの描画) そのものは対象の外ですが、ビューポートで行う操作には、パネルやキーボードでの代わりの手段を用意します。

### キーボード

- **マウスでできる主な操作は、キーボードだけでもできる**ようにします。Tab の順は見た目の順 (左から右、上から下) に合わせ、`tabindex` に正の値を使いません。
- 複数の部分からなる部品 (`Tabs`・`Toolbar`・`Menu`・`SegmentedControl`) は、部品の全体で 1 つのタブ止まりにし、中は矢印で移ります (roving tabindex)。
- フォーカスを閉じ込めるのはモーダルの `Dialog` だけです。それ以外でキーボードが抜け出せなくなる場所を作りません。
- Esc は、いちばん上に重なったもの (メニュー → ツールチップ → ダイアログの順) を 1 つだけ閉じ、フォーカスを開いた元に戻します。
- 1 文字のショートカット (修飾キー無し) は、文字の入力欄・数値の欄・`Select` にフォーカスがあるときは効かせません。ショートカットの一覧はヘルプとメニューに出します (`Kbd`)。

### フォーカス

- フォーカスは常に見えるようにします。`:focus-visible` に `--ugoe-focus-ring` を付け、`outline: none` を代わりの表示なしに使いません。
- フォーカスの輪は、隣り合う色に対して 3:1 以上にします (強調色はどちらのテーマの地にも 5.6 以上)。
- 項目を消したり、ダイアログを閉じたりしてフォーカスしていた要素が無くなったら、近くの意味のある要素 (次の行・一覧・開いた元のボタン) にフォーカスを移します。

### コントラストと色

- 文字は 4.5:1 以上 (24px 以上、または 18.66px 以上の太字は 3:1 以上)。無効の部品の文字は基準の外ですが、読めることを確かめます。
- 部品の境界 (入力欄の枠・チェックボックス・スライダーのつまみと溝) とアイコンは 3:1 以上。上の色のトークンはこれを満たします。新しい色を足すときは比を計算してこの表に書きます。
- **色だけで状態を伝えません。** エラーは色に加えて印と文で、選択は色に加えて形 (枠・チェックの印) で示します。

### そのほか

- **名前:** すべての操作できる部品に、読み上げの名前を付けます (見えるラベル、`<label>`、または `aria-label`)。`aria-label` などの文言も翻訳を通します。
- **見せずに読ませる文字:** 読み上げだけに渡す文字 (表の見出しの無い列の名前・印の意味など) は、`@ugoe/ui` の `tokens.css` の
  `.ugoe-visually-hidden` を付けます。アプリの CSS に同じ指定 (`.visually-hidden`) を書き直しません。
- **押せる大きさ:** 押せる部品は 24×24 CSS px 以上 (WCAG 2.5.8)。`compact` でもこれより小さくしません。
  エディタの密な行 (タイムラインの行とキー・ツリーの行と開閉・パラメータの名前と値の欄) もこれに従います (2026-10-02、利用者の判断。T0907)。
  見た目の印 (◆・▸・グラフの点) は小さいままでよく、押せる範囲 (透明な枠) を 24px に広げます。行が減っても、選んだ行は見える所まで送ります。
- **動き:** `prefers-reduced-motion: reduce` では、部品の開閉の動きを止めます。1 秒に 3 回より多く点滅させません。
- **言語:** `<html lang>` を今の言語にします (読み上げとフォントの選択に効く)。
- **大きさの変化:** WebView の拡大 (200%) と高 DPI、狭い幅 (ビューアとスタジオは 360px、エディタは 1024px) で、文字が切れたり重なったりしないようにします。
  英語は日本語より長くなりやすいので、ボタンや見出しに固定の幅を付けません。
- **部品の作りの注意:** Bits UI に任せた部品も、包んだ後の名前・ラベルの付き方は自分で確かめます。
  例: Bits UI 2.19.4 の `Select` の項目は、無効でも `aria-disabled` を付けないので、`@ugoe/ui` で付けています。
  同じく `Tooltip` の中身に `role="tooltip"` を、`DropdownMenu`・`ContextMenu` の一覧に読み上げの名前 (`aria-label`) を付けています。
  `Toolbar` は、描いた時点で中の項目がすべて無効だと、押せるようになってもタブ止まりが無いまま (Tab で届かない) なので、
  `@ugoe/ui` で「押せる項目のうち 1 つだけを `tabindex="0"`」に保っています (T0907)。

### 確かめ方

| 何を | 道具 | 見るもの |
| --- | --- | --- |
| 部品 | Vitest (キーボード操作と aria の属性)、見本の画面 (`pnpm --filter @ugoe/ui gallery`)、`just ui-shots <出力先>` (両方のテーマ・言語・密度で撮り、名前を検査。[packages/ui/README.md](../packages/ui/README.md)) | 部品を足す・変えたとき (T0902・T0903) |
| 3 つのアプリの画面 | `just ui-audit` ([docs/testing.md](testing.md#ui-の監査)・[tests/ui-audit/README.md](../tests/ui-audit/README.md)) | コントラスト・横のはみ出し・文字の重なりと切れ・押せる大きさ・名前 (axe-core)・ARIA の規則・reduced motion の下の動き・画面のエラー・Tab の順。画面を変える PR の前と後で流して比べる (`--compare`) |
| スクリーンリーダー | 下の手順 | 名前・役割・状態が読まれるか、読む順が見た目の順か |
| 200% の拡大 | `just ui-audit` の `en-light-zoom200` (CSS の窓の幅を半分にし、devicePixelRatio 2) と、実物の WebView | 文字が切れない・重ならない・横に送らずに読める |
| 動きを減らす | `just ui-audit` (すべての場面を `prefers-reduced-motion: reduce` で開き、動いている animation・transition を数える) と、OS の設定 | 開閉・回る印が止まる |

- **スクリーンリーダー:** 機械の検査は名前の有無までで、読まれ方は人が聞いて確かめます。`just ui-audit` が残す `<場面>.aria.yml`
  (アクセシビリティの木。役割・名前・状態) は読まれる内容の近似で、先に目で見て足りないものを探せます。実物では、Linux は Orca
  (`orca` を起動し、Tauri のアプリを開く)、Windows は NVDA、macOS は VoiceOver (Cmd + F5) を使い、次をたどります:
  Tab と Shift + Tab で主な操作を一巡する (名前と役割が読まれ、見た目の順か)・メニューとダイアログを開いて閉じる (開いたことと、閉じた後の
  フォーカスの戻り先が分かるか)・状態を変える (トグル・選択・保存。新しい状態が読まれるか)・エラーを起こす (知らせが読まれるか)。
  実物の WebView (WebKitGTK・WebView2・WKWebView) で確かめるのは T0918・T0919 です。
- **200% の拡大:** ヘッドレスの Chrome では、窓の CSS の幅を半分にして devicePixelRatio を 2 にすると、200% に拡大した WebView と同じ
  レイアウトになります (`en-light-zoom200`)。実物では OS の拡大 (Windows の「拡大縮小」200%・GNOME の「大きな文字」・macOS の
  「ディスプレイ」の拡大) で確かめます。
- **動きを減らす:** `prefers-reduced-motion: reduce` は、Windows の「アニメーション効果」を切る・macOS の「視差効果を減らす」・GNOME の
  「アニメーション」を切る、で入ります。ヘッドレスの Chrome では `--force-prefers-reduced-motion` で入ります。

### 3 つのアプリの見直し (T0907)

エディタ・ビューア・スタジオの操作ウィンドウを、ビルドした画面をヘッドレスの Chrome で開いて見直しました
(ライト・ダーク × 英語・日本語 × 広い幅 (エディタ 1440px・ほか 1280px) と狭い幅 (エディタ 1024px・ほか 360px)、と devicePixelRatio 2)。

- **コントラスト:** 文字を持つ要素ごとに、文字の色と祖先を重ねた地の色の比を WCAG 2.x の式で計算しました (自前の小さなスクリプト。T0915 で `just ui-audit` にした)。
  無効の部品を除いて、すべて基準を満たします (非表示のパーツ名は `opacity` で 3.4 だったので控えめな文字の色に直した)。
- **キーボード:** Tab でたどってフォーカスの印と読み上げの名前を記録しました。スタジオのツールバーに届かなかったもの (上の `Toolbar`) と、
  エディタのツリーの薄い印を直しました。
- **押せる大きさ:** 24px 未満のものは 0 件です (`Toggle` のスイッチ・`Checkbox` の箱は見えるラベルまで、`NumberField` は枠の全体が押せる範囲)。
  エディタのタイムラインの行とキー・グラフの点と取っ手・ツリーの行と開閉・パラメータの名前と値の欄・標準の文字の欄を 24px にしました。

### 3 つのアプリの基準線 (T0915)

`just ui-audit` を入れたときの結果を、M10 のタスクの出発点にしました (詳細と振り分けは T0915 のタスクファイルのメモ)。
T0907 で見た場面ではコントラスト・はみ出し・押せる大きさ・名前が 0 件のままでしたが、T0907 で見ていなかった場面で次が見つかりました。

- エディタの右の列のタブ: 24px 未満の押せるもの (物理のリグと衝突体の一覧・スロットの一覧と描く順の端・ライティングのメッシュの一覧と
  チェックボックス・光の向きの印)。T0674 で直します。
- エディタの 200% の拡大: ビューポートの右上の `webgl2` が左上の道具の並びに重なる (T0675)。
- エディタの空のプロジェクト: パーツのツリーが `role="tree"` なのに項目が無い (axe の `aria-required-children`)。分割の境にフォーカスの印が無い (T0674・T0673)。
- ビューア: ビューポート (`role="img"`) の中に光の向きの印 (`role="slider"`) がある (axe の `nested-interactive`)。光の向きの印にフォーカスの印が無い (T0725)。

## 文言の書き方

### 置き場所と流れ

- 文言はパッケージ・アプリごとのカタログ `locales/<言語>.json` ([i18next JSON v4](https://www.i18next.com/misc/json-format)) に置きます。
  名前空間はパッケージの短い名前 (`ui`・`player`・`face-tracking`・`editor`・`viewer`・`studio`) です。
- 置き場所の一覧・訳す人の約束・言語の足し方は [docs/translation.md](translation.md) にあります。
- **原文は英語 (`en.json`)** です。文言を足す・変える PR では、日本語 (`ja.json`) の訳も同じ PR で足します。`just i18n` が欠け・余り・未使用のキーを見つけます。
- 画面の `.svelte` と画面の `.ts` に、表示する文言を直書きしません (`just i18n` が検査します。見つけるものと対象の外は
  [docs/translation.md の「直書きの文言の検査」](translation.md#直書きの文言の検査))。ログとテストの中の文字列は対象の外です。
- **画面に出る例外の文は `t()` で作ります** (`throw new Error(t("errors.modelMissing"))`)。画面は例外を `errorText(e)` でそのまま
  出すので、例外の文も直書きの文言として検査します。画面に出ない開発者向けの例外 (起こらないはずの状態など) は英語のまま、
  `i18n-ignore` と理由を書きます。
- OS のファイルダイアログに出るフィルタの名前 (`{ name, extensions }` の `name`) も文言です。言語を切り替えても追うように、
  定数にするときは読むたびに `t()` を引く getter にします (エディタの `lib/filePaths.ts` の `PSD_FILTER` など)。
- バックエンドのエラーはコードと引数で届くので、カタログの `errors.<code>` に訳を置きます (ADR 0029 の 4)。

### キーの名前

- キーは入れ子のオブジェクトで、`<画面または部品>.<要素>.<種類>` の順に書きます。各段は lowerCamelCase の英語です。

  ```json
  {
    "tracking": {
      "calibrate": {
        "label": "Calibrate",
        "tooltip": "Use the current face as neutral (facing forward, no expression)"
      },
      "recording": {
        "stop_one": "Stop recording ({{count}} frame)",
        "stop_other": "Stop recording ({{count}} frames)"
      }
    },
    "errors": {
      "camera": { "notAllowed": "The camera was not allowed (check the OS and app settings)" }
    }
  }
  ```

- 最後の段 (種類) は次から選びます: `label` (ボタン・欄の名前)・`tooltip`・`aria` (見えない読み上げの名前)・`placeholder`・`description` (補足の文)・`title` (ダイアログや画面の見出し)・`empty` (空の状態の案内)。
  1 つの要素に文言が 1 つしか無いときは、種類を省いてよいです。
- **キーは意味で付け、英語の文をキーにしません。** 英語の文を直しても、キーは変えません (訳が失われるため)。意味が変わったら新しいキーにします。
- **同じ英語でも、使う場所が違えばキーを分けます** (「Open」は「ファイルを開く」と「パネルを開く」で訳が違いうる)。共通の語 (`ok`・`cancel`・`close` など) だけを `ui` の `common.*` に置きます。
- 文を部品のつなぎ合わせで作りません (`t("a") + name + t("b")` は語順が言語で違うので訳せない)。引数を持つ 1 つの文にします。

### 引数

- 引数は名前付きの `{{name}}` です。位置 (`{{0}}`) は使いません。名前は訳す人が意味を推せるものにします (`{{path}}`・`{{model}}`)。
- カタログに HTML を書きません。Svelte が文字をエスケープするので、i18next のエスケープ (`escapeValue`) は切ります。
  文の一部だけを強調・リンクにしたいときは、文を分けずに済む形 (前後で別の要素にする・リンクを文の後ろに置く) を先に考え、必要なら `@ugoe/i18n` の差し込みの部品 `RichText` (訳文の `<link>…</link>` をスニペットで描く) を使います。

### 複数形

- 数で語形が変わる文は、引数 `count` を渡し、[CLDR の複数形の区分](https://www.unicode.org/cldr/charts/latest/supplemental/language_plural_rules.html) の接尾辞で書き分けます
  ([Plurals](https://www.i18next.com/translation-function/plurals))。英語は `_one` と `_other`、日本語は `_other` だけです。
- 0 のときに別の言い方をしたい場合 (「記録はありません」) は、複数形ではなく別のキー (`empty`) にして、コードで分けます。
- 数でない値 (名前・パス) に `count` を使いません。

### 数値・日付・単位

- 数値・日付・一覧は `@ugoe/i18n` の書式の関数 (`Intl` を今の言語で使う) を通します。`toFixed()` や文字列の連結で書式を作りません。
  - 数値: 桁の区切りと小数の桁数は関数に任せます。パラメータの値のような、編集のための数値は桁の区切りを付けず、小数点は `.` に固定します (入力と表示を揃えるため)。
  - 日付と時刻: `Intl.DateTimeFormat` の `dateStyle`・`timeStyle` を使い、並び順を自分で決めません。「3 分前」は `Intl.RelativeTimeFormat` を使います。
  - 単位: 秒・度・ピクセルは、`Intl.NumberFormat` の `unit` か、単位を含めたカタログの文 (`"{{value}} s"` / `"{{value}} 秒"`) で付けます。
- 一覧 (「A・B・C」と「A, B, and C」) は `Intl.ListFormat` を使います。

### 文体

| | 英語 | 日本語 |
| --- | --- | --- |
| ボタン・メニュー | 動詞で始める sentence case (`Save recording`)。末尾にピリオドを付けない | 短い名詞か動詞 (「保存」「忘れる」)。末尾に句点を付けない |
| 欄の名前・見出し | sentence case の名詞 (`Smoothing`) | 名詞 (「平滑化」) |
| 説明・エラー | 文。ピリオドで終える。何が起きたかと、次に何をすればよいか | 「です・ます」の文。句点で終える。何が起きたかと、次に何をすればよいか |
| 続きの入力がある操作 | 末尾に `…` (U+2026。`File…`) | 末尾に「…」(「ファイル…」) |

- 括弧は半角の `( )` で、前に空白を入れます (「カメラ (既定)」)。日本語の文の中の英数字の前後にも空白を入れます (「`.ugoe` を開く」)。
- 利用者を責める言い方 (「不正な」「間違った」) を避け、状態と直し方を書きます (「ファイルを読めません。ugoe のバンドル (.ugoe) かどうか確かめてください」)。
- 製品名・形式の名前 (ugoe・MediaPipe・`.ugoe`・OBS) は訳しません。
- 言語の切り替えの選択肢は、その言語自身の名前で出します (「日本語」「English」)。
