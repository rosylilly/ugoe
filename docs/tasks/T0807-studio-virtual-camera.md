---
id: T0807
title: スタジオ: 仮想カメラ (Windows と Linux)
status: done
milestone: M8
crates: [apps/studio, ugoe-headless, ugoe-render, scripts, commitlint.config.mjs, docs, Cargo.toml, Cargo.lock]
depends_on: [T0804]
assignee: claude-worker-T0807
---

# T0807: スタジオ: 仮想カメラ (Windows と Linux)

## 目的

会議アプリや配信ソフトのカメラの入力として、スタジオの出力をそのまま選べるようにする。macOS は署名と System Extension が要るので T0809 に分ける。

## 受け入れ条件

- [x] ADR 0028 で決めた描画の場所から、一定のフレームレートで RGBA の画を仮想カメラへ渡す (背景は不透明。透過は単色で埋める)
- [x] バックエンドに ugoe-render を常駐させて受け手として描き、読み戻す (ADR 0028。ugoe-headless に常駐の描画の口を足す)
- [x] Windows: Windows 11 の仮想カメラの API (Media Foundation) で出す。COM の unsafe は ADR 0028 で決めた専用の 1 クレートに閉じ込める。登録はインストール時に管理者権限。使えない版では理由を出す
- [x] Linux: v4l2loopback の機器に書く。機器が無いときは入れ方を案内する
- [x] 開始・停止・解像度・フレームレートの設定を操作ウィンドウに置く。止まった・遅れたことを知らせる
- [x] unsafe を許す仮想カメラ専用のクレートを作る (ADR 0028 の 8.。場所は apps/studio の下)。scripts/check-layers.mjs・ci-groups.mjs・check-licenses.mjs と commitlint の scope に登録する
- [x] 作業の環境 (WSL) で確かめられない部分は、手順を docs に書いて利用者の確認に残す
- [x] 依存を足すならライセンスを確かめる (GPL-3.0 と両立)。`just ci` が通る

## 参考

- ADR 0028、docs/studio.md

## メモ

### やったこと (2026-10-02)

- **常駐の描画の口 (`ugoe-headless`):** `Resident` (レンダラ・テクスチャ・描画先・読み戻しのバッファ 3 枚を持ち続け、`submit` は待たず、
  `poll` で終わった最新の 1 枚を受け取る) と `yuv` (`PixelFormat` = RGBA8 / NV12 / YUY2。BT.709 の限定範囲)。NV12・YUY2 は計算シェーダ
  (`yuv.wgsl`) で変換してから読み戻し、計算シェーダの無いアダプタでは CPU で変換する。1 枚の口 (`Gpu::render`) は変えず、共通の部分を関数に分けた。
  `tests/resident_gpu.rs` (Vulkan と GL の llvmpipe で確認)。`ugoe-render` は変えていない。
