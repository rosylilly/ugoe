---
id: T0918
title: UI 品質: スクリーンリーダーと OS の設定 (3 つのアプリ横断)
status: done
milestone: M10
crates: [packages, apps/editor, apps/viewer, apps/studio, docs]
depends_on: [T0673, T0674, T0675, T0676, T0725, T0818]
assignee: claude-worker-T0918
---

# T0918: UI 品質: スクリーンリーダーと OS の設定 (3 つのアプリ横断)

## 目的

T0907 で確かめていない、スクリーンリーダーでの読み上げと OS の設定 (動きを減らす・高コントラスト・200% の拡大) への対応を 3 つのアプリで確かめる。
基準は docs/ui.md (T0915 で足す「UI の品質の基準」を含む)。直せないもの・大きいものは後続のタスクにする (milestone は M10)。

## 受け入れ条件

- [x] ランドマーク (header・nav・main・aside) と見出しの階層が、3 つのアプリで画面の構造と合っている
- [x] 状態の変化 (保存した・AI の提案が届いた・エラー・シーンが変わった) を live region で知らせる (うるさくならない範囲で)
- [x] NVDA (Windows) か Orca (Linux) で主な流れを読み上げ、名前・役割・状態が分かることを確かめた (確かめられない環境なら、手順を docs/manual-check.md に足して利用者の確認に回す)
- [x] `prefers-reduced-motion`・`forced-colors` (Windows の高コントラスト)・WebView の 200% の拡大で崩れない
- [x] docs/ui.md の「アクセシビリティの基準」に、確かめたことと確かめ方を足した
- [ ] `just ci` が通る

## 参考

- docs/ui.md
- docs/ui.md
- docs/manual-check.md

## メモ

- T0032 (3 回目の起票のまとめ) から: T0915 の候補をこのタスクで扱う。
  - `just ui-audit` に forced-colors (`forcedColors: "active"`) の組を足す (受け入れ条件の `forced-colors` の確認を、監査でも繰り返せるように)。
  - (任意) Tab の止まりのフォーカスの印の判定は、自分と 2 つ上の祖先の outline・box-shadow を見るので、常に影を持つ祖先の中では印の欠けを見逃しうる。
    フォーカスの前と後の見た目の差 (画像) で判定する作りにするか検討する。

### 結果 (claude-worker-T0918)

やったこと:

- **`just ui-audit` (tests/ui-audit。`crates:` の外だが、T0032 のメモで本タスクに振られた候補なので行った):**
  - `structure` の検査: axe-core のランドマークと見出しの規則 (`landmark-*`・`page-has-heading-one`・`heading-order`・`empty-heading`・`region`。
    best-practice の札なので名指しで流す)。メニュー・Popover・ダイアログと、中身が `role="separator"` だけの要素は除く。
  - 強制色の組 `en-dark-forced-wide` (Tab もたどる)・`ja-light-forced-narrow` (Playwright の `forcedColors: "active"` = CDP の Emulation)と、
    `forced` の検査 (同じ種類の部品の入と切の見た目が強制色で同じもの)。強制色では canvas の上の文字を画素で測れないので `unmeasured` に数える。
  - 日本語の 200% の拡大 `ja-light-zoom200`。場面ごとの骨組み `<場面>.outline.txt` (ランドマーク・見出し・live region)。反復用の `--variants`。
- **packages/ui:** `announce()` (読み上げだけの知らせ。見えない polite と assertive の領域は `Toaster` が先に置く)。`tokens.css` に強制色の節
  (押したボタン・区切りの選択・チェックボックス・スイッチ・スライダー・タブの下線・入力の誤り)。テストは overlays.test.ts と tokens.test.ts (強制色の節が指すクラスが残っているか)。
- **packages/face-tracking:** `TrackingPanel` に見出しの段 `level` (エディタのプレビュー 3・スタジオ 4)。
- **3 つのアプリ:** 見た目で隠した `<h1>` (アプリの名前) をバナーの中に。エディタの左右の列に名前、検証と AI のタブに隠した `h2`、
  AI に送っている間の経った秒数を `role="status"` の外へ、新しく届いた提案を `announce()` (proposalsView.test.ts)。日本語の 200% で
  ヘッダーのモジュールのボタンが重なっていたので、名前の欄が縮みきったらヘッダーを折り返す。ビューアのモーション・表情の見出しからボタンを外へ、
  モデルを開き終えたら `announce()` (loadedModel.ts)。スタジオのパネルの見出しを `h2`・入力のまとまりを `h3` に、出しているシーンが変わったら
  (クリック・ホットキー・API) と手で保存したら `announce()` (liveScene.ts)。
- **文書:** docs/ui.md (「骨組み (ランドマークと見出し)」「状態の変化の知らせ」「強制色」、確かめ方の表、結果の「スクリーンリーダーと OS の設定 (T0918)」)・
  docs/manual-check.md の U03 (スクリーンリーダーで主な流れを読む。NVDA・VoiceOver・Orca の手順) と U04 (高コントラスト・動きを減らす・拡大)・
  tests/ui-audit/README.md・docs/testing.md・packages/ui/README.md。
- **確かめたこと:** `just ui-audit` (全組) は前 (T0918 の前の main) と比べて +0 -0。新しい検査 (structure・forced) と新しい組 (強制色・日本語の 200%) を
  含めて、残るのはスタジオのコントラストの 2 種類 (3) だけで、前からあるもの (下の候補)。`just e2e` (3 つのアプリ) はすべて通った。

やらなかったこと・確かめていないこと:

- 実際のスクリーンリーダー: 作業の環境 (WSL2) に Orca も NVDA も無く、読み上げは聞いていない。手順を docs/manual-check.md の U03 に足して
  利用者の確認に回した (受け入れ条件の「確かめられない環境なら」の側でチェックした)。`.aria.yml` と `.outline.txt` で近似を見た。
- 実物の WebView の強制色 (Windows のコントラスト テーマ)・OS の拡大・動きを減らす: ヘッドレスの Chrome のエミュレーションだけ。U04 に回した。
- Tab の止まりの印を「フォーカスの前と後の画像の差」で判定する案 (メモの任意の候補): 強制色の組で Tab をたどる (影の輪は強制色で消えるので、
  影だけの輪は印の無い止まりとして見つかる) ことで一部を補い、画像の差の判定は作っていない。

起票の候補:

- スタジオのダークの狭い幅の `-approved` の場面で、シーンの一覧の `scenes__model` と `Kbd` の「+」が 4.29 (地は hover の重ねの rgb(62, 58, 75))。
  T0918 の前からある。ポインタが行の上に残った場面かもしれないので、hover の重ねの上の控えめな文字の色を見直す。
- 条件で足す `Notice` (`role="status"`) は、足した時点で読まれない読み上げソフトがある。実機 (U03) で読まれないものが見つかったら `announce()` に移す。
- モーダルのダイアログ (Bits UI。`aria-modal`) を開いている間の `announce()`: 外の live region を読まない読み上げソフトがありうる。U03 の結果を見て、
  `Toaster` の top layer の中に領域を移すか決める。
- フェイストラッキングの状態 (顔が見つからない など) は、fps・再生の位置と一緒の文で毎フレーム変わるので live region にしていない。
  状態だけを分けて (揺れを抑えて) 知らせるかを決める。
- Tab の止まりの印の判定を、フォーカスの前と後の画像の差にする (上の「やらなかったこと」)。

