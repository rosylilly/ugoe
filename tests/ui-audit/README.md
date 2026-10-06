# UI の監査 (`just ui-audit`)

エディタ・ビューア・スタジオの画面を、同じ物差しで繰り返し確かめる道具です (T0915)。基準は [docs/ui.md の「UI の品質の基準」](../../docs/ui.md#ui-の品質の基準)。
ビルドした画面をヘッドレスの Chrome で開き、テーマ・言語・幅・devicePixelRatio を変えて撮り、機械で見つけられる崩れを数えます。
見た目の良し悪し (配置・余白・階層) は機械では決まらないので、撮った画像を人が見ます。

## 実行

```sh
just ui-audit                          # 3 つのアプリをビルドしてすべての場面を撮る (5 分ほど)
just ui-audit --only editor,viewer     # アプリを選ぶ
just ui-audit --quick                  # ライトの英語の広い幅と狭い幅だけ (反復中に)
just ui-audit --no-build               # 前にビルドした dist と見本を使う (アプリのコードを変えていないとき)
just ui-audit --out target/ui-audit-before   # 結果の置き場 (既定 target/ui-audit)
just ui-audit --compare target/ui-audit-before  # 前の summary.json と比べて、増えた・消えた・数が変わった種類を出す
just ui-audit --strict                 # 見つけたものがあれば終了コード 1 (既定は、場面が開けなかったときだけ 1)
```

直す前に `--out target/ui-audit-before` で撮っておき、直した後に `--compare target/ui-audit-before` で比べると、
M10 のタスクの「見直しの前と後」の比較になります。

要るもの: Google Chrome か Chromium (環境変数 `CHROME` で場所を指定できる)、`just wasm` に要るもの (wasm-bindgen-cli)、
cargo (エディタの見本を `ugoe sample dress-up` で作る)。Linux と WSL で確かめています。`just ci` には含めません (アプリのビルドと
200 ほどの場面の撮影で重いため)。道具の単体テストは `just ui-audit-check` (`just ci` に含む)。

## 流れ

1. **ビルド:** アプリごとに `just wasm <出力先>` と `vite build` (出力は `<out>/dist/<アプリ>`)。エディタには `ugoe sample dress-up --project` で
   見本のプロジェクトと、同じモデルの `.ugoe` を作る (`<out>/sample/`)。
2. **配る:** dist を小さな静的サーバ (`lib/serve.mjs`。空いているポート) で配る。並行に流してもぶつからない。
3. **偽のホスト:** Tauri の代わりを置く。画面を見るための偽物で、編集や保存はしない。
   - エディタ (`lib/editor-host.mjs`): `window.__TAURI_INTERNALS__.invoke` を Node の側の読むだけの偽物に中継する。開く前は空のプロジェクト、
     「開く…」で見本のプロジェクト (model.json・physics.json・motions・expressions と `.ugoe`) を開いたことにする。編集のコマンドは
     `ui_audit.read_only` のエラー、知らないコマンドは `null` を返して報告に名前を出す (バックエンドのコマンドが増えたら足す)。
     ファイルのダイアログは `window.__UGOE_FILE_DIALOGS__` で差し替える。
   - スタジオ (`lib/studio-host.mjs`): `window.__UGOE_STUDIO_HOST__` に、`StudioHost` (apps/studio/src/lib/host.ts) のすべてのメソッドを置く。
     見本のプロファイル (apps/studio/scripts/sample-profile.mjs) に画面の既定値 (src/lib/inputs.ts・virtualCamera.ts) を足して送る。
     ページの `window.__uiAuditStudio` の `emit` (指揮役の通知を送る) と `setStatus` (負荷の数値を変える) で、読み込んだ後に状態を変えて撮る
     (初めて起動した画面・承認を待つアプリ・登録できないホットキー・負荷の注意。通知の組は `firstRunMessages`・`attentionMessages`。T0818)。
     操作 API の承認・拒否・取り消しは、様子を変えて送り直す (承認の後の「取り消す」の知らせを撮るため)。
   - ビューア: Tauri の外の口 (ブラウザの口) のまま。ツールバーの「サンプル」で見本を開く。
4. **撮って検査する:** 下の「場面」ごとに PNG を撮り、下の「検査」を流す。
5. **まとめる:** `<out>/<アプリ>/report.md` (見つけたものと Tab の順)・`<アプリ>/tab.txt`・`<アプリ>/*.aria.yml`
   (アクセシビリティの木。スクリーンリーダーが読むものの近似)、`<out>/summary.md` と `summary.json` (数の表と、比べるためのデータ)。

## 場面

組 (テーマ × 言語 × 窓):

| 名前 | テーマ | 言語 | 窓 (CSS px) | devicePixelRatio |
| --- | --- | --- | --- | --- |
| `<en/ja>-<light/dark>-wide` | ライト・ダーク | 英語・日本語 | エディタ 1440×900、ほか 1280×800 | 1 |
| `<en/ja>-<light/dark>-narrow` | 同上 | 同上 | エディタ 1024×720、ほか 360×740 (docs/ui.md の下限) | 1 |
| `ja-light-wide-dpr2` | ライト | 日本語 | 広い幅 | 2 (開いた画面だけ) |
| `en-light-zoom200` | ライト | 英語 | 広い幅の半分 (200% の拡大と同じ CSS の幅) | 2 |
| `en-light-half` (スタジオだけ) | ライト | 英語 | 960×800 (1920 の画面の半分。列の並びが変わる幅) | 1 |

どの組も `prefers-reduced-motion: reduce` で開く (動きの途中を撮らないため。動きの検査も兼ねる)。組ごとに次の場面を撮る (名前は組の名前の後ろに付く)。

| アプリ | 場面 |
| --- | --- |
| エディタ | `-empty` (新しいプロジェクト)、(なし) 見本を開いた画面、`-bottom` (送れる所を下まで送った)、`-settings`、`-file-menu` (ヘッダーの「File」のメニュー)、`-export`、`-selected` (パーツを選んだ)、`-tab-<physics/glue/lighting/slots/poses/verify/ai>` (右の列のタブ)、`-deformers-expressions` |
| ビューア | `-empty`、`-samples-menu`、(なし) 見本を開いた画面、`-bottom`、`-background` (ツールバーの「背景」の Popover)、`-settings` |
| スタジオ | (なし) 開いた画面、`-bottom`、`-settings`、`-profile-menu`、`-output` (「出力ウィンドウ」のパネルまで送った)、`-attention` (承認を待つアプリ・登録できないホットキー・負荷の注意)、`-hotkey-removed` (「いまの状態」のホットキーの印を押して「ホットキー」のパネルへ送り、1 つ目を削除した後の「元に戻す」の知らせ)、`-attention-hotkeys` (「ホットキー」のパネルまで送った)、`-approved` (承認した後の「取り消す」の知らせ)、`-first-run` (初めて起動した画面: 新しいプロファイル) |

広い幅のライトの英語では、Tab を最大 120 回押してフォーカスをたどる (`-tab`)。

場面を足すときは `lib/apps.mjs` の `extras` (読み込んだ後) か `load` (読み込みの途中) で、操作してから `ctx.scene("名前")` を呼ぶ。
ボタンは役割と名前 (アプリのカタログ `apps/<アプリ>/locales/<言語>.json` から `text()` で引く) で探す。使ったキーが英語と日本語の
カタログにあるかは `just ui-audit-check` が確かめる。

## 検査

| 名前 (報告の見出し) | 見つけるもの | 作り |
| --- | --- | --- |
| `contrast` (文字のコントラスト) | 4.5:1 (大きい文字は 3:1) に届かない文字 | 自前 (`lib/page-checks.js`)。文字の色と、祖先をたどって重ねた地の色を WCAG 2.x の式で比べる。無効の部品は数えない。画像の地・canvas の上の半透明の地は測れないので数だけ出す |
| `overflow` (横のはみ出し) | 文書の横の送り・窓の右端を越える要素 | 自前。送る祖先の中は除く |
| `overlap` (文字の重なり・切れ) | 別の要素の文字が透けて重なる・`overflow: hidden` で省略記号なしに文字が切れる | 自前。上に不透明な地を敷いた重なり (ダイアログ・メニュー) と、送る枠の外に隠れた部分は数えない |
| `target-size` (24px 未満の押せるもの) | 24×24 CSS px 未満のボタン・リンク・入力・タブなど | 自前。数値の欄は枠の全体、チェックボックスとスイッチは見えるラベルまで、標準のチェックボックス・ラジオは包む `<label>` までを押せる範囲とする。スライダーのつまみは除く |
| `name` (名前の無い操作できる部品) | 読み上げの名前が無い操作できる部品 | axe-core の名前の規則 (`*-name`・`label` など) |
| `focus` (Tab の止まりの問題) | フォーカスの印が無い (自分と 2 つ上の祖先に outline・box-shadow が無い)・名前が無い・画面の外の止まり | 自前 (`lib/walk.mjs`) |
| `aria` (そのほかの ARIA・言語) | WCAG 2.0〜2.2 の A・AA の axe-core の規則の違反と、`<html lang>` が画面の言語でないこと | axe-core (`color-contrast` と `target-size` は自前の検査と重なるので切る) |
| `motion` (reduced motion の下の動き) | `prefers-reduced-motion: reduce` で動いている 1ms を超える CSS の animation・transition | 自前 (`document.getAnimations()`) |
| `error` (エラー) | 画面に出たエラーの文 (`role="alert"`・`.error` など)・スクリプトのエラー・場面を開けなかったこと | 自前 |

同じ部品の行 (メッシュの一覧の 14 個のボタンなど) は 1 つの「種類」にまとめて数えます。種類は、クラスの付いた要素なら `tag.class`、
クラスが無ければ 3 つ上までの祖先のクラス (`tag@.class`)、どちらも無ければ名前まで含めた要素です。報告の数は「種類 (延べ)」で、
延べは場面ごとに数えた数です。

### 依存

- **playwright-core** (Apache-2.0): 入っている Chrome を動かす (ブラウザはダウンロードしない)。役割と名前で要素を探し、操作を待つため。
- **axe-core** (MPL-2.0): 読み上げの名前と ARIA の規則。名前の計算 (accname) を自前で正しく書くのは難しく、規則も多いため。
  道具の中でだけ使い、手を加えず、アプリには入りません (ルートの devDependencies)。

### 確かめないこと

- 実物の WebView (WebView2・WKWebView・WebKitGTK) での表示。WebKitGTK は `just e2e` (T0006) の側で撮る (T0919)。
- 実際のスクリーンリーダーの読み上げ (`.aria.yml` は近似。docs/ui.md#確かめ方)。OS の高コントラスト (`forced-colors`)。
- ビューポート (モデルの描画) の中身・ギズモの見分けやすさ。偽のホストが返さない状態 (AI の提案・モジュールのプロジェクト・
  エラーの知らせ・接続中のクライアントなど) の画面。要るなら場面と偽のホストの答えを足す。
- 配置・余白・階層などの見た目の良し悪し (撮った画像を人が見る。docs/ui.md#ui-の品質の基準)。
