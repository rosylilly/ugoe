---
id: T0676
title: エディタ UI: AI・モジュール・着せ替えの画面
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: [T0673, T0670]
assignee: claude-worker-T0676
---

# T0676: エディタ UI: AI・モジュール・着せ替えの画面

## 目的

AI の提案と承認、モジュールのプロジェクト・違いの比較・着せ替えのプレビューの画面を見直す。
基準は docs/ui.md (T0915 で足す「UI の品質の基準」を含む)。直せないもの・大きいものは後続のタスクにする (milestone は M10)。

## 受け入れ条件

- [x] AI の提案の一覧・差分・承認と却下の配置を見直し、何が変わるか・承認するとどうなるかが一目で分かる
- [x] AI の待ち時間 (送信中・考え中・失敗・取り消し) の見せ方を揃えた
- [x] モジュールの違いの比較 (T0654〜T0659) の表の配置と、長い一覧での送り・絞り込み
- [x] 着せ替えのプレビューと警告の出し方 (T0649・T0652) を揃えた
- [x] 同意・API キーの設定の画面で、秘密の扱いが分かる文と配置になっている
- [x] T0915 の監査 (`just ui-audit`) で、範囲の画面のコントラスト・はみ出し・押せる大きさ・名前の無い部品が 0 件 (残すものは理由を docs/ui.md に書く)
- [x] 見直しの前と後のスクリーンショット (ライト・ダーク × 日英 × 広い幅・狭い幅) を比べ、メモに要点を書いた
- [x] 画面が変わったところを、マニュアル (docs/manual/) の文と画像の更新の候補として T0919 のメモに書き足した
- [x] `just ci` が通る

## 参考

- docs/ui.md
- docs/ai-integration.md
- docs/manual/ai.md
- docs/manual/dress-up.md

## メモ

- T0032 (3 回目の起票のまとめ) から: T0915 の候補「`just ui-audit` の偽のホストに、AI の提案・モジュールのプロジェクト・エラーの知らせの状態を足す」の
  うち、このタスクの範囲の画面 (AI の提案・モジュール・着せ替え) を撮るのに要る状態は、このタスクの中で足す (tests/ui-audit の偽のホスト)。
- T0040 (6 回目の起票のまとめ) から書き足した: T1629 で、開いたプロジェクトの分類のキャッシュはバックグラウンドで戻すようになった (大きな PSD では
  AI のタブに分類が出るまで少し待つ)。戻している最中であることを AI のタブに出す (受け入れ条件の「AI の待ち時間の見せ方を揃えた」に含める。
  終わりはイベント `classification-restored`、戻さなかったときの知らせは無いので、要ればバックエンドに足す)。
- **やったこと:**
  - 偽のホスト (tests/ui-audit): `host.set({ ai, restoring, dressUp, module })` で、AI の提案と分類・外部に送れる設定・送る前の確認・答えない送信
    (取り消すと `ai.cancelled`)・失敗 (`ai.time_limit`)・分類のキャッシュを戻している最中・着せ替えのモジュール (見本の `.ugoemod` と読めない
    ファイル)・モジュールのプロジェクト (今の編集は見本の `.ugoemod` の `MODU` の名前を `Sailor (editing)` に書き換えて作る)・違いの長い一覧を返す
    (`lib/editor-fixtures.mjs`)。場面 `-settings-ai`・`-ai`・`-ai-list`・`-ai-restoring`・`-consent`・`-ai-sending`・`-ai-cancelled`・`-ai-error`・
    `-preview-dress-up`・`-module-ai`・`-module-preview` を足した。失敗の出し方を撮る場面は `scene(name, { errors: true })` で画面のエラーの文を数えない。
    答えを待たせるため `ipcBinding` を async にした。単体テスト (hosts.test.mjs・checks.test.mjs) を足した。
  - AI のタブ (ProposalsPanel・proposals.svelte.ts・proposals.ts): 提案のカードに「採用で変わるもの」(`changeSummary`)・採用が 1 つの編集で 1 回の
    Undo で戻ることの文 (採用のボタンの `aria-describedby`)、ボタンを Reject・Regenerate・Preview・Adopt の順 (決定が最後)、古い提案は `Notice` (warning)。
    `primary` は画面に 1 つ (提案が無ければ Propose rig、あればプレビュー中か最初の採用できる提案の Adopt。法線マップは secondary)。理由の文の内側の
    送りとコマンドの一覧の高さの上限をやめた。閉じた `<details>` (Labels・コマンドの一覧・まとまり) は中を描かない (重なりの 12 種類の原因)。
    コマンドの一覧のチェックボックスは 24px の label で包んだ。
  - 待ち時間: 進み具合・送り始めてからの秒数 (`sentAt`。確認の後に送り直すと数え直す)・Cancel・確認待ちの文・取り消した (`Notice` info)・失敗
    (`Notice` danger、閉じられる) を、始めた場所 (パネルの頭か提案のカード。`scope`) に出す `status` のスニペットにまとめた。
  - 分類のキャッシュを戻している最中の表示 (T1629 の続き): バックエンドに `ProposalState.restoring` (開いたときに立てる。キャッシュのファイルが
    無ければ戻す仕事も作らない。終えた・戻せなかった・PSD を選び直したら下ろす) を足し、イベントを `classification-restore-finished` (戻せなくても送る)
    に替えた。`ipc.onClassificationRestoreFinished`。Rust のテスト `the_panel_shows_while_the_classification_is_being_restored`。
  - AI の設定 (SettingsDialog): API キーの欄に見えるラベルと、保存先・送り先・書かない所の文 (環境変数のキーならその旨) を `aria-describedby` で結び、
    削除を `AlertDialog` で確かめる。設定の失敗と接続の確認の失敗を `Notice` (danger) に。プロバイダのラジオの行を 24px に。送る前の確認
    (ConsentDialog) に「送るのは表のものだけ、キーは認証だけ」の文、規約のリンクを 24px の行に。
  - 着せ替え (DressUpSection・PreviewPanel・dressUp.ts): 失敗を `Notice` (danger。パネルの頭は閉じられる)、行の警告と自動で外れた知らせを `Notice`
    (warning) にそろえ、行のファイルは名前だけ (全体は `title`)。違いの一覧を、行の文の下の要素の表 (ID・違い・Select) に。要素が 8 件を超えると絞り込み
    (`filterDifferences`)、1 行は 8 件まで出して「さらに N 件」。要素の ID とライセンスの口は `app.css` の `.ed-link` (24px)。札は `.ed-tag`。
  - テスト: proposals.test.ts (`changeSummary`)・proposalsView.test.ts (送り始めた時刻・場所)・ProposalsPanel.svelte.test.ts (カードの文・ボタンの順・
    primary が 1 つ・古い提案の知らせ・戻している最中)・dressUp.test.ts (`filterDifferences`)。e2e は失敗の探し方を `Notice` の danger にも合わせた。
  - 文書: docs/editor.md (提案のカード・待ちの見せ方・キーの扱い・着せ替えの知らせと違いの一覧・キャッシュの知らせ・読み込み中の表)、docs/ui.md
    (「エディタの AI・モジュール・着せ替え (T0676)」・`.ed-link`・残した title)、docs/manual/ai.md・dress-up.md、tests/ui-audit/README.md、T0919 のメモ。
