---
id: T0816
title: コードレビュー: スタジオの仮想カメラ (vcam)
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: []
assignee: claude-worker-T0816
---

# T0816: コードレビュー: スタジオの仮想カメラ (vcam)

## 目的

スタジオの仮想カメラ (vcam) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `apps/studio/vcam` のすべて (Windows の Media Foundation・Linux の v4l2loopback・macOS)

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - すべての `unsafe` に `// SAFETY:` があるか、COM の参照の数・解放・スレッドの前提 (ADR 0028)
  - unsafe を使う範囲が最小か (安全な包みの外に漏れていないか)、デバイスが無い・奪われたときの扱い
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0829〜T0833 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)。使ったのは T0829〜T0831。
- 範囲: `apps/studio/vcam` のすべて (`src/lib.rs`・`shared.rs`・`v4l2.rs`・`mf/mod.rs`・`mf/source.rs`・`cmio.rs`・`macos/` の Swift と plist)。
  Windows と macOS の Rust のコードは `cargo clippy -p ugoe-studio-vcam --all-targets --target x86_64-pc-windows-msvc` /
  `aarch64-apple-darwin` (`-D warnings`) で直す前と後に検査した (どちらも通る)。Windows と macOS のテストは動かせない (WSL) ので、
  OS に依らない判断は `shared.rs`・`lib.rs` の safe な関数に出してテストした。Swift はコンパイルできないので直していない (T0830)。
- `unsafe`: すべてのブロックに `// SAFETY:` がある (`undocumented_unsafe_blocks` を拒否にしていて、3 つのターゲットで通る)。
  `unsafe` は `mf`・`cmio`・`v4l2` の中に閉じていて、外 (`VirtualCamera`・`availability`・`shared`) は safe な API だけ。
  COM の参照: `IMFVirtualCamera::from_raw` で +1 を引き取り、`Drop` で `Stop`→`Shutdown`→`MFShutdown`→`CoUninitialize` の順。
  `CoInitializeEx` が `RPC_E_CHANGED_MODE` のときは釣り合わせない。`ActivateObject`・`CreateInstance`・`DllGetClassObject` は `query` で
  参照を足して返す。メディアソースの COM の入口はどれも `guard` (`catch_unwind`) で包み、リリースのプロファイルは `panic = unwind`。
  `Source` と `Stream` は互いを持つ (循環) が、`Shutdown` で切る。スレッド: `Stream`・`Source`・`Activate` は `Mutex` で守り、
  `ThreadingModel` は `Both`。スタジオ側の `mf::Camera` は生のハンドルを持つので `Send` でなく、作ったスレッドで落ちる (`CoUninitialize` の前提)。
  macOS の `unsafe impl Send for Camera` は `CMSimpleQueue` の 1 つの書き手と 1 つの読み手の約束に頼り、理由が書いてある。
- 機器が無い・奪われたとき: Linux は開くときに `noDevice` / `busy`、書き込みの失敗で止まる (`os`)。Windows は共有メモリが開けなければ
  1 秒ごとに開き直し、受け取るアプリがいないとみなす。macOS は拡張の機器が無ければ有効にし直して 5 秒待つ。拡張が落ちた後は
  キューが取り出されないので「アプリがいない」のまま (T0830 の 4.)。

