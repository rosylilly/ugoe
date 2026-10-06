---
id: T0848
title: "スタジオ: 指揮役の表示中のポーズの要素を操作ウィンドウに出し、操作 API に pose.changed を足す (判断が要る)"
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: []
assignee: claude-worker-T0848
---

# T0848: スタジオ: 指揮役の表示中のポーズの要素を操作ウィンドウに出し、操作 API に pose.changed を足す (判断が要る)

## 目的

T0736 で、`@ugoe/player` の Player とエディタのプレビューのポーズの欄は、モーションが切り替えた要素に追従するようになった。スタジオの操作ウィンドウの
「ポーズ」のパネルは、指揮役 (Rust の側) のシーンに覚えた選択を出すので、モーションで手が変わっても欄は選んだ要素のまま
(docs/studio.md「パネルの表示はモーションに追従しない」。T0736 の候補)。
また操作 API (T0843 の `pose.list`・`pose.set`) には、ポーズの要素が変わったときの通知が無く、Stream Deck などのボタンの状態を合わせられない (T0843 の候補)。
どちらも指揮役の Puppet の表示中の要素 (`PosePlayer::visible_parts`) と、シーンに覚える選択をどう分けて外に見せるかで決まるので、1 つのタスクで扱う。

**判断が要る:** 案:

- A. 欄と `pose.changed` は表示中の要素 (モーションの切り替えを含む) を出す。シーンに覚える選択は `pose.list` の別の欄 (`selected` など) にする。
- B. 欄は Player と同じ規則 (後から来たほうが勝つ) で表示中の要素を出し、`pose.changed` は選択が変わったとき (操作ウィンドウ・ホットキー・`pose.set`・
  シーンの切り替え) だけ送る。モーションによる表示の変化は送らない。
- C. 欄はシーンの選択のまま (今のまま) にし、`pose.changed` だけを選択の変化で足す。

推奨は、指揮役が表示中の要素を読む頻度 (毎フレームか変化だけか) と、受け手 (出力ウィンドウ) との食い違いを調べてから書く。

## 受け入れ条件

- [x] 案と推奨をメモに書き、利用者の判断を得た
- [x] 決めた形で、操作ウィンドウの「ポーズ」のパネルの表示が変わる (C なら変えない)。指揮役の単体テストと部品のテストがある
- [x] 操作 API に `pose.changed` があり (`events.subscribe` で選べる)、決めた場合に送る。`api/` のテストがある
- [x] docs/studio-api.md の Events の表と docs/studio-api.schema.json (スキーマの検査が通る)、docs/studio.md#ポーズの要素・docs/manual/studio.md を直した
- [x] `just ci` が通る

## 参考

- T0736 のメモ (やったこと・やらなかったこと・起票の候補)、T0843 のメモ (起票の候補)、T0844 (読み直したシーンの要素をフェードせずに出す)
- apps/studio/src-tauri/src (`conductor.rs`・`api/methods.rs`)、apps/studio/src/PosePanel.svelte、docs/studio-api.md#events

## メモ

- T0041 で起票した (元の候補: T0736 (操作ウィンドウのポーズの欄の追従)・T0843 (`pose.changed`))。

### 調べたこと (T0848、判断待ち)

- **指揮役が表示中の要素を読む頻度:** 今は実行中に一度も読まない (`PosePlayer::visible_parts` を呼ぶのは `conductor.rs` のテストだけ)。
  パネル (`scene.pose`)・`pose.list` の `shown`・`begin_show_pose` の土台は、どれもシーンを出したときに `show_pose` が埋めた `look.pose`
  (= 選択) を読む。A・B にするなら `Conductor::tick` の `update` の後に毎刻み読み、前の刻みと比べて変わったときだけ印を立てる
  (`take_animation_changed` と同じ形)。グループは数個で、`visible_parts()` と覚えた並びを `iter().eq` で比べれば確保も要らず、負荷は無視できる。
- **モーションの切り替えは残る:** `PosePlayer::update_by` は要求が無くなっても表示中の要素を保つので、モーションが手を `ArmB` にすると、
  モーションが終わっても指揮役も受け手も `ArmB` のまま。シーンの選択 (`look.pose`) は `ArmA` のまま。
- **受け手との食い違い (今もある):**
  - 接続していた受け手は、フレームのパーツの不透明度のカーブを自分のポーズに当てる (`Puppet::follow`) ので、指揮役と同じ要素になる (`follow_match.rs`)。
  - **途中から来た受け手は食い違う:** 接続したときの `scene` の通知の `pose` は選択 (`ArmA`) なので、モーションが切り替えて終わった後に
    開いた出力ウィンドウ・ブラウザソース (OBS での読み直しを含む)・仮想カメラは `ArmA` を出し、指揮役と前からの受け手は `ArmB` を出す。
    次にシーンを出し直す (下) まで続く。コードを読んだ結果で、テストでは再現していない。
  - **シーンを出し直すとモーションの切り替えは戻る:** ライティングの編集・別のグループの要素の選択なども `show_pose` で `look.pose` を表示させるので、
    モーションが変えたグループは選択 (`ArmA`) にフェードで戻る (指揮役と受け手で揃う)。パネルが表示中の要素を出す (A・B) と、
    `ArmB` と出ている欄が、別のグループを選んだだけで `ArmA` に戻って見える。
