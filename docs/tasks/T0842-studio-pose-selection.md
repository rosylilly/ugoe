---
id: T0842
title: "スタジオ: プロファイルやシーンでポーズの要素を選んで覚える"
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: []
assignee: claude-worker-T0842
---

# T0842: スタジオ: プロファイルやシーンでポーズの要素を選んで覚える

## 目的

T0735 で、`@ugoe/player` の `Player` にポーズのグループ (`poseGroups`)・表示中の要素 (`poseShown`)・`showPose` が入り、ビューアで選べるようになった。
スタジオでは選べず、配信で衣装や持ち物の差分を切り替えられない (T0735 の候補)。

## 受け入れ条件

- [x] スタジオの操作ウィンドウでポーズの要素を選べ、出力ウィンドウ (受け手) の姿が切り替わる (指揮役から受け手への知らせ。部品か関数のテスト)
- [x] 選んだ要素をプロファイルかシーン (どちらに持つかを決めて docs/studio.md に理由を書く) に覚え、読み直したときに戻る。モデルに無い要素は捨てる。テストがある
- [x] ホットキーで要素を切り替えられるようにするかを決め、するならホットキーの動作に足した (しないなら理由をメモに書く)
- [x] docs/studio.md と docs/manual のスタジオの章を直した。画像の撮り直しが要れば T0919 のメモに書き足した
- [x] `just ci` が通る

## 参考

- T0735 のメモ (起票の候補)
- packages/player/src/pose.ts、apps/studio/src/lib/profile.ts・hotkeys.ts、docs/studio.md

## メモ

- T0038 で起票した (元の候補: T0735 (スタジオ))。

### やったこと

- **シーンに持つ** (docs/studio.md#ポーズの要素 に理由): シーンの設定に `pose` (グループごとに表示する要素のパーツの ID。モジュールのパーツは
  `<モジュール>/<パーツ>`) を足した。要素はそのシーンのモデルと着せ替えの組でだけ意味を持ち、シーンはその組を持つ単位なので。
  `profile.rs` は読むときに空と重複を除き 256 個まで。プロファイルの形式の表と例を直した (移行の説明は書いていない)。
- **出す:** `Look` (受け手への `scene` の通知) に `pose` を足した。指揮役の `present` が、合成したポーズのグループごとにシーンの要素
  (無ければ先頭の要素) を指揮役のパペットに表示させ、その並びを配る。**モデルに無い要素は捨てる** (設定には残り、次にそのシーンで選ぶと消える。
  開いただけで未保存にしないため)。モデルの無いシーンは空。
- **受け手:** 出力ウィンドウ・ブラウザソース (`Output.svelte`) は読み直し・付け直しの後、そのシーンのフレームの前に `Player.showPose`。
  仮想カメラ (`virtual_camera.rs` の `Feed::set_scene`) も同じく `pose_mut().show`。
- **選ぶ:** バックエンドのコマンド `show_pose(part)` (`Stage::begin_show_pose`: 出しているシーンの `pose` を、指揮役が表示させている要素の
  そのグループだけ替えたものにして、見せ方だけの編集として出し直す)。モデルに無い要素は `scene.unknown_pose`。操作ウィンドウの通知 `scene` に
  `poseGroups` を足し、「表情とモーション」の下に「ポーズ」のパネル (`PosePanel.svelte`。SegmentedControl / 5 個以上は Select。送っている間は
  選んだ要素を先に出す) を、ポーズのグループがあるときだけ出す。
- **ホットキー:** する。`{ "kind": "pose", "part": … }` (「ポーズの切り替え」。選択肢は配信しているモデルの要素)。押すと `show_pose` と同じ。
  配信中に衣装の差分を手を離さずに切り替えたい (着せ替えのホットキーと同じ理由) ので足した。
- テスト: `profile.rs` (往復・文書の例・収め直し)、`conductor.rs` (`presenting_shows_the_scenes_pose`: 捨てる・同じグループは後のもの・
  挙がっていないグループは先頭)、`stage.rs` (`the_scene_remembers_the_chosen_pose`: 選ぶ・覚える・未知はエラー・開き直すと戻る)、
  `hotkeys.rs`・`error.rs`、`Output.svelte.test.ts` (フレームの前に表示させる)、`PosePanel.svelte.test.ts`、`HotkeysPanel.svelte.test.ts`、
  `lib/hotkeys.test.ts`。
- 文書: docs/studio.md (フレームの配信の `scene`・操作ウィンドウ・「ポーズの要素」の節・プロファイルの形式・受け手・舞台のロック・ホットキー)、
  docs/manual/studio.md (「ポーズ」の節・画面・ホットキー)、docs/manual/dress-up.md (スタジオへの案内)、apps/studio/README.md (コマンドとエラーのコード)。
  画像の撮り直しは T0919 のメモに書き足した。
- `just ci` が通った。`just ui-audit --only studio --quick` も流し、偽のホストのポーズのグループでパネルが出て、指摘が 0 件だった。

### 範囲の外の小さな変更

- `tests/ui-audit/lib/studio-host.mjs`: 偽のホストの `scene` の通知に `pose` と `poseGroups` (偽のグループ) を足した (ui-audit でパネルを出すため)。

### ついでに直したこと

- docs/studio.md の「受け手はモーションのパーツの不透明度のカーブを使わない (T0133)」と、docs/manual/studio.md の制限
  「パーツの表示を切り替えるモーションは出力に出ない」は T0135 で古くなっていたので直した・消した。

### やらなかったこと

- 操作 API (`docs/studio-api.md`) からポーズを選ぶメソッドは足していない (Stream Deck などから替えたいなら後続)。
- パネルの表示はモーションに追従しない (モーションがカーブで同じグループの要素を求めている間はモーションが勝つが、欄は選んだ要素のまま。ビューアと同じ)。
- モデルを読み直したシーンでも、選んだ要素へは先頭の要素からフェードで切り替わる (`PosePlayer` にフェードを飛ばす口が無い)。
- 仮想カメラの `Feed::set_scene` のポーズはテストしていない (GPU が要る。`virtual_camera_gpu.rs` は CI の GPU のジョブ)。指揮役と受け手の一致の検査
  (`follow_match.rs`) にもポーズの切り替えは入れていない。
- 実機の WebView・実際の配信での見た目は確かめていない (ヘッドレスの Chrome の ui-audit だけ)。見本のプロファイル (`scripts/sample-profile.mjs`) に
  ポーズのホットキーは足していない。

### 起票の候補

- 操作 API に `pose.list`・`pose.set` (配信しているシーンのポーズのグループと要素を返す・選ぶ) を足し、studio-api.md と JSON Schema を直す。
- wasm・ネイティブの `PosePlayer` に、フェードを飛ばして表示する口 (モデルを読み直したシーンで選んだ要素から始める) を足す。
- `follow_match.rs` にポーズの切り替え (シーンの `pose` を替えた出し直し) を入れ、指揮役と受け手の不透明度が揃うことを確かめる。
- ビューアの `PosePanel.svelte` とスタジオの `PosePanel.svelte` は欄の作り方が同じ。`@ugoe/player` か `@ugoe/ui` の部品にまとめる。
- 見本のプロファイルの「着せ替え」のシーンに、手を振る差分 (`Wave`) のポーズのホットキーを足す (マニュアルの見本の表も)。

