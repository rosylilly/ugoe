---
id: T0841
title: "スタジオ: ホットキー・外部 API が始めた切り替えにも読み込み中を出し、重いモジュールの合成をロックの外で行う"
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0839, T0840]
assignee: claude-worker-T0841
---

# T0841: スタジオ: ホットキー・外部 API が始めた切り替えにも読み込み中を出し、重いモジュールの合成をロックの外で行う

## 目的

T0837 でパペットの組み立て (`Conductor::present`) を舞台のロックの外で行い、操作ウィンドウから始めた切り替えに読み込み中を出した。次が残っている (T0837 の候補)。

1. ホットキーや外部の操作 API が始めたシーンの切り替えでは、操作ウィンドウに読み込み中が出ない。バックエンドが切り替えの始まりと終わりを通知する。
2. 大きなモジュールの着せ替えが重ければ、モジュールの合成もロックの外で行う (パペットを写してから付ける、など)。まず時間を測ってから決める。

同じ apps/studio のバックエンドを触る T0839・T0840 の後に着手する。

## 受け入れ条件

- [x] ホットキー・操作 API から始めた切り替えでも、操作ウィンドウに読み込み中が出て、終わる (失敗でも) と消える。通知の形を docs/studio.md に書き、テストがある
- [x] 見本の着せ替え (`assets/samples/dress-up`) と大きめのモジュールで、モジュールの合成がロックを握る時間を測ってメモに書いた。長ければロックの外で合成し、そのテストがある
- [x] `just ci` が通る

## 参考

- T0837 のメモ (起票の候補)
- docs/studio.md、ADR 0028

## メモ

- T0032 で起票した (元の候補: T0837 (2 つ))。

### 完了時 (claude-worker-T0841)

やったこと:

- `stage.rs`: `Stage::loading()` (舞台のロックの外で読んでいる切り替えのうち最後に始めたもののシーンの ID)。始めるとき (`plan`) に
  番号と ID を覚え、最後に始めた切り替えが終わった (`finish` が `Again` 以外を返した) ら消す。番号が今の番号でなければ `None`
  (後から始めた切り替え・新しいプロファイル・プロファイルを開くで自然に消える)。
- `lib.rs`: 操作ウィンドウへの `Monitor::SceneLoading { scene }` (`{"kind":"sceneLoading","scene":"…"|null}`)。`with_stage` の前後で
  `Stage::loading()` が変わったときだけ送る (読むファイルが無く握ったまま差し込む切り替えは送らない)。`subscribe_status` でも送る。
  ロックの順 (舞台 → 指揮役) と「後から始めたものが勝つ」は変えていない (通知は舞台を握ったまま送る)。
- 操作ウィンドウ: `SceneLoading.notice(scene)` (バックエンドの通知。`track` と同じく 150 ms 遅らせ、後から始めた切り替えが勝つ。
  `null` は通知の後に操作ウィンドウが始めた切り替えの表示を消さない)。`host.ts` の `MonitorMessage`、`Control.svelte` の受け口。
- 文書: docs/studio.md の「シーンの切り替えと舞台のロック」に通知の形と、合成の時間を測った結果。「メインのスレッドを塞がない」の該当行。
- テスト: `stage.rs` の `the_latest_switch_is_loading_until_it_ends`、`api/tests.rs` の `tells_the_control_window_while_a_switch_is_loading`
  (外部 API・ホットキーの切り替えで読み始めと終わり、失敗でも終わり、同じモデルでは送らない、読んでいる間の後の切り替えで終わりを 1 回だけ)、
  `lib.rs` の `monitor_messages_are_tagged` に形。Vitest: `sceneLoading.svelte.test.ts` に通知の 2 件。

モジュールの合成の時間 (WSL2、`Conductor::present(None, modules)` = ロックの中で行う付け直し。3 周目の値):

| 付け替え | リリース | デバッグ |
| --- | --- | --- |
| 見本 (`assets/samples/dress-up`: CatEars・LongHair+Sailor・CatEars+Sailor+TwinTails・外す・Sailor) | 0.2〜0.6 ms | 0.6〜1.0 ms |
| 大きめ (TwinTails のメッシュを 41 倍に複製: メッシュ 246 個・頂点 4,674 個) | 約 1.7 ms | 約 3.4〜4.0 ms |
| (参考) avatar.ugoe (メッシュ 24・頂点 1,410) の `Puppet::new` | 0.26〜0.35 ms | 約 1.3 ms |

刻みの間隔 (60 回/秒で 16.7 ms) より十分に短いので、合成はロックの中のまま (ロックの外へ出すにはパペットの写しか、`ugoe` の
合成を分ける公開 API が要る。`crates:` の外)。測るコードは一時的なテストで、コミットしていない。

確かめた範囲・確かめていないこと:

- WSL2 で `just ci`・`just crate ugoe-studio`・`just vcam-check`・Vitest・svelte-check が通る。
- `just studio` を手で動かしての確認はしていない (振る舞いはテストで確かめた)。本体も大きくスロットを持つモデルでの合成の時間は測っていない
  (見本に該当するものが無い。合成は本体を含めて組み直すので、本体のコンパイルの時間が加わる)。

起票の候補:

- 本体の大きい着せ替えモデルで合成が刻みの間隔に近づくなら、`ugoe` に合成だけを行う公開 API (`Wardrobe::compose` を外に出す等) を足し、
  ロックの外で合成する。

