---
id: T0840
title: "スタジオ: 受け手ごとに読めなかったものを 1 つでなくすべて知らせる"
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: []
assignee: claude-worker-T0840
---

# T0840: スタジオ: 受け手ごとに読めなかったものを 1 つでなくすべて知らせる

## 目的

T0836 で受け手 (仮想カメラ・出力ウィンドウ) の読み込みの失敗を知らせるようにしたが、1 つの受け手で読めないものが複数あるとき
(モデルは読めたが背景の画像とモジュールも読めない、など)、最初の 1 つだけを知らせる (T0836 の候補)。すべてを集めて知らせる。

## 受け入れ条件

- [x] 1 つの受け手で複数のもの (モデル・モジュール・背景の画像) が読めないとき、すべてが操作ウィンドウに出る。知らせの形 (イベントの中身) を docs/studio.md に書いた
- [x] 複数の失敗を集めるテスト (Rust) と、操作ウィンドウの表示のテストがある
- [x] `just ci` が通る

## 参考

- T0836 のメモ (起票の候補)
- docs/studio.md

## メモ

- T0032 で起票した (元の候補: T0836)。

### 完了時 (claude-worker-T0840)

やったこと:

- バックエンド (`load_failure.rs`): 受け手ごとに 1 つでなく、今のシーンで読めなかったものをすべて覚える (`LoadFailures` を
  ラベル → `Vec<LoadFailure>` に)。同じもの (モデル・同じ番号のモジュール・背景の画像) の報告は置き換え、変わらなければ配り直さない。
  並びはモデル・モジュール (番号の順)・背景の画像 (`Asset` に `Ord` を足した)。まとめて受ける `report_load_failures` を足した
  (仮想カメラ用。捨てるものが混じっても残りは受ける)。受け手が去った・仮想カメラが止まったときは、その受け手の分をすべて捨てる。
- 知らせの形 (`loadFailures` の `failures`) は平らな並びのまま (同じラベルが複数並ぶようになった)。報告の口 (`report_load_failure`・
  WebSocket の `loadFailure`) も 1 つずつのまま (互換)。
- 仮想カメラ (`virtual_camera.rs`): `Feed::set_scene` が `Vec<LoadFailure>` を返し、`Event::LoadFailure` を `Event::LoadFailures(Vec)` にした。
  `attach_modules` は飛ばしたモジュールをすべて番号の順に返す。モデルを読めなければ、モジュールと背景の画像はそのシーンでは読まないので、
  返すのはモデルだけ (以前と同じ扱い)。
- 出力ウィンドウ・ブラウザソース (`Output.svelte`): 飛ばしたモジュールをすべて知らせる。**振る舞いの変更:** 受け取れないモジュールが
  あるとモデルを読まずに止まっていたのを、仮想カメラと同じく飛ばしてモデルと残りのモジュールで出すようにした (そうしないと背景の画像などを
  同時に知らせられない。docs/studio.md の「モジュールを読めなければ、モデルと読めたモジュールで出す」にも合う)。
- 操作ウィンドウ: `ReceiverFailuresNotices.svelte` を足し、受け手ごとに 1 つの知らせにして読めなかったものを 1 行ずつ並べる
  (`receivers.ts` の `groupLoadFailures`)。閉じるとその受け手の分をすべて消す。以前の `{#each … (failure.label)}` は同じラベルが
  並ぶと重複したキーになるので置き換えた。
- 文書: docs/studio.md の「受け手が読めなかったとき」(知らせの JSON の例・並び・置き換え・モデルを読めなかったとき・出力ウィンドウの
  モジュールの扱い)、apps/studio/README.md の構成。
- テスト: `api/tests.rs` の `every_load_failure_of_a_receiver_is_kept` (1 つずつとまとめての報告・置き換え・並び・去ったら捨てる・
  シーンの切り替え)、`virtual_camera.rs` の `unreadable_modules_are_skipped_and_all_are_reported`、`tests/virtual_camera_gpu.rs`
  (モジュール 2 つと背景の画像を同時に返す)、`Output.svelte.test.ts` (モジュールと背景をすべて知らせる・モデルを読めなければモデルだけ・
  受け取れないモジュールを飛ばす)、`ReceiverFailuresNotices.svelte.test.ts` (受け手ごとの知らせにすべて出る・閉じる)、
  `receivers.test.ts` (`groupLoadFailures`)。

確かめた範囲:

- WSL2 で `just ci`・`just vcam-check`・`cargo test -p ugoe-studio --features gpu-tests --test virtual_camera_gpu` (dzn) が通る。
  `just ci` の 2 回目は `doc-check` が `target/doc/ugoe` の削除で失敗した (cargo doc の中の競合。変更と無関係) が、流し直して通った。
- 実際の仮想カメラの機器・WebView・OBS では動かしていない (T0836 と同じ)。

やらなかったこと:

- モデルを読めないときに、モジュールと背景の画像も取って知らせること (そのシーンでは付けない・出さないため、知らせない)。

起票の候補:

- なし。
