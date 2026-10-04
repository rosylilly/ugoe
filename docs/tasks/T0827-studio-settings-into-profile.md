---
id: T0827
title: スタジオ: 仮想カメラの設定と刻みの速さをプロファイルに持つ
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0815]
assignee: claude-worker-T0827
---

# T0827: スタジオ: 仮想カメラの設定と刻みの速さをプロファイルに持つ

## 目的

仮想カメラの設定 (解像度・フレームレート・透過を埋める色。`lib/virtualCamera.ts`) と刻みの速さ (`lib/settings.ts`) は、
操作ウィンドウの localStorage に覚えている。コメントには「プロファイル (T0802) ができたらそちらへ移す」とあるが、プロファイルは
できている (T0815 のレビューで見つけた。docs/studio.md も「プロファイルへ移すときも省ける項目として足す」としている)。
配信ごとに出力の形を切り替えられるよう、プロファイルの省ける項目として持つ。古いプロファイルは既定の値で読む。

## 受け入れ条件

- [x] プロファイルの形式 (profile.rs・lib/profile.ts・docs/studio.md#プロファイルの形式) に省ける項目として足した。版の扱いを決めた
- [x] localStorage に覚えた値は、初めてプロファイルに持つときに引き継ぐか捨てるかを決め、docs/studio.md に書いた
- [x] 外部 API・ブラウザソースへの影響を確かめた
- [x] `just ci` が通る

## 参考

- docs/studio.md#プロファイルの形式・#刻みの速さと正確さ・#仮想カメラ

## メモ

- **形:** `output.tickRate` (整数、既定 60) と `output.virtualCamera` (`width`・`height`・`fps`・`background`。色はほかの項目と同じく `#rrggbb`、既定 1280×720・30・`#f0f0f0`)。
  `profile.rs` の `OutputSettings`・`VirtualCameraSettings`、`lib/profile.ts` の `OutputSettings`、docs/studio.md#プロファイルの形式 の例と表。
  選べない値は読むときに項目ごとに既定へ戻す (`OutputSettings::sanitize`)。
- **版:** 1 のまま (省ける項目を足すだけ。docs の「足すときの規則」どおり)。後方: 項目の無いプロファイルは既定で読む
  (`reads_profiles_without_the_output_settings_as_defaults`・文書の例のテスト)。前方: `output` も `virtualCamera` も
  `deny_unknown_fields` ではないので、前の版のスタジオは新しい項目を読み飛ばす (保存し直すと消える。`ignores_unknown_output_settings` で
  知らない項目を読み飛ばすことを確かめた)。
- **localStorage:** 引き継がずに捨てる。起動時に `ugoe-studio.tick-rate`・`ugoe-studio.virtual-camera` を消す (`forgetLegacySettings`)。
  理由は docs/studio.md#プロファイルの形式 (プロファイルの値を開く計算機に依らせない・スタジオは未公開)。
- **経路:** 刻みの速さ・仮想カメラの設定とも、操作ウィンドウは T0824 の `PendingEdit` で送る (`Control.svelte` の `tickRateEdit`、
  `VirtualCameraPanel.svelte` の `edit`。続けて変えても消えないことを `VirtualCameraPanel.svelte.test.ts` で確かめた)。
  刻みのスレッドが読む `Studio::tick_rate` は `with_stage` のたびにプロファイルから写す (開く・新しくする・変えるのどれでも効く)。
- **仮想カメラに効かせるのはバックエンド (`lib.rs` の `sync_camera`):** パネルで変えたとき (`set_virtual_camera_settings`。
  `set_virtual_camera_background` を置き換えた) と、プロファイルを開いた・新しくしたとき、動いている仮想カメラと設定を比べ、
  解像度かフレームレートが違えば始め直し、色だけなら次の画から。止まった (失敗した) カメラは始め直さない。
  そのため `new_profile` を `async` (spawn_blocking) にした。`start_virtual_camera` はプロファイルの設定で始める。`settings` は省ける引数として残した
  (`tests/e2e/specs/studio.test.mjs` が渡している。渡すと先にプロファイルの設定にする)。
- **外部 API・ブラウザソース:** 外部 API のメソッドと通知は刻みの速さも仮想カメラも扱わず、`StageSnapshot` は `output` を見ないので、
  これらを変えても `scene.listChanged` などは出ない (影響なし)。ブラウザソースは刻みのフレームを受けるので、プロファイルを開くと
  その刻みの速さが実質のフレームレートになる (docs/studio.md#刻みの速さと正確さ に書いた)。プロトコルとページは変えていない。
- **確かめていないこと:** 実機の仮想カメラ (Windows・macOS・v4l2loopback) で、プロファイルを開いたときの始め直しを動かしてはいない
  (始め直しは前からある `start_camera` の経路。`sync_camera` の分かれ道は偽のカメラのスレッドでテストした)。
- `just ci` は 1 回目で通った (rebase の後)。
- 起票の候補: `set_virtual_camera_settings`・`start_virtual_camera` のエラーも `Result<_, String>` のまま (T0814 の範囲の、エラーをコードで返す直しに含める)。
