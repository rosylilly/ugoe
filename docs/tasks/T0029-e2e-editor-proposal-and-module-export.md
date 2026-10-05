---
id: T0029
title: "e2e: エディタの提案のプレビューと「Export Module…」の筋書き、見本を開く手順の時間切れの調査"
status: done
milestone: M9
crates: [tests/e2e, docs]
depends_on: []
assignee: claude-worker-T0029
---

# T0029: e2e: エディタの提案のプレビューと「Export Module…」の筋書き、見本を開く手順の時間切れの調査

## 目的

T0023 でエディタの e2e に PSD の取り込み・書き出し・着せ替えのプレビュー・画像からのトラッキングを足したが、T0685 の残りの
提案のプレビュー (`get_proposal_preview_bundle`) と、モジュールのプロジェクトの「Export Module…」(`.ugoemod` の書き出し) は通していない (T0023 の候補)。

また、T0022 で、並行のビルドの直後にエディタの「opens the sample project」が 1 回だけ見本を開けずに時間切れ (20 秒) になった。原因は調べていない (T0022 の候補)。

## 受け入れ条件

- [x] AI の提案を偽の提供者 (ネットワークなし) で出し、プレビューを開いて閉じる手順がエディタの筋書きにある
- [x] モジュールのプロジェクトを開いて「Export Module…」で `.ugoemod` を書き出し、ファイルができて一時ファイルが残らないことを確かめる手順がある
- [x] 「opens the sample project」の時間切れを負荷の下 (並行のビルドなど) で再現しようとし、原因を直すか、再現しなかった条件と待ちの長さの見直しをメモに書いた
- [x] tests/e2e/README.md の筋書きの表を更新した
- [x] `just e2e` と `just ci` が通る

## 参考

- T0023・T0022 のメモ (起票の候補)、T0685
- tests/e2e/specs/editor.test.mjs

## メモ

- T0026 で起票した (元の候補: T0023 の提案のプレビューと Export Module…、T0022 の時間切れ)。
- T0022 の候補の「エディタの筋書きにプレビューのトラッキング (画像で MediaPipe)」は T0023 で入った (`lib/tracking.mjs` の `checkImageTracking`) ので起票しない。
- T0029 (やったこと):
  - 提案のプレビュー: 偽の AI のプロバイダ `tests/e2e/lib/fake-ai.mjs` (127.0.0.1 の空いているポートの OpenAI 互換の
    Chat Completions のサーバ。答えは要求の JSON Schema から決める) を足した。エディタは送り先がループバックなら確認も API キーも
    要らないので、アプリには手を入れずに本物の経路 (`classify_psd_layers`・`propose_rig_from_psd` の `use_ai`・`OpenAiProvider`) を通る。
    筋書き「proposes a rig from a PSD with a fake AI provider and previews it」: 「New from PSD…」で `character.psd` から作り、
    設定の「AI provider」で送り先を偽のサーバにし (画面の操作)、AI タブで「Use …」を入れて「Propose rig」→ 分類とリグの要求が
    画像付きで届き、分類が AI のもの・提案の理由に偽の答えの文 → 「Preview」で `get_proposal_preview_bundle` が 1 回成功し、
    ビューポートに提案の印とスライダーが出る → 「End preview」で戻り、`get_model_json` が変わらない。
    MCP の経路は使わなかった (ポートとトークンの準備が要り、提案の一覧から先の画面は同じため)。
  - Export Module…: 筋書き「opens a module project and exports a .ugoemod」: 見本を開いたときに書き出したモジュールのプロジェクト
    `cat-ears.ugoeproj` を「Open…」で開き (保存していない変更を捨てる確認を通る。フォルダーのダイアログも `answerDialog` で
    答えられるようにした)、「Export Module…」で書き出す → ファイルが見本の `cat-ears.ugoemod` とバイト列で同じ・一時ファイルが残らない。
  - 押す操作: 負荷の下で、操作の直後のボタン (無効のあいだ) や閉じかけのダイアログの下の部品を押すと「element not interactable」で
    落ちたので、`Session.clickWhen`・`clickText` (無効でなくなるまで待ち、not interactable なら探し直して押し直す) を足し、
    新しい 2 つの手順で使った。
  - CSP: 負荷の下で画像からのトラッキングが 60 秒を超えると、MediaPipe (@mediapipe/tasks-vision 1.0.1) が利用状況を
    `https://odml.pa.googleapis.com/v1/log` へ送ろうとし (60 秒ごとと閉じたとき。止める設定は無い)、アプリの CSP (`connect-src`) が
    止めて「has no CSP violations」が落ちた。止まって正しいものなので `lib/harness.mjs` の `EXPECTED_BLOCKS` に入れて数えない
    (console.json には残る)。3 つのアプリに効く。
  - 文書: tests/e2e/README.md (筋書きの表・ダイアログの代わり・「偽の AI のプロバイダ」・CSP の止まって正しいもの・押す操作)、
    docs/testing.md。
- T0029 (見本を開く手順の時間切れの調査):
  - 再現しなかった。7 回流して、「opens the sample project」はすべて通った。条件: 24 コアの WSL2 で、ほかのワーカーのビルドで
    load average が 37〜80 の下、(a) さらに busy loop を 48 本、(b) 別の target-dir でのワークスペースの `cargo build` を並行、
    (c) (a) と (b) の両方。手順の中の時間を測った (一時的に足して戻した): クリックから見本のファイルができるまで 0.1〜1.2 秒、
    プロジェクトの名前が出るまで 0.7〜5 秒、ビューポートが描けるまで 1.5〜5 秒。手順の残り (4〜8 秒) は最後のスクリーンショット
    (WebKitWebDriver の Take Screenshot が負荷の下で遅い)。
  - 待ちの長さ: 名前とビューポートの待ち (20 秒) は、測った最大 (5 秒) の 4 倍あるので変えない。時間切れの文はどの待ちか
    (`nav.file-menu .project .name with text "dress-up"`・`the viewport to be ready` など) を言うので、次に起きたらそこから絞れる。
    T0022 で落ちたときの記録 (target/e2e) は残っておらず、どの待ちだったかは分からない。
  - 負荷の下で実際に落ちたのは、ただの `click` の「element not interactable」と MediaPipe の CSP (上) だった。T0022 の 1 回も
    同じ種類 (押す前に画面が組み替わった) だった可能性があるが、確かめられない。
- T1616 (エディタのダイアログをバックエンドで開く) の後に rebase し、新しい手順のダイアログの答えを `UGOE_EDITOR_DIALOG_ANSWERS`
  の形 (`answerDialog("open-directory" | "open-file" | "save-file", path)`) に合わせた。rebase の後に `just e2e` (エディタ・ビューア・
  スタジオ) と `just ci` が通った。
- 確かめていないこと: CI (`e2e.yml`) での実行。Windows・macOS (e2e は Linux だけ)。
- 起票の候補:
  - editor/ai: 見本の着せ替えの本体 (`dress-up.ugoeproj`) に `character.psd` を「Import PSD…」で取り込んでから「Propose rig」すると、
    `no valid rig after 3 attempts: command would make the model invalid: slot "Hair": colliders refers to unknown physics collider "Shoulders"`
    で提案が作れない (偽の AI のプロバイダで見た。AI を使わないときも同じかは確かめていない)。リグの提案が本体の物理の衝突体を
    消すか置き換えているらしい。
  - face-tracking: MediaPipe の利用状況の送信 (`odml.pa.googleapis.com`) を CSP が止めていることを、プライバシーの説明
    (docs や同梱の表記) に書くか、送らせない方法 (`ModuleFactory` の差し替えなど) を調べる。