- **案ごとの影響:**
  - A (欄・`pose.changed`・`pose.list` の `shown` を表示中の要素にし、選択は `selected` に分ける): 欄・API・画面に映るものが一致し、
    ビューアの Player (T0736) とも同じ。毎刻みの比較が要る。`pose.changed` はモーションの切り替えでも飛ぶ (変わったときだけなので、モーション 1 回に数回)。
    `pose.list` の結果の形と Schema が変わる。指揮役が表示中の要素を持つので、接続したときの `scene` の通知の `pose` を表示中の要素にすれば
    上の途中参加の食い違いも直せる。
  - B (欄は表示中の要素、`pose.changed` は選択の変化だけ): 欄の変更は A と同じ。ただ欄と `pose.list` の `shown`・`pose.changed` の中身が
    モーションの後に食い違い、Stream Deck のボタンは画面に映る手と違う要素を点ける。
  - C (欄は今のまま、`pose.changed` を選択の変化で足す): 変更が最小 (`api/` と文書だけ)。欄と API は一致するが、どちらもモーションの後の画面と食い違う。
    途中参加の食い違いは残る (別に直す)。
- **推奨: A。** ポーズの差分を切り替えるモーションは「手を振る → 振った手のまま」のように見た目を残すので、操作する人とボタンには画面に映るものを
  見せるほうが分かりやすい。読む費用は毎刻み数個の比較で小さく、同じ仕組みで途中参加の受け手の食い違いも直せる。
  合わせて決めたいこと: 要素を選んだとき (`begin_show_pose`) にほかのグループの土台を選択のまま (今の形。モーションの切り替えは戻る) にするか、
  表示中の要素にする (モーションの切り替えをシーンに覚える) か。A の推奨は「選択のまま」(シーンが姿を決める今の規則を保つ。欄の説明文にその旨を書く)。

### 利用者の判断

- **案 A** に決まった: 欄・`pose.changed`・`pose.list` の `shown` は表示中の要素、シーンの選択は `selected` に分ける。途中から開いた受け手の
  食い違いも同じ仕組みで直す。要素を選んだときのほかのグループは「選択のまま」(モーションが替えた要素は戻る)。欄の説明文にその旨を書く。
  リリース前なので `pose.list` の形の変更に移行の説明は書かない。

### やったこと

- **指揮役 (`conductor.rs`):** `shown_pose` (表示中の要素)・`joining_scene` (`look.pose` を表示中の要素にしたシーン)・`take_pose_changed`
  (表示中の要素か選択が前に読んだときから変わったか。変わらなければ確保しない) を足した。単体テスト `the_shown_pose_follows_motions_and_is_noticed_once`。
- **配る (`lib.rs`):** 刻みの後と `with_stage` (舞台の通知 `scene.changed`・`dressup.changed` の後、指揮役を握ったまま) で `notify_pose` を呼び、
  変わっていれば操作ウィンドウに `pose` の通知 (`shown`) と外部 API に `pose.changed` を送る。操作ウィンドウの `scene` の通知に `poseShown` を足した。
  接続したばかりの受け手 (`subscribe_frames`・ブラウザソースの `source_connect`・仮想カメラの `camera_scene`) には `joining_scene` を送る。
- **操作 API:** `pose.list` は `{parts, module, shown, selected}`。`pose.changed` (中身は `pose.list` と同じ) を `EVENTS` と Schema に足した。
  テスト: `pose_changes_are_reported_with_the_shown_and_selected_parts` (刻んでも変わらなければ送らない・モーションの切り替えで 1 回・
  選択だけの変化・シーンの切り替えで `scene.changed` の後)、`lists_and_sets_the_pose_of_the_scene_on_air` に `selected`。
- **途中参加:** `follow_match.rs` の `a_follower_joining_after_a_pose_switch_matches_the_conductor` (モーションの後に `joining_scene` で
  始めた受け手は指揮役と各フレームで揃い、選択で始めた受け手は食い違う)。
- **操作ウィンドウ:** `Control.svelte` が `poseShown` を `scene` と `pose` の通知で持ち、`PosePanel` に渡す。欄の説明文 (en・ja) に
  「欄は画面に出ているパーツ (モーションの切り替えを含み、終わっても残る)」「選ぶとほかのグループはシーンの選択に戻る」を書いた。
  部品のテスト `PosePanel.svelte.test.ts` に追従のテスト。
- **文書:** docs/studio-api.md (Pose・Events)、docs/studio-api.schema.json、docs/studio.md (ポーズの要素・外部 API の表・通知の表・操作ウィンドウの表・
  フレームの配信)、docs/manual/studio.md (ポーズ)。
- **範囲の外 (小さなもの):** `tests/ui-audit/lib/studio-host.mjs` の偽の `scene` の通知に `poseShown` を足した (無いと欄が `undefined` を受ける)。

- main に追従した後、`just ci`・`just vcam-check`・`just e2e --only studio` (13 件) が通った。

### やらなかったこと

- 操作ウィンドウの `Control.svelte` の通知の振り分けそのもののテストは無い (部品は props で確かめた。Control を描くテストが無いため)。
- 実機の Stream Deck・OBS のブラウザソースでは試していない (テストの接続と Schema・指揮役と受け手の一致の検査だけ)。
