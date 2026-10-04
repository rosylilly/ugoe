---
id: T0831
title: 仮想カメラの止まった理由に、再起動が要る・機器を開く権限が無いを足す
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0816]
assignee: claude-worker-T0831
---

# T0831: 仮想カメラの止まった理由に、再起動が要る・機器を開く権限が無いを足す

## 目的

T0816 のレビューで、利用者のすべきことが決まっているのに、汎用の理由や誤った案内になる場合が見つかった。
`ugoe_studio_vcam::Error` に理由を足し、操作ウィンドウで文言と案内を出す (ADR 0029 の 4.: 文言はコードで選ぶ)。

1. **macOS で再起動が要る:** `OSSystemExtensionRequest` が `willCompleteAfterReboot` で終わると、いまは `extensionFailed` に
   英語の詳細 ("the camera extension will be activated after a restart") を付けるだけ (`apps/studio/vcam/src/cmio.rs` の `finished`)。
   `needsRestart` のような理由にし、「再起動してください」を出す。
2. **Linux で機器を開く権限が無い:** `/dev/video*` を開けない機器は飛ばすので (`apps/studio/vcam/src/v4l2.rs` の `find_device`)、
   v4l2loopback の機器があっても権限が無ければ `noDevice` になり、v4l2loopback の入れ方を案内してしまう。開けなかった理由が
   権限 (`EACCES`) なら、`video` のグループに入る案内を出す。

## 受け入れ条件

- [x] `Error` に 2 つの理由を足し、`Error::code` とテストを更新した
- [x] `apps/studio/locales/*.json` の `virtualCamera.errors.*` と `VirtualCameraPanel.svelte` の案内を足した
- [x] `docs/studio.md#仮想カメラ` の知らせの表と、Linux・macOS の節を更新した
- [ ] `just ci` が通る

## 参考

- docs/studio.md#仮想カメラ、ADR 0029
- T0816 のメモ

## メモ

- やったこと:
  - `ugoe_studio_vcam::Error` に `NeedsRestart` (`needsRestart`) と `NoPermission` (`noPermission`) を足した (`apps/studio/vcam/src/lib.rs`)。
  - macOS (`cmio.rs`): 求めが `WillCompleteAfterReboot` で終わったら `Activation::NeedsRestart` にし、`Error::NeedsRestart` を返す。
    再起動まで結果は変わらないので、`NeedsApproval` と同じくプロセスの中では求め直さない (前は `Failed` 扱いで毎回求め直していた)。
  - Linux (`v4l2.rs`): `/dev/video*` を開けなかった理由が `io::ErrorKind::PermissionDenied` (`EACCES`・`EPERM`) の機器があり、
    書ける loopback の機器が無く、`/sys/module/v4l2loopback` があれば `NoPermission`。開けない機器が loopback のものかは開かないと
    分からないので、モジュールが読み込まれていることで判断する (読み込まれていなければ従来どおり `noDevice` で入れ方を案内)。
    loopback の機器が書かれていれば `busy` を優先する。判断は `not_found` に出してテストした。
  - 操作ウィンドウ: `guideFor` に `noPermission` → `videoGroup` (新しい案内。`sudo usermod -aG video "$USER"` とログインし直し) を足し、
    `VirtualCameraPanel.svelte` の `errorText` と案内、`locales/{en,ja}.json` の `virtualCamera.errors.{needsRestart,noPermission}` と
    `virtualCamera.guide.videoGroup.*` を足した。`virtualCamera.test.ts` を更新した。
  - `docs/studio.md#仮想カメラ`: 知らせの表・案内の一覧、Linux の節 (権限の判断と未確認の 5.)、macOS の節 (再起動の扱いと未確認の 8.) を更新した。
- 確かめた範囲: `just crate ugoe-studio-vcam` (Linux のテスト。`not_found` と `Error::code`)、`just vcam-check` (Windows・macOS の clippy、
  `-D warnings`) が通る。`@ugoe/studio` の check と test が通る。実機 (macOS の再起動が要る入れ替え・`video` のグループに入っていない利用者で
  v4l2loopback を開く) では確かめていない (WSL に macOS も v4l2loopback も無い)。OS のエラーと理由の対応は Apple の
  `OSSystemExtensionRequest.Result` (completed / willCompleteAfterReboot の 2 つ) と open(2) の `EACCES`・`EPERM` から決めた。
- やらなかったこと: 開けない機器が loopback のものかを sysfs から見分けること (確かな方法が無い。T0816 の所見の見送りと同じ理由)。

### `just ci`

- rebase の後 (main は `d9aa9a35` の時点) の 1 回目で、`web-check` の `@ugoe/editor` の `src/lib/viewport/ToolOverlay.test.ts` の
  「deletes the selected vertices with Delete」が落ちた (`expect(selection.vertices.Quad).toEqual([])` が `[0]`)。このタスクは `apps/editor` を
  触っておらず、main の作業ツリーでも同じテストが 3 回とも落ちる (決定的。T0699 で足したテストと T0693 などの後の変更の組み合わせと思われる)。
  `web-check` の残り (viewer・studio・face-tracking・player・i18n・runtime・ui の check と test) と `just i18n app-build-check script-check` は
  流し直して通った。`web-check` より前の `fmt-check lint test layers refs groups licenses publish-check spec` は 1 回目で通っている。

### 起票の候補

- `apps/editor` の `ToolOverlay.test.ts` の「deletes the selected vertices with Delete」が main で落ちる (上)。
