---
id: T0023
title: "e2e: エディタの PSD の取り込み・書き出し・着せ替えのプレビューと、画像からのトラッキング"
status: done
milestone: M9
crates: [tests/e2e, docs]
depends_on: [T0022]
assignee: claude-worker-T0023
---

# T0023: e2e: エディタの PSD の取り込み・書き出し・着せ替えのプレビューと、画像からのトラッキング

## 目的

T0685 でエディタのファイルを扱うコマンド (PSD の取り込み・`.ugoe` の書き出し・プレビューの着せ替え) を非同期にしたが、
実機の画面での流れは確かめていない (T0685 の実機の受け入れ条件が未チェック)。e2e の筋書きに足して、実機の WebView で確かめる。
また、プレビューのトラッキング (MediaPipe を読む) は、カメラの無い CI でも画像を入力にすれば流せる (T0686 の候補)。
ビューアとスタジオにも同じ入口 (画像からトラッキングを始める) があれば、同じ手順を共通にする。

## 受け入れ条件

- [x] エディタの筋書き: `assets/fixtures` の PSD を取り込み、ツリーにレイヤーが出る → `.ugoe` を書き出してファイルができる → プレビューでモジュールを付け外しする
- [x] 画像からトラッキングを始め、MediaPipe が読み込まれてパラメータが動くことを確かめる (エディタ。ビューア・スタジオにも入口があれば同じ補助で)
- [x] 流した結果で、T0685 の未チェックの受け入れ条件を確かめられたものをメモに書いた
- [ ] `just e2e` と `just ci` が通る

## 参考

- T0685・T0686 のメモ (起票の候補)
- tests/e2e/specs/editor.test.mjs

## メモ

- T0019 で起票した (元の候補: T0685 の e2e、T0686 の画像からのトラッキング)。T0022 と同じ tests/e2e を触るので、その後に着手する。
- T0023 (やったこと):
  - エディタの筋書きに 4 手順を足した (保存の後、言語の切り替えの前): 「Import PSD…」で `assets/fixtures/character.psd` を取り込み、
    ツリーに最上位のパーツ `character`・グループ `頭`・レイヤー `前髪` の行が出て、モデルにレイヤーのメッシュがある →
    「Export…」で `.ugoe` を書き出し、ファイルができ (10KB 超)、同じフォルダーに一時ファイルが残らない (T0687) →
    プレビューで「Load .ugoemod…」から見本の `cat-ears.ugoemod` を読み込むとスロット Headwear に付き、チェックを外すと外れ、
    スロットの選択欄で付け直せる → 画像からトラッキング。ファイルを選ぶダイアログは `answerDialog` で次に返すパスを決めてから押す。
  - 画像からのトラッキング: `lib/tracking.mjs` の `checkImageTracking` を 3 つのアプリ (エディタのプレビュー・ビューア・スタジオの
    操作ウィンドウ) で使う。今のトラッキングを止め、TrackingPanel の「File…」の入力欄に WebDriver の Element Send Keys で
    画像のパスを渡し (隠れた入力欄でも WebKitWebDriver は受ける)、トラッカーが `running`・入力が画像・顔が見つかるまで待ち
    (本物の MediaPipe の wasm とモデルを読む)、`MouthOpenY` のスライダーが 0.15 より大きく動くことを確かめる。
    結果は `target/e2e/<アプリ>/<アプリ>-tracking-image.json` とスクリーンショット。3 つとも `MouthOpenY` 0 → 0.43 (検出は約 1 回/秒、
    MediaPipe を読んで顔を見つけるまで 10〜15 秒)。
  - 顔の画像: 人の写真は入れず、見本のアバターの描画 (`assets/samples/avatar-params.jpg` の口を開けた升、MIT) を切り出した
    `tests/e2e/fixtures/avatar-mouth-open.jpg` (176x168、14KB) を置いた。先にヘッドレスの Chrome で、MediaPipe がアニメ調の
    この顔を見つける (`jawOpen` 0.49) ことを確かめた。見本の着せ替えの描画 (`dress-up.jpg`) は顔が見つからなかった。
  - 文書: tests/e2e/README.md (筋書きの表・ダイアログの代わり・「画像からのトラッキング」の節)、docs/testing.md。
  - `just e2e` (3 つのアプリ、ビルドから) が通り、その後 `--no-build` で 2 回流して通った。
- T0685 の未チェックの受け入れ条件 (「実機 (`just editor` と T0006 の e2e) で、開く・保存・PSD の取り込み・書き出し・プレビューが
  今までどおり動く」) のうち、e2e の側は確かめた: 同梱したページのデバッグビルド (Linux・WebKitGTK・Xvfb) で、見本を開く・保存・
  PSD の取り込み (非同期で作り、ツリーに出る)・`.ugoe` の書き出し・プレビュー (着せ替えの付け外し・トラッキング) が動く。
  `just editor` (開発モード) を手で操作することと、提案のプレビュー (`get_proposal_preview_bundle`)・`.ugoemod` の書き出し・
  法線マップは確かめていない (この環境では画面を手で操作できない)。T0685 のチェックはオーケストレーターに任せる。
- `just ci` (受け入れ条件 4 は付けていない): このブランチの変更と無関係な 1 件で失敗した。`web-check` の
  `apps/editor/src/lib/viewport/ToolOverlay.test.ts > ToolOverlay mesh edit keys > deletes the selected vertices with Delete`
  (`AssertionError: expected [ +0 ] to deeply equal []`、`expect(selection.vertices.Quad).toEqual([])`)。T0699 で入ったテストで、
  main (a1a4a7c6) の作業ツリーでも 3 回とも同じく失敗する。このブランチは tests/e2e と docs しか変えていない。
  `just ci` はそこで止まるので、残りの手順 (viewer・studio・face-tracking・ui・i18n・player・runtime の check と test、
  `i18n`・`app-build-check`・`script-check`) を個別に流し、すべて通った。fmt-check・lint・test・layers・refs などの前半は通った。
- 確かめていないこと: Windows・macOS (e2e は Linux だけ)。CI の `e2e.yml` での実行 (MediaPipe のモデルはビルドで取得するので
  CI でも同梱される前提。CPU の速さで MediaPipe の読み込みが 60 秒の待ちを超えないかは見ていない)。
- 起票の候補:
  - editor: `ToolOverlay.test.ts` の「deletes the selected vertices with Delete」が main で失敗する (T0699 の後)。
  - e2e: 提案のプレビュー (AI の提案を偽の提供者で出してプレビューする) と、モジュールのプロジェクトの「Export Module…」の筋書き
    (T0685 の残り)。