- **`just ui-audit --only editor` (前: 場面を足して UI を変える前、`target/ui-audit-before`。後: main c11f710d に rebase した後):** 前は足した場面で
  重なり 12 種類・24px 未満 4 種類・ARIA 2 種類 (理由の文の送りにキーボードが届かない)。後は範囲の画面ですべて 0 件 (`--compare`: -18)。
  +4 は T0675 の `-gizmo` の場面のインスペクタのキーの印 (`button.go`・`button.marker`・`button.remove`) で、T0675 が docs/ui.md に残したもの (範囲の外)。
- **前と後の画像の要点 (ライト・ダーク × 日英 × 1440px・1024px、200%):** AI のタブは、カードの理由の下に採用で変わるものと採用の文が出て、ボタンが
  1 行目に Reject・Regenerate、2 行目に Preview・Adopt (狭い幅では 1 行)。primary が 1 つになり、提案があると Propose rig は secondary。
  送信中は「… 1 s」と Cancel、失敗は赤い枠の知らせ。設定はキーの欄の上に扱いの文、ラベル付きの欄。着せ替えは、行のパスが 3 行から
  ファイル名 1 行になり、失敗が文の色から知らせの枠に、違いの一覧が表 (ID・違い・Select) と絞り込みの欄に。ダークでも知らせと表の線は見える。
- **見た目の大きな判断は無し** (部品の形は既存の Notice・TextField・Button のまま)。
- **やらなかったこと:**
  - 提案のカードの「採用で変わるもの」はコマンドの種類 (`add_parameter` など) の数で、利用者の言葉 (「パラメータを 5 個足す」) にはしていない
    (種類ごとの訳が要る。起票の候補)。
  - AI の待ちの回る印は付けていない (文と秒数だけ。reduced motion の扱いが要らない)。法線マップの提案の待ちは秒数を出さない (送らない)。
  - 実物の WebView・スクリーンリーダーでの確認 (T0918・T0919)。偽のホストでは本物の送信・本物のキャッシュの戻しの時間は作れない
    (e2e の AI の手順は通る)。
- **`just ci`:** 通った (main c11f710d に rebase した後)。`just e2e --only editor` は 15 件すべて通った。
- **起票の候補:**
  - 提案のカードの「採用で変わるもの」を、コマンドの種類ごとの訳 (「パラメータを足す × 5」) にする (エディタのカタログに種類の名前を足す)。
  - 提案の一覧の分類の直しの行に、残すラベルと左右を出す (T1631 の候補のまま)。
  - プレビューの着せ替えの行の自動で外れた知らせ (`dressUp.notices`) と行の失敗が同じ理由を 2 回出す (同じスロットに 2 つ付けたいとき)。どちらか一方にまとめる。

