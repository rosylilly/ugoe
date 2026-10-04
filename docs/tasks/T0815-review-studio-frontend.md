---
id: T0815
title: コードレビュー: スタジオのフロントエンド
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: []
assignee: claude-worker-T0815
---

# T0815: コードレビュー: スタジオのフロントエンド

## 目的

スタジオのフロントエンド のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `apps/studio/src` のすべて (操作ウィンドウ・出力ウィンドウ・ソース)

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - 操作ウィンドウと出力ウィンドウの状態の同期、`$effect` の後片付け
  - カメラ・マイクの資源の解放、出力の毎フレームの処理
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0824〜T0828 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)
- 読んだ範囲: `apps/studio/src` のすべて (入口 3 つ・部品 16・lib 23 とそのテスト 13)。`host.ts` のコマンド名は、バックエンドの `generate_handler!` の一覧とすべて一致することを確かめた。
- 部品と $effect のテストの下地を足した: `apps/studio/vitest.config.ts` (`browser` の条件で解決) と devDependencies の happy-dom (packages/ui と同じもの)。
  テストのファイルの先頭に `// @vitest-environment happy-dom` を書くと、mount と $effect が動く (既定の node 環境では server 向けになり動かない)。

### 所見

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 高 | `src/ApiApproval.svelte` | 承認の知らせの権限の選択を、`api` の通知のたびに (ほかのクライアントがつないだなど) クライアントが求めた権限へ戻していた。利用者が「読むだけ」に狭めても、承認の直前に通知が来ると「操作する」で承認される | 直した (別のクライアントになったときだけ当て直す。`ApiApproval.svelte.test.ts`) |
| 中 | `src/lib/microphone.svelte.ts` の `start` | getUserMedia を待つ間に `stop` か次の `start` (機器の選び直し) が来ると、後で届いたマイクのストリームと AudioContext を止めずに使い続けた (止めたのに動き出す・先のマイクを掴んだまま) | 直した (始めた回数で古い方を捨てる。`microphone.svelte.test.ts`) |
| 中 | `src/InputPanel.svelte` の `$effect` | トラッカーの値を書き戻す $effect がプロファイルの設定にも反応し、カメラが動いている間に別のプロファイルを開くと、そのプロファイルのカメラを動いているカメラで書き換えて未保存にした (保存先があれば自動で保存される) | 直した (`lib/trackerSettings.svelte.ts` に分け、カメラは使い始めた・替えたときだけ書き戻す。テストを追加) |
| 中 | `src/HotkeysPanel.svelte`・`MotionPanel.svelte`・`InputPanel.svelte` | 変更を「今の props の写し + 変更」で送り、画面は通知で変わる。通知の前に続けて変えると先の変更が消える。割り当てを足した直後に押したキーは捨てられることがある | T0824 を起票 |
| 中 | `src/Output.svelte` の `applyScene` | モジュール・モデルを取れない・読めないとき、「次のシーンになった」とみなして黙って戻る。読めなかった理由は `player.error` に書くだけで誰にも見えない | T0825 を起票 (IPC が要る) |
| 中 | `src/Output.svelte` | シーンの通知の順の処理・古いシーンの取り消し・blob URL の解放にテストが無い | T0826 を起票 |
| 中 | `src/lib/errors.ts`・`VirtualCameraPanel.svelte`・`ApiPanel.svelte` | バックエンドの `Result<_, String>` のコマンド (`set_tick_rate`・`start_virtual_camera`・`set_output_size`・`set_api_port`・`scene_model`) と `ApiView.error` の英語の文を、そのまま画面に出す (ADR 0029 の 4.) | 見送り (直すのはバックエンドのエラーの形。T0814 の範囲。横断に書いた) |
| 低 | `src/lib/virtualCamera.ts`・`src/lib/settings.ts` | 仮想カメラの設定と刻みの速さが localStorage のまま (コメントは「プロファイルができたら移す」) | T0827 を起票 |
| 低 | `src/Control.svelte` の `error` | 上の帯のエラーは、成功しても消えず、閉じるボタンも無い (プロファイルを開く・モデルを選ぶまで残る) | 見送り (画面の作りは M10 の T0817 で扱う) |
| 低 | `src/Control.svelte` の `nextScene` | 足したシーンを、プロファイルの通知が届くまで 25ms ごとに最大 20 回待って探す | 見送り (動いている。T0824 で送っている途中の値を持つ形にすれば要らなくなる) |
| 低 | `src/SourcePanel.svelte` の `$effect` | URL を取り直す途中の応答を取り消さない (古い応答で上書きしうる) | 見送り (画面に出すのはトークンを伏せた URL で、古いものと見分けが付かない) |
| 低 | `src/LicenseDialog.svelte` | 読み込み中に開き直すと 2 回読む | 見送り (害が無い) |
| 低 | `src/lib/feed.ts` の `sourceFeed` | `subscribeFrames` を 2 回呼ぶと WebSocket が 2 つになる。つなぎ直しを止める口が無い | 見送り (ページの寿命の間に 1 回だけ呼ぶ) |
| 低 | `src/lib/pulse.ts` の `schedule` | つなぐ前に登録した処理は requestAnimationFrame のまま (隠れると止まる) | 見送り (起動の直後のごく短い間だけ。カメラは押してから始まる) |
| 低 | `src/SceneEditor.svelte` | 大きさのスライダーの上限が 5 で、プロファイルの範囲 (`SCALE_RANGE` は 20) より狭い | 見送り (スライダーの範囲は意図して狭い。見せ方は M10) |
| 低 | `src/Control.svelte` の `chooseModel` | `structuredClone($state.snapshot(...))` で 2 回写している | 見送り (害が無い) |
| 低 | `src/lib/settings.test.ts`・`virtualCamera.test.ts` | 偽の localStorage (`MemoryStorage`) が重複 | 見送り (テストの小さな重複) |

