---
id: T0836
title: "スタジオ: 受け手の読み込みの失敗を残らず知らせる (仮想カメラ・背景の画像・KTX2・GPU のエラーの理由)"
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: []
assignee: claude-worker-T0836
---

# T0836: スタジオ: 受け手の読み込みの失敗を残らず知らせる (仮想カメラ・背景の画像・KTX2・GPU のエラーの理由)

## 目的

T0825 で出力ウィンドウの受け手が読めなかったモデル・モジュールを `loadFailures` で知らせるようにしたが、残りがある。

- 仮想カメラの受け手 (`src-tauri/src/virtual_camera.rs`) がシーンのモデル・モジュールを読めなかったときは知らせない (T0825 の候補)。
- 背景の画像を受け手がデコードできなかったとき (`Image.decode` など) の知らせが無い (T0825 の候補)。
- 仮想カメラは PNG だけをデコードする。`ugoe_headless::image::decode_image*` で KTX2 も読む (T0222 の候補)。
- 仮想カメラの描画・読み戻しのエラー (GPU の検証エラー・デバイスの喪失など) の理由が `os` (「OS がエラーを返しました」) になり、誤解を招く。GPU の理由に分ける (T0832 の候補)。

## 受け入れ条件

- [x] 仮想カメラの受け手が読めなかったモデル・モジュールが、出力ウィンドウと同じ `loadFailures` で操作ウィンドウに出る。テストがある
- [x] 背景の画像をデコードできなかったことが受け手から知らされ、操作ウィンドウに出る。テストがある
- [x] 仮想カメラが KTX2 の画像を読む (非圧縮。T0222 で読めるもの)。テストがある
- [x] 仮想カメラの GPU のエラーが `os` でない理由 (例えば `gpu`) で止まり、`locales/{en,ja}.json` に文がある。テストがある
- [x] docs/studio.md を更新した
- [x] `just ci` が通る

## 参考

- T0825・T0832・T0222 のメモ (起票の候補)、T0835 (コマンドのエラーの形)

## メモ

- T0026 で起票した (元の候補: T0825 の 2 つ、T0832 の GPU の理由、T0222 のスタジオの KTX2)。

### 完了時 (claude-worker-T0836)

やったこと:

- 仮想カメラ (`src-tauri/src/virtual_camera.rs`):
  - `Feed::set_scene` が `Result<Option<LoadFailure>, FeedError>` を返す。モデル・モジュール・背景の画像を受け取れない (`unavailable`)・
    読めない (`unreadable`) ときは止まらず、スレッドの知らせ `Event::LoadFailure` → `Studio::camera_event` → `report_load_failure("virtualCamera", …)`
    で出力ウィンドウと同じ `loadFailures` に出す。出力ウィンドウと同じ扱い: モデルを読めなければ前のモデルを残し (このシーンのフレームは `Ahead` で
    当てない)、モジュールは 1 つずつ読んで読めない・本体に付けられないものを飛ばし (`attach_modules`)、背景の画像は設定の色で埋める。
    読めなかったものは見せ方だけの次のシーンでも読み直す (`modules_revision` を `Option` にし、壊れた背景の画像を覚えない)。
    1 つのシーンで複数あれば、モデル・モジュール・背景の順に最初のものを返す (バックエンドは受け手ごとに 1 つ覚えるため)。
  - 止まった (`Stopped`・`Failed`) ら `virtualCamera` の報告を捨てる。`SceneFiles.modules` を `Vec<Option<…>>` にしてシーンの番号を保つ
    (以前は `filter_map` で受け取れないものを詰めていた)。
  - モデルとモジュールのテクスチャ・法線マップを `decode_image*` で読む (PNG と非圧縮の KTX2)。読めない形式の KTX2 は wasm と同じく白・法線マップ無し、
    壊れた画像は読めないモデル・モジュール。モデルは `resolve_textures` をやめた (BC3 などの KTX2 のモデルを丸ごと読めなくしないため)。
  - `FeedError { Gpu, Model }` を足し、描画・読み戻し・描画の準備の GPU のエラーを `gpuError` で止める (`os` は OS の部品のエラーだけ)。
    `gpuError` のときは GPU を手放して次の開始で作り直す (デバイスの喪失なら使い回せないため。常駐の描画はふつうに落とす)。
    `locales/{en,ja}.json` に `virtualCamera.errors.gpuError`、`VirtualCameraPanel.svelte` の `errorText` に足した。
  - **範囲の中の小さな修正:** `Studio::stop_camera` が `camera` のロックを握ったままスレッドの終わりを待っていた。仮想カメラのスレッドが知らせで
    指揮役を握るようになったので、指揮役 → `camera` の順に握る刻み・`send_scene` と行き詰まらないよう、放してから待つようにした。
- 出力ウィンドウとブラウザソース (`Output.svelte`): 受け取った背景の画像を、CSS に渡すのと同じ blob URL で `HTMLImageElement.decode`
  (`src/lib/decodeImage.ts`) させて確かめ、できなければ `background` の `unreadable` を返して出さない (次のシーンの通知で取り直す)。
- 文書: docs/studio.md の「受け手が読めなかったとき」(仮想カメラ・デコード・1 受け手 1 つ・取り直しの知らせ直し)、「仮想カメラ」の止まった理由の表、
  「描き方」(KTX2 と読めないときの扱い)、「GPU が止まったとき」(`gpuError`)。apps/studio/README.md の構成。
- テスト: `virtual_camera.rs` (KTX2・BC3・壊れた画像のモデルとモジュール、モジュールを飛ばして最初を知らせる、背景の画像の理由と読み直し、
  `FeedError::code`)、`api/tests.rs` (仮想カメラの知らせが `loadFailures` に出て止まったら消える、モジュールの番号を保つ)、
  `tests/virtual_camera_gpu.rs` の `the_camera_feed_reports_what_it_cannot_read` (`gpu-tests`。lavapipe で通る)、
  `Output.svelte.test.ts` (デコードできない背景を知らせて取り直す)、`VirtualCameraPanel.svelte.test.ts` (`gpuError` を OS のエラーと分けて出す)。

確かめた範囲・確かめていないこと:

- WSL2 で `just ci`、`just crate ugoe-studio`、`cargo test -p ugoe-studio --features gpu-tests --test virtual_camera_gpu` (lavapipe)、`just vcam-check` が通る。
- 実際の仮想カメラ (v4l2loopback・Windows・macOS の機器) では動かしていない (この環境に機器が無い)。`Feed` (受け手と描画) は GPU のテストで、
  スレッドの知らせからバックエンドまでは `camera_event` を直接呼ぶテストで確かめた。`run` の中の `gpuError` で止まる経路は、実際の GPU のエラー
  (デバイスの喪失) を起こせないので、コードの対応 (`FeedError::code` のテスト) だけで確かめた。
- `HTMLImageElement.decode` は happy-dom でデコードしないので、テストでは差し替えた。実際の WebView (WebView2・WKWebView・WebKitGTK)
  と OBS のブラウザソースでは確かめていない。

やらなかったこと:

- 背景の画像の KTX2 (背景は PNG・JPEG・WebP だけを受ける。舞台が拡張子で確かめる)。
- 受け手ごとに複数の読めなかったことを覚える形 (今は 1 つ。T0825 の形のまま)。

起票の候補:

- 受け手ごとに読めなかったことを 1 つでなくすべて出す (モデルは読めたが背景とモジュールも読めない、などを同時に知らせる)。
