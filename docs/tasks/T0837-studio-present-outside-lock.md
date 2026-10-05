---
id: T0837
title: "スタジオ: パペットの組み立て (Conductor::present) を舞台のロックの外で行い、操作ウィンドウに読み込み中を出す"
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0836]
assignee: claude-worker-T0837
---

# T0837: スタジオ: パペットの組み立て (Conductor::present) を舞台のロックの外で行い、操作ウィンドウに読み込み中を出す

## 目的

T0834 でシーンの切り替えのファイルの読み込みを舞台のロックの外に出したが、残りがある (T0834 の候補)。

- 大きなモデルでは `Conductor::present` (`src-tauri/src/conductor.rs`。パペットの組み立て) も重い。`Puppet` をロックの外で作ってから差し込む。
- 操作ウィンドウは、シーンの読み込み中に何も出さない (応答が返るまで)。

## 受け入れ条件

- [x] パペットの組み立てを舞台のロックの外で行い、組み立ての間にほかのコマンドが答えるテストがある (T0834 のテストの形)
- [x] 組み立ての間にシーンが変わったら、古い組み立ての結果を差し込まない。テストがある
- [x] 操作ウィンドウに読み込み中の表示があり、Vitest のテストがある
- [x] docs/studio.md#メインのスレッドを塞がない を更新した
- [x] `just ci` が通る

## 参考

- T0834 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0834 の 2 つ)。受け手の読み込みを触る T0836 の後に着手する。

### 完了時 (claude-worker-T0837)

やったこと:

- `conductor.rs`: パペットの組み立て (`Puppet::new`・物理の設定・段を切る・モジュールの合成) を `assemble(NewModel, &[Arc<Module>]) -> Assembled`
  に分け、指揮役の状態を使わずに組み立てる。`Conductor::present` は `Option<Assembled>` を受けて差し込む (演技の読み込み `Animation::load`
  だけを行う。組み立てたときと違うモジュールの組なら付け直す)。`Conductor::load` は `assemble` してから `present`。
- `stage.rs`: `Pending::load_with` が読んだ後に組み立てる (`Prepared<NewModel>` → `Prepared<Assembled>`)。読む関数と組み立てる関数を
  `Loading { read, assemble }` にまとめ (ふつうは `LOADING`)、テストで差し替えられるようにした。組み立てられない (モジュールを付けられないなど)
  ときのエラーの写し方 (`dressUp`・`unreadable`) は差し込むときと同じ `load_error`。後から始めた切り替えがあれば、組み立てた結果も
  T0834 の番号の仕組みで捨てる (差し込まない)。
- `lib.rs`: `switch_scene_with`・`present_with` が `Loading` を受ける。ロックの順 (舞台 → 指揮役) と T0834・T0836 の振る舞いは変えていない。
- 操作ウィンドウ: `src/lib/sceneLoading.svelte.ts` の `SceneLoading` (後から始めた切り替えが勝つ・150 ms より早く返れば出さない)。
  `Control.svelte` がシーンを押す・出しているシーンの編集・モデルを選ぶ・プロファイルを開く・前のプロファイルを開き直すを包み、
  `ScenesPanel.svelte` が読み込んでいるシーンに「読み込み中」の印 (`aria-busy`) を付け、一覧の下の案内 (`role="status"`) を
  「シーンを読み込んでいます…」に替える。`locales/{en,ja}.json` に `scenes.loading`・`scenes.loadingStatus`。
- 文書: docs/studio.md の「シーンの切り替えと舞台のロック」(読むときに組み立てる・捨てる・操作ウィンドウの読み込み中) と
  「メインのスレッドを塞がない」。apps/studio/README.md の構成。
- テスト: `api/tests.rs` の `answers_while_a_puppet_is_assembled_and_drops_the_stale_one` (組み立てる関数を差し替え、組み立ての間に舞台と
  指揮役のロックが空いていること・`scene.list`/`studio.state` が答えること・その間の `scene.activate` が勝ち、先の組み立ての結果を差し込まない
  (指揮役の `revision` が 1 つだけ進む) こと、次の切り替えが無ければ差し込むこと)。`conductor.rs` の既存のテストは `assemble` を通す形にした。
  Vitest: `src/lib/sceneLoading.svelte.test.ts` (遅らせて出す・早く終われば出さない・後の切り替えが勝つ・失敗で消える)、
  `src/ScenesPanel.svelte.test.ts` (印と案内の切り替え、シーンが分からないとき)。

確かめた範囲・確かめていないこと:

- WSL2 で `just ci`・`just crate ugoe-studio`・`just vcam-check` が通る。
- `just studio` を手で動かしての確認と `just ui-audit` はしていない (振る舞いはテストで確かめた)。大きなモデルでの実際の時間は測っていない。

やらなかったこと:

- 同じモデルのままモジュールの組だけが変わるとき (着せ替え) の合成は、今のパペットに付け直すので差し込む側 (ロックの中) のまま
  (ロックの外で行うにはパペットの写しが要る)。
- ホットキーと外部 API の切り替えの読み込み中は操作ウィンドウに出さない (操作ウィンドウは始まったことを知らない)。

起票の候補:

- ホットキー・外部 API が始めた切り替えも操作ウィンドウに読み込み中を出す (バックエンドが切り替えの始まりと終わりを通知する)。
- 大きなモジュールの着せ替えが重ければ、モジュールの合成もロックの外で行う (パペットを写してから付ける、など)。