- `$effect` の後片付け: リスナーを足す $effect は無い (`HotkeysPanel` は `<svelte:window>`、`Control` と `Output` は onMount の戻り値で外す)。
- カメラ・マイクの解放: マイクは上の修正で、止めた後と始め直した後に届いたものも捨てる。カメラ (@ugoe/face-tracking の `FaceTracker`) にも同じ形の競合がある (下の「横断」)。
- 出力の毎フレームの処理: 届いたフレームの `decodeFrame` と `pushFrame`、描いたときの `RenderStatsWindow.record` だけで、配置の計算 (`layout`) は大きさとシーンが変わったときだけ。負荷の報告は約 1 秒に 1 回。
- 操作ウィンドウと出力ウィンドウの同期: どちらもバックエンドの通知 (`subscribe_status`・`subscribe_frames`) だけを見て、ウィンドウ同士では直接やりとりしない。出力はシーンの版 (`revision`) で古い通知を捨てる。

### 横断 (T0008 向け)

- **$effect が思わぬ値に反応する:** $effect の中で呼ぶ関数 (`$state.snapshot(props)` など) が読んだ props も依存になり、外から届いた通知で書き戻しや選択の戻しが走る (ApiApproval・InputPanel)。依存にするものだけを読み、残りは `untrack` で読む。ほかのアプリの部品にもありそう。
- **非同期の start と stop:** getUserMedia などを待つ間に stop や次の start が来ると、後で届いた資源を掴んだままになる。`packages/face-tracking` の `FaceTracker.#start` / `startCamera` も同じ形 (待った後に `this.#stream = stream` と `status = "running"` を確かめずに書く)。T0913 の範囲。
- **バックエンドの英語のエラーをそのまま出す:** Tauri のコマンドが `Result<_, String>` で返すと、フロントエンドは翻訳できない (ADR 0029 の 4.)。スタジオでは T0814 の範囲。エディタ・ビューアのコマンドも確かめるとよい。
- **部品と $effect のテストの下地:** 既定の node 環境では svelte が server 向けになり、mount と $effect が動かない。スタジオには下地を足した (vitest.config.ts と happy-dom)。ほかのアプリにも部品のテストは無い。
- **通知を待つ編集:** 設定を送ってから通知で画面を変える形では、続けて変えると先の変更が消える (T0824)。エディタは Command と Undo を通すので別だが、ビューアなどの同じ形も確かめるとよい。

### `just ci`

- 1 回目は `web-check` の `@ugoe/editor` の `src/lib/preview/tracking.test.ts` が `Hook timed out in 10000ms` (`beforeAll` の動的な import) で止まった。
  このブランチの変更は apps/studio・docs と pnpm-lock.yaml の happy-dom の 1 行だけで、エディタには触れていない。そのファイルだけを流すと 9 秒で通り
  (10 秒の上限に近い)、並行のワーカーで負荷が高い (load average 24) ためと判断した。`pnpm --filter @ugoe/editor test` を流し直して 44 ファイルすべて通った。
- `web-check` の残り (viewer・studio・face-tracking・player・i18n・runtime・ui の check と test)・`i18n`・`app-build-check` を流し直してすべて通った。
  `web-check` より前の段 (fmt-check・lint・test・layers・groups・licenses・publish-check・spec) は 1 回目で通っている。
- Biome の警告 1 件 (`bindings/unity/release.mjs` の使っていない import) は main にもあり、失敗にはならない。
