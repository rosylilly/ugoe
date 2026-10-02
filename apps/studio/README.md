# apps/studio

ugoe のスタジオ (Tauri v2 + Svelte 5)。配信者が配信中に使うアプリで、モデルを **出力ウィンドウ** に出し、**操作ウィンドウ** から動かす。
範囲・構成・描画の場所は [ADR 0028](../../docs/adr/0028-studio.md)、画面と配信の形は [docs/studio.md](../../docs/studio.md)。
T0801 で骨組み (2 つのウィンドウ・指揮役と受け手)、T0802 でシーンとプロファイル、T0803 で入力 (カメラのトラッキング・マイクの口パク・
モーションと表情・グローバルなホットキー)、T0804 で配信向けの出力、T0807 で仮想カメラ (Windows・Linux)、T0809 で macOS の仮想カメラ (Camera Extension。配布物に入れるのは後)、
T0805 で外から操作する API (ループバックの WebSocket。[docs/studio-api.md](../../docs/studio-api.md))、T0808 で OBS のブラウザソース用のページ
([docs/studio.md#ブラウザソース](../../docs/studio.md#ブラウザソース)) を作った。
配信者向けの使い方 (マニュアル) は [docs/manual/studio.md](../../docs/manual/studio.md)、見本のプロファイルは `pnpm sample-profile` (T0806)。

**ライセンスは GPL-3.0-only** (エディタ・ビューアと同じ)。アプリ同士で import はしない (共有するものは `packages/` に置く)。

```
index.html / src/control.ts   操作ウィンドウの入口 (@ugoe/ui のトークン・言語の設定)
src/Control.svelte            操作ウィンドウ: プロファイルのメニュー・モデルを選ぶ・出力ウィンドウの表示/最前面/大きさ・いまの状態・パラメータ
src/SettingsDialog.svelte     設定 (ツールバーの歯車): 言語・テーマ・ライセンスの表示 (T0907)
src/LoadPanel.svelte          操作ウィンドウの「フレームレートと負荷」: 刻みの速さ (30 / 60 / 120)・1 刻みの時間・出力ごとの描画の負荷
src/ScenesPanel.svelte        シーンの一覧 (切り替え・追加・複製・並べ替え・削除)
src/SceneEditor.svelte        シーンの編集 (名前・モデル・モジュール・位置と大きさと回転・背景・ライティング)
src/LightingEditor.svelte     シーンの光源 (組・環境光・平行光源・点光源)
src/MotionPanel.svelte        表情とモーション (表情の付け外し・モーションの再生と優先度・待機モーション・トラッキング中のモーションの優先)
src/InputPanel.svelte         入力 (@ugoe/face-tracking のパネルでカメラのトラッキング、マイクの機器・感度・ノイズの閾値・メーター)
src/HotkeysPanel.svelte       ホットキーの割り当て (キーを押して決める・操作と引数・登録できない理由)
src/VirtualCameraPanel.svelte 操作ウィンドウの「仮想カメラ」: 開始と停止・解像度・フレームレート・透過を埋める色・止まった/遅れたの知らせと案内
src/LicenseDialog.svelte      ライセンスの表示 (設定の「ライセンスを表示…」)。本文は public/third-party-notices.txt
public/third-party-notices.txt 同梱する第三者のもの (MediaPipe の wasm とモデル・JS のライブラリ) のライセンスの表記
src/ApiPanel.svelte           操作 API (有効にする・接続先・ポート・トークンの写しと作り直し・承認したアプリの切断と取り消し・承認を待つアプリ)
src/ApiApproval.svelte        操作 API に初めて接続したアプリの承認の知らせ (権限の選択・承認・拒否・後で)
src/SourcePanel.svelte        ブラウザソース (OBS): 出す・やめる・URL のコピー (トークンは伏せて出す)・作り直し・OBS での設定の手順
source.html / src/source.ts   ブラウザソースのページの入口 (スタジオの待ち受けが `/source/<トークン>/` で出す。受け手は Output.svelte)
output.html / src/output.ts   出力ウィンドウの入口 (背景は透過。@ugoe/ui と @ugoe/i18n は読まない)
src/Output.svelte             出力ウィンドウとブラウザソースのページの受け手: @ugoe/player の受け手のモードでモデルだけを描く。シーンの見せ方を当てる。
                              配信の口 (`feed`) を受け取る。出力ウィンドウではドラッグで動かす
src/lib/feed.ts               受け手の配信の口 (OutputFeed)。ブラウザソースは WebSocket とファイルの HTTP (`sourceFeed`。切れたらつなぎ直す)
src/lib/host.ts               バックエンドとの口 (StudioHost)。Tauri の外では window.__UGOE_STUDIO_HOST__ を使う
src/lib/settings.ts           覚える設定 (言語・刻みの速さ。localStorage) と出力の大きさのプリセット
src/lib/load.ts               負荷の注意の判断 (刻みが遅れた・揺れた・描画が重い・フレームを捨てた・描いていない)
src/lib/profile.ts            プロファイルとシーンの形 (profile.rs と同じ)・エラーの形・背景や配置の既定値
src/lib/inputs.ts             入力・モーション・ホットキーの設定と演技の様子の形 (input.rs・hotkeys.rs・animation.rs と同じ)
src/lib/api.ts                操作 API とブラウザソースの様子の形 (mod.rs の ApiView・SourceView と同じ)・接続先・ポートの範囲・URL のトークンを伏せる
src/lib/hotkeys.ts            ホットキーのキーの書き方 (押したキー → `Ctrl+Shift+KeyA`、Kbd の表記)
src/lib/lipSync.ts            マイクの音から口を開く量を作る式 (dB・ノイズの閾値・感度・開く速さと閉じる速さ)
src/lib/microphone.svelte.ts  マイク (getUserMedia → AnalyserNode)。刻みごとに口を開く量を指揮役へ送る
src/lib/pulse.ts              入力の処理を回す刻み (バックエンドの `subscribe_pulse`。requestAnimationFrame に頼らない。重ければ間引く)
src/lib/channel.ts            Tauri の Channel の受け取りを、例外を外へ出さない形に包む (投げると以後のメッセージが止まるため)
src/lib/latest.ts             送っている間に届いた値は最後のものだけを次に送る (トラッキングと口パクの IPC)
src/lib/outputLayout.ts       受け手がシーンの通知ですること (planScene)・配置を舞台とビューと CSS の回転に分ける計算
src/lib/errors.ts             バックエンドのエラー (code) を文にする
src/lib/labels.ts             シーンの表示名
src/lib/virtualCamera.ts      仮想カメラの設定 (localStorage) と状態・知らせの判断
src/lib/i18n.ts               文言 (@ugoe/i18n の名前空間 `studio`。カタログは locales/)
src/lib/wasm/                 ugoe-wasm の読み込み。generated/ は `pnpm wasm` の出力 (gitignore 済み)
src-tauri/                    バックエンド (Rust クレート `ugoe-studio`)
  src/conductor.rs              指揮役: 物理を切ったネイティブの Puppet。刻みごとにフレームを作る。シーンを替える (present)
  src/animation.rs              指揮役の演技: モーション・表情・待機モーション・トラッキング中のモーションの優先・トラッキングと口パクの入力
  src/input.rs                  プロファイルの入力とモーションの設定・操作ウィンドウから届くトラッキングの 1 フレーム
  src/hotkeys.rs                ホットキーの割り当ての形と、登録してよいかの判断 (キーを読めるか・ぶつかり)
  src/pacing.rs                 刻みの速さ (30 / 60 / 120)・次の刻みの時刻・実際の速さと揺れの測定
  src/profile.rs                プロファイルの形式 (版の確かめ・値を収める・書き出し)・最近のプロファイル
  src/stage.rs                  舞台: 開いているプロファイルとシーンの一覧の操作・シーンの切り替え (ファイルを読んで指揮役へ)・受け手に出すファイル
  src/stream.rs                 配信の形 (フレームのバイト列・シーンの JSON)
  src/follower.rs               ネイティブの受け手 (一致の検査と仮想カメラが使う)
  src/lighting.rs               シーンの光源 (LightingSettings の JSON) をレンダラの光源にする (@ugoe/player と同じ規則。仮想カメラが使う)
  src/virtual_camera.rs         仮想カメラの受け手 (Feed: 受け手 + ugoe-headless の常駐の描画) とスレッド (T0807)
  src/lib.rs                    刻みのスレッド・Tauri のコマンド・出力ウィンドウの操作
  src/api/                      操作 API (T0805。docs/studio.md#外部-api、英語のリファレンスは docs/studio-api.md)
    mod.rs                        接続ごとの状態 (名乗る前・承認待ち・使える)・承認と取り消し・通知
    methods.rs                    メソッド (指揮役と舞台に入れる。ホットキーと同じ口)
    server.rs                     ループバックの待ち受け・経路 (`/api` と `/source/<トークン>/…`)・接続ごとのスレッド (読むのを区切って通知を送る)
    source.rs                     ブラウザソース (T0808): ページ・ファイル・フレームの WebSocket・スイッチとトークン・描画の負荷の受け取り
    source_check.rs               ブラウザソースの画の確かめの待ち受け (手で実行する ignored のテスト。下の「ブラウザソースの画を確かめる」)
    http.rs                       要求の頭の読み取りと検査 (Host・転送ヘッダ・Origin)
    ws.rs                         WebSocket の握手の検査と接続の設定 (フレームは tungstenite)
    rpc.rs                        JSON-RPC 2.0 の読み書きと誤りの符号
    store.rs                      studio-api.json (有効・ポート・トークン・承認したクライアントの鍵・ブラウザソースのスイッチとトークン。アプリのデータの場所)
    commands.rs                   操作ウィンドウのパネルが呼ぶコマンド
    tests.rs                      接続と操作のテスト (承認・権限・操作・通知・本物の待ち受け・JSON Schema との一致・ブラウザソースの経路と配信)
  tests/follow_match.rs         一致の検査 (ADR 0028 の 10.)
  tests/virtual_camera_gpu.rs   仮想カメラの画が、物理を有効にした 1 つの Puppet の画と一致する (`just test-gpu`)
  tauri.windows.conf.json       Windows のビルド: 仮想カメラの DLL を作る・NSIS を管理者の権限で入れる
  windows/hooks.nsh             NSIS: 仮想カメラの DLL を入れて regsvr32 で登録する・消す
  tauri.macos-vcam.conf.json    macOS で Camera Extension を入れたスタジオを作る設定 (`just app::build TARGET=studio PLATFORM=macos VCAM=1` が `--config` で重ねる。docs/release.md#macos-の仮想カメラ)
  Entitlements.vcam.plist       その entitlement (Entitlements.plist に System Extension を入れる権限を足したもの)
scripts/build-vcam.mjs        `pnpm vcam`: 仮想カメラのメディアソースの DLL を作る (Windows 向け。CRT は静的)
scripts/build-vcam-macos.mjs  `pnpm vcam:macos`: macOS の Camera Extension を swiftc で作って署名する (macOS だけ)
scripts/sample-profile.mjs    `pnpm sample-profile <出力>`: 見本のモデル (assets/samples) を使う見本のプロファイルを書き出す (T0806。docs/manual/studio.md#見本のプロファイル)
vcam/                         仮想カメラの OS の境界 (クレート `ugoe-studio-vcam`。unsafe を許す唯一のアプリのクレート。下の「仮想カメラ」)
```

## 仕組み

- **指揮役 (バックエンド):** 刻みのスレッドが設定の速さ (30 / 60 / 120 回/秒。既定 60) で `dt` を測り (0.1 秒で頭打ち)、物理を切った `Puppet` を `update(dt)` して、
  `{seq, revision, dt, values, lip_sync}` のフレーム (`values` は物理の段に入る直前の値。T0134) を出力ウィンドウへ Tauri の `Channel` で配る。WebView の外で回すので、ウィンドウの抑制を受けない。
  操作ウィンドウへは、シーン (パラメータの定義) と、毎秒 10 回に間引いた値・1 刻みの時間・実際の刻みの速さ・受け手の数と描画の負荷を別の `Channel` で配る。
- **隠れても止まらない:** 両方のウィンドウで WebView のバックグラウンドの抑制を切る (`tauri.conf.json` の `additionalBrowserArgs` と
  `backgroundThrottling`)。出力ウィンドウは届いたフレームを届いたときに適用し、描画だけを `requestAnimationFrame` で行う。
  OS ごとの効き方と、OBS での取り込みの手順は [docs/studio.md](../../docs/studio.md#隠れても止まらない)。
- **受け手 (出力ウィンドウ):** `@ugoe/player` の受け手のモード (`Player.startFollowing`)。シーンの通知を受けたら `scene_model(revision)` で
  モデルのバイト列を取って読み、そのシーンのフレームを順にすべて `follow` で適用して (物理・リップシンク・ポーズだけが動く)、最後に 1 回描く。
- **操作ウィンドウはモデルを描かない** (GPU の負荷を出力の分だけにする)。様子は値と数値で示す。
- パラメータのスライダーは指揮役に書き込む (`set_parameter`。次の刻みから基準値になる)。動かしてから 0.5 秒は、指揮役からの値で表示を上書きしない。
- 言語は「OS に合わせる」が既定で、OS の言語は `tauri-plugin-os` の `locale()` で取る (WebView の `navigator.languages` は OS と違うことがある。
  `packages/i18n/README.md`)。選んだ言語は `ugoe-studio.locale` に覚える。
- 言語とテーマ (OS に合わせる・ライト・ダーク。`@ugoe/ui` の `ThemeSetting`) はツールバーの歯車の設定のダイアログ。テーマは `ugoe-studio.theme` に覚える。
  操作ウィンドウだけに効かせ、出力ウィンドウ (`output.ts`) は @ugoe/ui を読まないので透過のまま。
- 出力ウィンドウの大きさは物理ピクセルのプリセット (1920×1080・1280×720・1080×1920・1080×1080・自由)。選んだものはプロファイルに持ち、
  プロファイルを開いたときに合わせる (T0802。T0801 の `ugoe-studio.output-size` は使わない)。
- **シーンとプロファイル (T0802):** プロファイル (版の番号付きの JSON) は舞台 (`stage.rs`) が持ち、変わるたびに操作ウィンドウへ `profile` の
  通知を配る。シーンの切り替えは指揮役の `present` で行い、受け手には `scene` の通知 (モデルとモジュールの版・見せ方) を配る。
  形式と切り替えの規則は [docs/studio.md](../../docs/studio.md#シーンとプロファイル)。

### Tauri のコマンド

| コマンド | ウィンドウ | 内容 |
| --- | --- | --- |
| `restore_profile()` | 操作 | 起動して最初の 1 回だけ、最後に使ったプロファイルを開く |
| `new_profile()` / `open_profile(path)` / `save_profile(path?)` / `forget_recent_profile(path)` | 操作 | プロファイル (`save_profile` は省くと今の保存先) |
| `add_scene(name)` / `duplicate_scene(id, name)` / `move_scene(id, index)` / `remove_scene(id)` | 操作 | シーンの一覧 |
| `update_scene(scene)` / `activate_scene(id)` | 操作 | シーンの設定を置き換える (配信中なら出し直す) / 切り替える (ファイルの拡張子と大きさを確かめる) |
| `set_output_preset(preset)` | 操作 | 出力ウィンドウの大きさのプリセットをプロファイルに持つ |
| `subscribe_status(channel)` | 操作 | プロファイル・シーン・値・負荷・出力ウィンドウの状態を受け取る |
| `set_parameter(id, value)` / `reset_parameters()` | 操作 | 指揮役のパラメータを書き換える |
| `output_state()` / `set_output_visible(visible)` / `set_output_on_top(onTop)` / `set_output_size(width, height)` | 操作 | 出力ウィンドウ |
| `tick_rate()` / `set_tick_rate(rate)` | 操作 | 刻みの速さ (30 / 60 / 120。ほかの値はエラー) |
| `subscribe_pulse(channel)` | 操作 | 刻みごとに入力の処理を促すメッセージを受け取る (カメラとマイクの処理。最小化しても止まらない) |
| `set_tracking(frame)` / `set_tracking_paused(paused)` / `set_auto_eye_blink(enabled)` | 操作 | トラッキングの入力 (`null` で外す)・一時停止・自動の目パチ |
| `set_lip_sync_level(level)` / `set_lip_sync_paused(paused)` | 操作 | マイクの口を開く量 (0〜1、`null` で止める)・一時停止 |
| `play_motion(name, priority)` / `stop_motions()` | 操作 | モーション (優先度が低くて始まらなければ `false`) |
| `set_expression(name)` / `add_expression(name)` / `remove_expression(name)` / `clear_expressions()` | 操作 | 表情 |
| `set_motion_settings(settings)` / `set_input_settings(settings)` / `set_hotkeys(hotkeys)` | 操作 | プロファイルの `motion`・`input`・`hotkeys` (ホットキーは登録し直す) |
| `suspend_hotkeys(suspended)` | 操作 | 割り当ての画面でキーを押している間、ホットキーの登録を外す |
| `module_slot(path)` | 操作 | モジュールが付くスロットの名前 (ホットキーの着せ替え) |
| `api_state()` / `set_api_enabled(enabled)` / `set_api_port(port)` | 操作 | 操作 API の様子・待ち受けの切り替え (覚える)・ポート (1024〜65535) |
| `api_token()` / `regenerate_api_token()` | 操作 | 操作 API のトークンを写す・作り直す |
| `approve_api_client(id, permission)` / `deny_api_client(id)` / `revoke_api_client(id)` / `disconnect_api_client(id)` | 操作 | 承認を待つアプリの承認・拒否、承認したアプリの取り消し・切断 |
| `set_source_enabled(enabled)` / `source_url()` / `regenerate_source_token()` | 操作 | ブラウザソースを出す・やめる (覚える)・OBS に入れる URL (トークンを含む)・トークンの作り直し (新しい URL を返す)。様子は `api` の通知の `source` |
| `subscribe_frames(channel)` | 出力 | シーンの通知 (JSON) とフレーム (バイト列) を受け取る。開き直したページは前の Channel を置き換える |
| `scene_model(revision)` / `scene_module(revision, index)` / `scene_background(revision)` | 出力 | 今のシーンのモデル・モジュール・背景の画像のバイト列 (今のシーンでなければエラー) |
| `report_render_stats(stats)` | 出力 | 描画の負荷 (回/秒・1 回の時間の平均と最大・捨てたフレーム) を約 1 秒ごとに送る。操作ウィンドウに出す |
| `virtual_camera_state()` | 操作 | 仮想カメラの状態 (止めている・始めている・動いている・止まった。始められない理由も) |
| `start_virtual_camera(settings)` / `stop_virtual_camera()` | 操作 | 仮想カメラを始める (動いていれば始め直す)・止める。状態の変化は `subscribe_status` の `virtualCamera` で届く |
| `set_virtual_camera_background(background)` | 操作 | 透過を埋める色 (RGB) を変える |

- プロファイルとシーンのコマンドのエラーは `{ code, ... }` の形 (`profile.rs` の `ProfileError`・`stage.rs` の `SceneError`)。画面は `code` で文言を選ぶ。

- 出力ウィンドウは閉じる操作で隠すだけにする (受け手を作り直さない)。操作ウィンドウを閉じるとアプリを終える (仮想カメラも止める)。
- 権限 (`src-tauri/capabilities/`): 操作ウィンドウはファイルを開く・保存先を選ぶダイアログと OS の言語、出力ウィンドウはウィンドウのドラッグだけ。
  カメラとマイクは、バックエンドの `on_permission_request` が操作ウィンドウの要求だけを許す (macOS は `Info.plist` と `Entitlements.plist`)。
  ホットキーはバックエンドが `tauri-plugin-global-shortcut` で登録する (JS の口は使わないので、プラグインの権限は付けない)。
- 操作ウィンドウへの通知には、ほかに `animation` (モーション・表情・トラッキングの様子)・`hotkeys` (登録できなかった割り当て)・
  `hotkey` (押されたホットキー。キャリブレーションと、行えなかったときの理由。操作 API の `tracking.calibrate` も同じ通知を使う)・
  `api` (操作 API の様子。待ち受け・承認したアプリ・承認を待つアプリ) がある。
- 出力ウィンドウの背景の透過は、macOS では `macOSPrivateApi` (tauri の `macos-private-api` feature) が要る (Mac App Store では配布できなくなる)。

## 仮想カメラ

[docs/studio.md#仮想カメラ](../../docs/studio.md#仮想カメラ) と [ADR 0028 の 8.](../../docs/adr/0028-studio.md#8-仮想カメラの-os-の部品と-unsafe-の例外)。

- バックエンドの仮想カメラのスレッド (`src-tauri/src/virtual_camera.rs`) が、指揮役のフレームをネイティブの受け手で適用し、`ugoe-headless` の
  常駐の描画 (`Resident`) で描いて NV12 (Windows・macOS) / YUY2 (Linux) で読み戻し、`ugoe-studio-vcam` に渡す。止めている間はスレッドを作らない。
- `vcam/` (`ugoe-studio-vcam`、GPL-3.0-only) は OS の境界だけを持つ。内部のクレートに依存しない。
  - `src/lib.rs`: スタジオが使う safe な API (`VirtualCamera::start`・`send`・`availability`・`Error::code`)。
  - `src/shared.rs`: スタジオと Windows のメディアソースの間の共有メモリの形 (safe な Rust でテストする)。
  - `src/v4l2.rs`: Linux。v4l2loopback の機器を探し、`VIDIOC_S_FMT` してから `write` する。
  - `src/mf/`: Windows。`mod.rs` はスタジオ側 (`MFCreateVirtualCamera` を実行時に引く・共有メモリに書く)、`source.rs` は
    Frame Server が読み込むメディアソースの DLL (`DllGetClassObject`・`DllRegisterServer` など)。
  - `src/cmio.rs`: macOS。Camera Extension を有効にし (`OSSystemExtensionRequest`)、拡張の受ける側のストリームに `CMSampleBuffer` を積む
    (objc2 の束縛)。
  - `macos/`: macOS の Camera Extension (Swift。`Extension/Camera.swift` が機器と 2 つのストリーム、`Info.plist`・`Extension.entitlements`)。
    Cargo のビルドには入らない (`pnpm vcam:macos` が作る)。
  - **`unsafe` を許す** (ADR 0028 の 8.)。ワークスペースの lint を `Cargo.toml` で写して `unsafe_code` だけを許し、
    `clippy::undocumented_unsafe_blocks` (どの `unsafe` にも `// SAFETY:`) と `unsafe_op_in_unsafe_fn` を拒否にしている。
- Windows のコードは Linux からも `cargo clippy -p ugoe-studio-vcam --target x86_64-pc-windows-msvc` で検査できる (リンクまでは `cargo xwin build`)。
  macOS の Rust のコードも `--target aarch64-apple-darwin` で検査できる (Swift の拡張は macOS でしか作れない)。

## ブラウザソースの画を確かめる

出力ウィンドウ・ブラウザソースのページ・仮想カメラの画を、同じフレームの列で止めて比べる ([docs/studio.md#画の確かめ](../../docs/studio.md#画の確かめ-adr-0028-の-10))。

```sh
pnpm --filter @ugoe/studio wasm && pnpm --filter @ugoe/studio exec vite build   # dist/ を作る
UGOE_STUDIO_CHECK_DIR=/tmp/source-check cargo test -p ugoe-studio --features gpu-tests --lib \
  serves_a_frozen_scene_for_a_browser_check -- --ignored --nocapture
```

待ち受けは `/tmp/source-check/url.txt` に URL を書き、ページが 2 つつなぐのを待つ。ヘッドレスの Chrome で、その URL (ブラウザソース) と、
同じ Origin の別のパスで返した `dist/output.html` (`window.__UGOE_STUDIO_HOST__` に、同じ `frames` の WebSocket と `scene/…` を読む口を置く) を
480×270 で開く。フレームを配り終えると `done.txt` と仮想カメラの画 `vcam.png` を書くので、その後に 2 つのページを撮って `vcam.png` と比べる。
`UGOE_STUDIO_CHECK_PORT` (既定 28461)・`UGOE_STUDIO_CHECK_SECS` (止めた後に待ち受ける秒数、既定 60)・`UGOE_STUDIO_CHECK_ROTATION` (回転、既定 8)・
`UGOE_STUDIO_CHECK_NO_LIGHT` (光源を切る) で条件を変えられる。ループバックの外の Origin のページ (Playwright の `route` で返したものなど) から
ループバックへ接続するときは、Chrome の Local Network Access の検査で止められることがある (`--disable-features=LocalNetworkAccessChecks`)。
OBS のページはスタジオ自身が出すので、この検査には当たらない。

## ヘッドレスのブラウザで確かめる

Tauri の外では、ページを読む前に `window.__UGOE_STUDIO_HOST__` に `StudioHost` (`src/lib/host.ts`) を置く (Playwright の `addInitScript` など)。
置かなければ「Tauri の中で動かしてください」というエラーを出す。操作ウィンドウのトラッカーは `window.__UGOE_TRACKER__`
(`startReplay(text)` で記録を流せる)。Chrome の偽のカメラ・マイク (`--use-fake-device-for-media-stream --use-file-for-fake-video-capture=<.y4m>
--use-file-for-fake-audio-capture=<.wav>`) で入力の経路を確かめられる。出力ウィンドウの `Player` は `window.__UGOE_STUDIO_OUTPUT__` で見られる
(`status`・`following`・`error`)。WebGPU の無いヘッドレスの Chrome でも、SwiftShader の WebGL2 で描ける
(`--use-angle=swiftshader --enable-unsafe-swiftshader`)。

## コマンド

```sh
just studio                       # 開発モードで起動 (vite の 1440 番ポート + tauri dev)
just studio-build                 # リリースビルド (`just app::build TARGET=studio` の別名)。例: just studio-build --bundles deb
just app::build TARGET=studio PLATFORM=windows  # Linux (WSL2) から Windows 向け (仮想カメラの DLL も cargo-xwin で作って入れる)
just app::build TARGET=studio PLATFORM=macos VCAM=1  # macOS で Camera Extension を入れる (署名が要る。docs/release.md#macos-の仮想カメラ)
pnpm --filter @ugoe/studio wasm   # ugoe-wasm をビルドして src/lib/wasm/generated/ に JS バインディングを出す
pnpm --filter @ugoe/studio mediapipe  # MediaPipe の wasm とモデルを public/mediapipe/ に用意する (dev・build でも実行される。gitignore 済み)
pnpm --filter @ugoe/studio vcam   # 仮想カメラの DLL (Windows 向け。Linux からは cargo-xwin でクロスビルド)
pnpm --filter @ugoe/studio sample-profile ~/sample-profile.json  # 見本のプロファイル (--lang en でシーンの名前を英語に。--force で上書き)
cargo test -p ugoe-studio         # 指揮役・配信の形・一致の検査・操作 API (本物のループバックの待ち受けも使う)
cargo test -p ugoe-studio-vcam    # 仮想カメラの共有メモリの形・設定・v4l2 の構造体
cargo test -p ugoe-studio --features gpu-tests --test virtual_camera_gpu  # 仮想カメラの画 (just test-gpu に含まれる。CI では除く)
```

必要な道具 (Rust・Node.js・pnpm・just・wasm-bindgen-cli・各 OS の前提パッケージ) はエディタと同じ
([apps/editor/README.md#各-os-でのビルド](../editor/README.md#各-os-でのビルド))。