### 所見

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 中 | `src/shared.rs:281` (`Reader::read`)・`:152` (`scale_nv12_nearest`) | 共有メモリの頭の幅と高さ (対話でログオンした利用者なら書ける) を確かめず、0 だと最も近い画素の拡大縮小が空のスライスを添字で引いてパニックする。`guard` が `E_FAIL` にするが、カメラは画を出さなくなる。奇数の大きさも NV12 として通っていた | 直した (読み手が 0・奇数を拒み、拡大縮小は大きさ 0 で何もしない。先に失敗するテストを 2 つ足した) |
| 中 | `src/mf/source.rs:105` (`offered_config`)・`SetMediaType`・`current` | 受け取るアプリが選んだ media type の大きさとフレームレートをそのまま信じる。出していない大きさ (巨大なもの) なら Frame Server の中で巨大な画を確保する (確保の失敗は `catch_unwind` で捕まらず落ちる) | 直した (`shared::decode_media_type` で選べる組だけを受け、ほかは `MF_E_INVALIDMEDIATYPE`。テストを足した。Windows の上では動かしていない) |
| 中 | `src/cmio.rs:160` (`send`) | キューが埋まっていれば 1 枚ごとに「アプリがいない」にする。受け取る側のフレームレートがスタジオより低いと (30 と 60)、2 枚のキューは 1 枚おきに埋まるので、知らせが明滅し、状態の変化の通知がフレームごとに出る | 直した (Windows と同じ 2 秒の判断を `lib.rs` の `ConsumerWatch` にまとめ、両方で使う。テストを足した) |
| 中 | `src/mf/mod.rs:306` (`send`) | `Writer::new` の失敗を `.ok()` で握りつぶす。共有メモリの形の版がメディアソースの DLL と違うと (入れ直しの途中など)、ずっと「アプリがいない」に見える | 直した (頭を書いている途中 (`Magic`) だけ開き直し、版・大きさの違いは `os` のエラーで止める) |
| 中 | CI (`scripts/ci-groups.mjs` の `apps`) | `unsafe` の大半 (Windows と macOS) が CI で一度もコンパイルされず、`undocumented_unsafe_blocks` が効いていない | T0829 を起票 |
| 中 | `macos/Extension/Camera.swift` (`CameraStream`) | `activeFormatIndex`・`frameRate` をクライアントのキューで書き、画のキューで読む (データ競合)。動いている間にフレームの長さが変わってもタイマーを作り直さない | T0830 を起票 (Swift のコンパイルと実機が要る) |
| 中 | `src/cmio.rs` (`start`)・`Camera.swift` | スタジオは受ける側のストリームの書式 (既定 1280×720) を設定に合わせずに 1920×1080 の画を積む。受け取られるかは確かめていない | T0830 を起票。`docs/studio.md` の macOS の未確認に 9. を足した |
| 低 | `src/cmio.rs:114` (`start`) | プロセスで最初に始めたとき、有効にしたばかりで機器がまだ現れていないと、待たずにもう一度有効にする求めを出す | 直した (`activate` が求めたかを返し、求めたばかりなら待つだけにする) |
| 低 | `src/mf/source.rs:351` (`RequestSample`) | 1 枚ごとに画 (最大 3 MiB) を複製してからサンプルのバッファに写す | 直した (状態を握ったまま、置いた画から直接写す) |
| 低 | `src/cmio.rs:564` (`finished`) | 再起動の後に有効になるときを `extensionFailed` と英語の詳細で知らせる (利用者のすべきことは「再起動」と決まっている。ADR 0029) | T0831 を起票 |
| 低 | `src/v4l2.rs:195` (`find_device`) | 開けない機器を飛ばすので、権限が無い (`EACCES`) と `noDevice` になり、v4l2loopback の入れ方を案内してしまう | T0831 を起票 |
| 低 | `src/shared.rs:245` (`Writer::write`) | 幅・高さ・フレームレートは枠ごとでなく 1 組で、枠の版 (seqlock) の外にある。大きさを変えて始め直した直後の 1 枚だけ、前の枠を新しい大きさとして読むことがある | 見送り (起こるのは始め直しの 1 枚だけで、大きさは上限の中なのではみ出さない。直すには共有メモリの形 (`VERSION`) を変える) |
| 低 | `src/v4l2.rs:195` (`find_device`) | loopback の機器を探すために、本物のカメラも含めて `/dev/video*` を読み書きで開く (機器によってはランプが点く) | 見送り (開くのは始めるときだけ。sysfs から loopback の機器を見分ける確かな方法が無い) |
| 低 | `src/mf/mod.rs:348` | `CoUninitialize` を同じスレッドで呼ぶ前提を「`VirtualCamera` は `Send` でない」に頼るが、型で固定していない (生のハンドルを持つので今は `Send` にならない) | 見送り (今は成り立つ。`Mapping` に `Send` を足すときに見直す) |
| 低 | `src/lib.rs` | `Delivery::Delivered` の doc が macOS に触れていない・`VirtualCamera::config` に doc が無い | 直した |

### 横断

- **OS ごとの `cfg` のコードが CI で検査されない:** `#[cfg(windows)]`・`#[cfg(target_os = "macos")]` の中は ubuntu の CI では型検査も
  clippy も通らない。ほかのアプリ (エディタ・ビューア・スタジオの `src-tauri`) の OS ごとのコードにもありそう (T0829 で vcam だけ足す)。
- **外から来た大きさを信じない:** 共有メモリ・COM の引数 (media type)・IPC で受け取る大きさは、確保と添字の前に上限と 0 を確かめる。
  `catch_unwind` はパニックを捕まえるが、確保の失敗 (中止) は捕まえない。
- **1 枚ごとの結果で状態を決めない:** フレームごとの成否を利用者への知らせにそのまま写すと、速さの違いで明滅する。時間で均す。
- **握りつぶし (`.ok()`) が「いつもの状態」に見える:** 失敗が「受け取るアプリがいない」などのふつうの状態に化けると、利用者にも
  開発者にも分からない。待てば直る失敗と直らない失敗を分ける。

### `just ci`

- 1 回目は `web-check` の `@ugoe/editor` の `src/lib/preview/tracking.test.ts` が `beforeAll` の 10 秒の時間切れで落ちた
  (`Error: Hook timed out in 10000ms.`)。並行するワーカーのビルドで負荷が高く (load average 58)、このタスクは `apps/editor` を触っていない。
  そのテストだけを流すと通り (11.6 秒)、`just web-check i18n app-build-check` (`ci` の残り) を流し直すと通った。それより前の
  `fmt-check lint test layers groups licenses publish-check spec` は 1 回目で通っている。