- **仮想カメラのクレート `ugoe-studio-vcam` (`apps/studio/vcam`):** unsafe を許す唯一のアプリのクレート。lint を Cargo.toml で写して `unsafe_code` だけ許し、
  `undocumented_unsafe_blocks` と `unsafe_op_in_unsafe_fn` を拒否。内部のクレートに依存しない。
  - safe な API: `VirtualCamera::start/send/device`・`availability`・`Error::code` (画面の文言のコード)。
  - Windows (`src/mf/`): `MFCreateVirtualCamera` を実行時に `mfsensorgroup.dll` から引く (Windows 10 でも exe が起動し、「Windows 11 が要る」を出す)。
    カメラは `Session` の寿命・今の利用者だけ。メディアソースの DLL (IMFMediaSource2・IMFMediaStream2・IKsControl・IMFActivate・クラスファクトリ・
    `DllRegisterServer` など) が `Global\` の共有メモリを作り、スタジオが NV12 を書く (`src/shared.rs`、seqlock。safe な Rust でテスト)。
    設定 (解像度・fps) はカメラの機器のプロパティで渡し、読めなければ全部の組を出して最も近い画素で拡大縮小する。CRT は静的にリンク。
  - Linux (`src/v4l2.rs`): v4l2loopback の機器を探し、`VIDIOC_S_FMT` (YUYV・BT.709) と `VIDIOC_S_PARM` の後に `write`。構造体の大きさと ioctl の番号をテスト。
  - 登録: check-layers・ci-groups (apps)・check-licenses (EDITOR_CRATES)・commitlint の scope `studio-vcam`・docs/architecture.md。
- **スタジオ (`virtual_camera.rs`):** 止めている間はスレッドを作らない。指揮役のフレームを同じプロセスのチャネルで受け、ネイティブの受け手で全部適用し、
  フレームレートの回数だけ描いて OS に渡す。T0802 のシーンに合わせ、モデルの読み直し・モジュールの付け直し (`Follower::continue_scene` を足した)・
  配置 (`placement_view`。出力ウィンドウと同じ置き方)・背景 (単色・クロマキー。透過は設定の色) を反映。状態 (止めている・始めている・動いている・止まった)
  と負荷 (`virtualCamera` の行。目標は仮想カメラの fps) を操作ウィンドウに配る。コマンド `virtual_camera_state`・`start_virtual_camera`・
  `stop_virtual_camera`・`set_virtual_camera_background`。`tests/virtual_camera_gpu.rs` は、受け手の画が物理を有効にした 1 つの Puppet の画と一致することを確かめる
  (物理を切った画とは一致しないことも確かめた)。
- **操作ウィンドウ (`VirtualCameraPanel.svelte`):** 開始と停止・解像度・fps・透過を埋める色 (@ugoe/ui の Toggle・Select・SegmentedControl・ColorPicker・Notice)。
  使えない・止まった・遅れた・受け取るアプリがいないの知らせと、v4l2loopback の入れ方・Windows 10 の OBS の手順・入れ直しの案内 (文言は en/ja)。
- **Windows の配布:** `tauri.windows.conf.json` (NSIS だけ・perMachine・`pnpm vcam` で DLL を作る) と `windows/hooks.nsh` (DLL を入れて 64 ビットの regsvr32 で登録・
  アンインストールで解除・使用中の DLL は退けて再起動で消す)。
- **docs:** docs/studio.md の「仮想カメラ」(描き方・Windows・Linux・macOS・利用者が確かめる手順)、apps/studio/README.md、crates/ugoe-headless/README.md、docs/testing.md。

### 確かめたこと

- `just ci`・`just test-gpu` (rebase 後)。`cargo xwin clippy -p ugoe-studio -p ugoe-studio-vcam --target x86_64-pc-windows-msvc -- -D warnings`。
- DLL を cargo-xwin でリンクし、書き出す関数 4 つと、`mfsensorgroup.dll`・VC++ ランタイムを読み込まないことを確かめた。
- Linux からスタジオの NSIS のインストーラをクロスビルドし、DLL が入り regsvr32 が呼ばれることを makensis の出力で確かめた (`__FILEDIR__` の誤りをこれで見つけて直した)。
- 偽のホストとヘッドレスの Chrome で、操作ウィンドウの仮想カメラの欄を英語・日本語・ダーク・狭い幅で撮った (scratchpad/T0807/shots/)。設定が覚えられ、開始で渡ることも確かめた。

- 2 回目の rebase (team-lead の依頼、T0803・T0909 の後): README・locales・package.json・lib.rs・Control.svelte・host.ts の衝突は両方を残して合わせた
  (lib.rs はホットキー・入力の変更と仮想カメラを並べ、刻みでは指揮役を手放す前に仮想カメラへフレームを送る)。pnpm-lock.yaml は変わらず、
  Cargo.lock は `--locked` で整合を確かめた。その後で `just ci`・スタジオのテスト・`just test-gpu`・Windows のクロス clippy を通した。

### 確かめていないこと (利用者の確認に残す)

- Windows の実機: インストール (UAC・登録)、カメラの一覧に「ugoe studio」が出る、各アプリで映る・色が正しい、受け取るアプリの有無の知らせ、終了でカメラが消える、
  Windows 10 の表示、アンインストールと上書き。手順は docs/studio.md の「Windows (Media Foundation)」の未確認の 7 項目。
  特にメディアソースが Frame Server に受け入れられるか (属性・IKsControl・IMFActivate の扱い) は実機でしか分からない。
- Linux の実機: v4l2loopback で映るか (WSL のカーネルに無い)。docs/studio.md の「Linux (v4l2loopback)」の未確認の 4 項目。

### 範囲外の変更

- ルートの `Cargo.toml` (members に `apps/studio/vcam`、[workspace.dependencies] に `ugoe-studio-vcam`・`libc`・`windows`・`windows-core`) と `Cargo.lock`。
  windows 0.62 と libc は lock に既にあった (新しいクレートは増えない)。依存の選定は team-lead に「論点・選択肢・推奨」を送った (返事は未着。推奨の windows + libc で実装)。
  crates: に Cargo.toml を足した。
- `scripts/gpu-test-args.mjs`: CI (環境変数 `CI`) では Tauri のアプリの `gpu-tests` を外す (CI の gpu ジョブは webkit を入れないため)。scripts は範囲内。

### 後続の候補

- 仮想カメラの画にシーンのライティングと背景の画像を反映する (`TODO(T0808)`。出力ウィンドウとの画の比べ合わせと一緒に)。
- 実機で確かめた結果に合わせてメディアソースを直す (Windows)。カメラの名前を設定できるようにする。
- 仮想カメラの設定 (解像度・fps・色) をプロファイルへ移す (いまは localStorage)。ツールバーに仮想カメラの状態の印を出す (docs/studio.md の図)。
- AGENTS.md のリポジトリ構成の `apps/studio/` の行 (「仮想カメラは後続のタスク」) を直す (範囲外なので触っていない)。

