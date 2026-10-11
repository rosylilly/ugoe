# apps/studio

ugoe のスタジオ (Tauri v2 + Svelte 5)。配信者が配信中に使うアプリで、モデルを **出力ウィンドウ** に出し、**操作ウィンドウ** から動かす。
範囲・構成・描画の場所は [ADR 0028](../../docs/adr/0028-studio.md)、画面と配信の形は [docs/studio.md](../../docs/studio.md)。
T0801 で骨組み (2 つのウィンドウ・指揮役と受け手)、T0802 でシーンとプロファイル、T0803 で入力 (カメラのトラッキング・マイクの口パク・
モーションと表情・グローバルなホットキー)、T0804 で配信向けの出力、T0807 で仮想カメラ (Windows・Linux)、T0809 で macOS の仮想カメラ (Camera Extension。配布物に入れるのは後)、
T0805 で外から操作する API (ループバックの WebSocket。[docs/studio-api.md](../../docs/studio-api.md))、T0808 で OBS のブラウザソース用のページ
([docs/studio.md#ブラウザソース](../../docs/studio.md#ブラウザソース)) を作った。
配信者向けの使い方 (マニュアル) は [docs/manual/studio.md](../../docs/manual/studio.md)、見本のプロファイルは `assets/samples/ugoe-sample-profile.json` (T0806・T0865)。

**ライセンスは GPL-3.0-only** (エディタ・ビューアと同じ)。アプリ同士で import はしない (共有するものは `packages/` に置く)。

```
index.html / src/control.ts   操作ウィンドウの入口 (@ugoe/ui のトークン・言語の設定)
src/Control.svelte            操作ウィンドウ: プロファイルのメニュー・モデルを選ぶ・いまの状態・パラメータ。
                              列の並び (シーン・配信中に触るもの・設定) と狭い幅の積み方 (T0817)、設定の並び (準備の順。T0818)
src/OutputPanel.svelte        操作ウィンドウの「出力ウィンドウ」: 表示・最前面・大きさと、様子 (いまの大きさ・枠・背景)・取り込み方の案内 (T0818)
                              透過のシーンなのに透過が画面で効かないときの「背景」の欄の注意と、背景をクロマキーにするボタン (T0846・T0849)
src/StatusBadge.svelte        「いまの状態」の印 1 つ (形・短い文・Tooltip。押すとその設定のパネルへ。中身は lib/status.ts。T0817・T0845)
src/SettingsDialog.svelte     設定 (ツールバーの歯車): 言語・テーマ・ライセンスの表示 (T0907)
src/LoadPanel.svelte          操作ウィンドウの「フレームレートと負荷」: 刻みの速さ (30 / 60 / 120)・使う GPU (T0860)・GPU の優先度 (Windows だけ。T0861)・1 刻みの時間・出力ごとの描画の負荷と描いている API と GPU
src/ScenesPanel.svelte        シーンの一覧 (切り替え・追加・複製・並べ替え・削除 (確かめてから)・切り替えのホットキーの表示)
src/SceneEditor.svelte        シーンの編集 (名前・モデル・モジュール・位置と大きさと回転・背景・ライティング)
src/LightingEditor.svelte     シーンの光源 (組・環境光・平行光源・点光源)
src/MotionPanel.svelte        表情とモーション (表情の付け外し・モーションの再生と優先度)
src/MotionSettingsPanel.svelte モーションの設定 (待機モーション・トラッキング中のモーションの優先。右の設定の列。T0818)
src/InputPanel.svelte         入力 (@ugoe/face-tracking のパネルでカメラのトラッキング、マイクの機器・感度・ノイズの閾値・メーター)
src/HotkeysPanel.svelte       ホットキーの割り当て (キーを押して決める・操作と引数・登録できない数と行ごとの印と理由・削除の「元に戻す」)
src/ReceiverFailuresNotices.svelte 操作ウィンドウの上の、受け手が読めなかったことの知らせ (受け手ごとに 1 つ。読めなかったものをすべて並べる。T0840)
src/VirtualCameraPanel.svelte 操作ウィンドウの「仮想カメラ」: 開始と停止・解像度・フレームレート・透過を埋める色・止まった/遅れたの知らせと案内
src/LicenseDialog.svelte      ライセンスの表示 (設定の「ライセンスを表示…」)。本文は public/third-party-notices.txt
public/third-party-notices.txt 同梱する第三者のもの (MediaPipe の wasm とモデル・JS のライブラリ・Rust のクレート) のライセンスの表記 (`just notices` が生成する)
src/ApiPanel.svelte           操作 API (有効にする・接続先・ポート・トークンの写しと作り直し・承認したアプリの切断と取り消し・承認を待つアプリ)
src/ApiApproval.svelte        操作 API に初めて接続したアプリの承認の知らせ (権限の選択・承認・拒否・後で)
src/SourcePanel.svelte        ブラウザソース (OBS): 出す・やめる・URL のコピー (トークンは伏せて出す)・作り直し・OBS での設定の手順
source.html / src/source.ts   ブラウザソースのページの入口 (スタジオの待ち受けが `/source/<トークン>/` で出す。受け手は Output.svelte)
output.html / src/output.ts   出力ウィンドウの入口 (背景は透過。@ugoe/ui と @ugoe/i18n は読まない)
src/Output.svelte             出力ウィンドウとブラウザソースのページの受け手: @ugoe/player の受け手のモードでモデルだけを描く。シーンの見せ方を当てる。
                              配信の口 (`feed`) を受け取る。出力ウィンドウではドラッグで動かす
src/lib/feed.ts               受け手の配信の口 (OutputFeed)。ブラウザソースは WebSocket とファイルの HTTP (`sourceFeed`。切れたらつなぎ直す)
src/lib/host.ts               バックエンドとの口 (StudioHost)。Tauri の外では window.__UGOE_STUDIO_HOST__ を使う
src/lib/settings.ts           出力の大きさのプリセットと刻みの速さ (プロファイルの値の読み方)
src/lib/storageKeys.ts        localStorage のキーの一覧 (`ugoe.studio.*`。出力ウィンドウの Player の接頭辞 `ugoe.studio.player` も。
                              docs/studio.md の「覚える値」)
src/lib/status.ts             「いまの状態」の印 (トラッキング・口パク・ホットキー・負荷・操作 API・ブラウザソース) の形と文と説明 (T0817)
                              透過が効かないかの判定 (`transparencyLost`。T0846)
src/lib/load.ts               負荷の注意の判断 (刻みが遅れた・揺れた・描画が重い・フレームを捨てた・描いていない)・受け手の API と GPU の名前
src/lib/profile.ts            プロファイルとシーンの形 (profile.rs と同じ)・背景や配置の既定値
src/lib/autosave.ts           自動の保存と、保存先を選ばせるかの決まり (書き換えないプロファイルは自動で保存しない。T0866)
src/lib/inputs.ts             入力・モーション・ホットキーの設定と演技の様子の形 (input.rs・hotkeys.rs・animation.rs と同じ)
src/lib/api.ts                操作 API とブラウザソースの様子の形 (mod.rs の ApiView・SourceView と同じ)・接続先・ポートの範囲・URL のトークンを伏せる
src/lib/hotkeys.ts            ホットキーのキーの書き方 (押したキー → `Ctrl+Shift+KeyA`、Kbd の表記)
src/lib/lipSync.ts            マイクの音から口を開く量を作る式 (dB・ノイズの閾値・感度・開く速さと閉じる速さ)
src/lib/microphone.svelte.ts  マイク (getUserMedia → AnalyserNode)。刻みごとに口を開く量を指揮役へ送る
src/lib/trackerSettings.svelte.ts トラッキングのパネルで変えた調整と使い始めたカメラを、プロファイルの入力の設定へ書き戻す
src/lib/pulse.ts              入力の処理を回す刻み (バックエンドの `subscribe_pulse`。requestAnimationFrame に頼らない。重ければ間引く)
src/lib/channel.ts            Tauri の Channel の受け取りを、例外を外へ出さない形に包む (投げると以後のメッセージが止まるため)
src/lib/latest.ts             送っている間に届いた値は最後のものだけを次に送る (トラッキングと口パクの IPC)
src/lib/outputLayout.ts       受け手がシーンの通知ですること (planScene)・配置を舞台とビューと CSS の回転に分ける計算
src/lib/errors.ts             バックエンドのコマンドのエラー (コードと引数) を今の言語の文にする (下の「コマンドのエラー」)
src/lib/commandMessage.ts     コマンドのエラーの形と英語の文 (@ugoe/i18n を読まない出力ウィンドウとブラウザソースも使う)
src/lib/labels.ts             シーンの表示名
src/lib/receivers.ts          受け手の表示名と、受け手が読めなかったこと (T0825) の文と受け手ごとのまとめ (T0840)
src/lib/decodeImage.ts        受け手が受け取った背景の画像をデコードできるか (壊れた画像を知らせる。T0836)
src/lib/virtualCamera.ts      仮想カメラの設定 (プロファイルの値) と状態・知らせの判断
src/lib/i18n.ts               文言 (@ugoe/i18n の名前空間 `studio`。カタログは locales/)
src/lib/wasm/                 ugoe-wasm の読み込み。generated/ は `pnpm wasm` の出力 (gitignore 済み)
src-tauri/                    バックエンド (Rust クレート `ugoe-studio`)
  src/conductor.rs              指揮役: 物理を切ったネイティブの Puppet。刻みごとにフレームを作る。パペットを組み立て (assemble。舞台のロックの外)、シーンを替える (present)
  src/animation.rs              指揮役の演技: モーション・表情・待機モーション・トラッキング中のモーションの優先・トラッキングと口パクの入力
  src/input.rs                  プロファイルの入力とモーションの設定・操作ウィンドウから届くトラッキングの 1 フレーム
  src/hotkeys.rs                ホットキーの割り当ての形と、登録してよいかの判断 (キーを読めるか・ぶつかり)
  src/pacing.rs                 刻みの速さ (30 / 60 / 120)・次の刻みの時刻・実際の速さと揺れの測定
  src/profile.rs                プロファイルの形式 (版の確かめ・値を収める・書き出し・相対パスを解く・戻す)・最近のプロファイル
  src/stage.rs                  舞台: 開いているプロファイルとシーンの一覧の操作・シーンの切り替え (ファイルを読んで指揮役へ)・受け手に出すファイル
  src/stream.rs                 配信の形 (フレームのバイト列・シーンの JSON)
  src/follower.rs               ネイティブの受け手 (一致の検査と仮想カメラが使う)
  src/lighting.rs               シーンの光源 (LightingSettings の JSON) をレンダラの光源にする (@ugoe/player と同じ規則。仮想カメラが使う)
  src/virtual_camera.rs         仮想カメラの受け手 (Feed: 受け手 + ugoe-headless の常駐の描画) とスレッド (T0807)
  src/lib.rs                    刻みのスレッド・Tauri のコマンド・出力ウィンドウの操作と、透過が画面で効くか (Linux は GDK に聞く。T0846)・2 つのウィンドウを作る
  src/gpu.rs                    使う GPU の設定と GPU の優先度 (studio-settings.json。機械ごと)・WebView2 の引数・Windows の「グラフィックの設定」を開く (T0860・T0861)
  src/gpu_priority.rs           GPU の優先度を当てる先 (スタジオと WebView2 の GPU のプロセス)・当てたかの記録・当てられなかった理由のコード (T0861)
  src/error.rs                  コマンドのエラー (`{ code, args, message }`。下の「コマンドのエラー」。T0835)
  src/load_failure.rs           受け手 (出力ウィンドウ・ブラウザソース・仮想カメラ) が今のシーンのファイルを読めなかったことの報告 (今のシーンのものだけを受け、受け手ごとにすべて覚えて操作ウィンドウに配る。T0825・T0836・T0840)
  src/acl.rs                    コマンドの名前と出力ウィンドウが呼べるもの (build.rs が読む。capabilities/ との一致のテスト。T0820)
  src/attention.rs              承認を待つクライアントを OS の側で知らせるか (操作ウィンドウが前面に無いときに注意を引き、待ちが無くなるか前に出たら止める。T0856)
  src/api/                      操作 API (T0805。docs/studio.md#外部-api、英語のリファレンスは docs/studio-api.md)
    mod.rs                        接続ごとの状態 (名乗る前・承認待ち・使える)・承認と取り消し・通知
    methods.rs                    メソッド (指揮役と舞台に入れる。ホットキーと同じ口)
    server.rs                     ループバックの待ち受け・経路 (`/api` と `/source/<トークン>/…`)・接続ごとのスレッド (読むのを区切って通知を送る)
    source.rs                     ブラウザソース (T0808): ページ・ファイル・フレームの WebSocket・スイッチとトークン・描画の負荷と読めなかったことの受け取り
    source_check.rs               ブラウザソースの画の確かめの待ち受け (手で実行する ignored のテスト。下の「ブラウザソースの画を確かめる」)
    http.rs                       要求の頭の読み取りと検査 (Host・転送ヘッダ・Origin)
    ws.rs                         WebSocket の握手の検査と接続の設定 (フレームは tungstenite)
    rpc.rs                        JSON-RPC 2.0 の読み書きと誤りの符号
    store.rs                      studio-api.json (有効・ポート・トークン・承認したクライアントの鍵・ブラウザソースのスイッチとトークン。アプリのデータの場所)
    commands.rs                   操作ウィンドウのパネルが呼ぶコマンド
    tests.rs                      接続と操作のテスト (承認・権限・操作・通知・本物の待ち受け・JSON Schema との一致・ブラウザソースの経路と配信)
  tests/follow_match.rs         一致の検査 (ADR 0028 の 10.)
  tests/sample_profile.rs       見本のプロファイル (assets/samples) を Profile::read で開け、フォルダごと写しても開け、見本の印がある (T0865・T0866)
  tests/virtual_camera_gpu.rs   仮想カメラの画が、物理を有効にした 1 つの Puppet の画と一致する (`just test-gpu`)
  tauri.windows.conf.json       Windows のビルド: 仮想カメラの DLL を作る・NSIS を管理者の権限で入れる
  windows/hooks.nsh             NSIS: 仮想カメラの DLL を入れて regsvr32 で登録する・消す
  tauri.macos-vcam.conf.json    macOS で Camera Extension を入れたスタジオを作る設定 (`just app::build TARGET=studio PLATFORM=macos VCAM=1` が `--config` で重ねる。docs/release.md#macos-の仮想カメラ)
  Entitlements.vcam.plist       その entitlement (Entitlements.plist に System Extension を入れる権限を足したもの)
scripts/build-vcam.mjs        `pnpm vcam`: 仮想カメラのメディアソースの DLL を作る (Windows 向け。CRT は静的)
scripts/build-vcam-macos.mjs  `pnpm vcam:macos`: macOS の Camera Extension を swiftc で作って署名する (macOS だけ)
scripts/sample-profile.mjs    `pnpm sample-profile [--check]`: 見本のモデルを相対パスで指す見本のプロファイル (assets/samples の ugoe-sample-profile.json・.en.json) を書き出す・確かめる (T0806・T0865。docs/manual/studio.md#見本のプロファイル)
vitest.config.ts              フロントエンドのテスト (`pnpm test`)。部品と $effect のテストは先頭に `@vitest-environment happy-dom` を書く
vcam/                         仮想カメラの OS の境界 (クレート `ugoe-studio-vcam`。unsafe を許すアプリのクレート。下の「仮想カメラ」)
gpu-priority/                 Windows の GPU の優先度 (クレート `ugoe-studio-gpu-priority`。unsafe を許すアプリのクレート。下の「GPU の優先度」)
```

## 仕組み

- **指揮役 (バックエンド):** 刻みのスレッドが設定の速さ (30 / 60 / 120 回/秒。既定 60) で `dt` を測り (0.1 秒で頭打ち)、物理を切った `Puppet` を `update(dt)` して、
  `{seq, revision, dt, values, lip_sync}` のフレーム (`values` は物理の段に入る直前の値。T0134) を出力ウィンドウへ Tauri の `Channel` で配る。WebView の外で回すので、ウィンドウの抑制を受けない。
  操作ウィンドウへは、シーン (パラメータの定義) と、毎秒 10 回に間引いた値・1 刻みの時間・実際の刻みの速さ・受け手の数と描画の負荷を別の `Channel` で配る。
- **隠れても止まらない:** 両方のウィンドウで WebView のバックグラウンドの抑制を切る (`tauri.conf.json` の `backgroundThrottling` と、
  WebView2 の引数 `src/gpu.rs` の `BROWSER_ARGS`)。2 つのウィンドウは `tauri.conf.json` に `create: false` で書き、`setup` で使う GPU の設定を
  読んでから、同じ引数を足してコードで作る (`lib.rs` の `window_configs`。T0860)。出力ウィンドウは届いたフレームを届いたときに適用し、描画だけを `requestAnimationFrame` で行う。
  OS ごとの効き方と、OBS での取り込みの手順は [docs/studio.md](../../docs/studio.md#隠れても止まらない)。
- **受け手 (出力ウィンドウ):** `@ugoe/player` の受け手のモード (`Player.startFollowing`)。シーンの通知を受けたら `scene_model(revision)` で
  モデルのバイト列を取って読み、そのシーンのフレームを順にすべて `follow` で適用して (物理・リップシンク・ポーズだけが動く)、最後に 1 回描く。
  新しいフレームも見せ方の変化も無い `requestAnimationFrame` では描かない (描く回数の上限は刻みの速さ。T0857)。
- **操作ウィンドウはモデルを描かない** (GPU の負荷を出力の分だけにする)。様子は値と数値で示す。
- パラメータのスライダーは指揮役に書き込む (`set_parameter`。次の刻みから基準値になる)。動かしてから 0.5 秒は、指揮役からの値で表示を上書きしない。
- 言語は「OS に合わせる」が既定で、OS の言語は `tauri-plugin-os` の `locale()` で取る (WebView の `navigator.languages` は OS と違うことがある。
  `packages/i18n/README.md`)。選んだ言語は `ugoe.studio.locale` に覚える。
- 言語とテーマ (OS に合わせる・ライト・ダーク。`@ugoe/ui` の `ThemeSetting`) はツールバーの歯車の設定のダイアログ。テーマは `ugoe.studio.theme` に覚える。
  操作ウィンドウだけに効かせ、出力ウィンドウ (`output.ts`) は @ugoe/ui を読まないので透過のまま。
- 出力ウィンドウの大きさは物理ピクセルのプリセット (1920×1080・1280×720・1080×1920・1080×1080・自由)。選んだものはプロファイルに持ち、
  プロファイルを開いたときに合わせる (T0802)。
- **シーンとプロファイル (T0802):** プロファイル (版の番号付きの JSON) は舞台 (`stage.rs`) が持ち、変わるたびに操作ウィンドウへ `profile` の
  通知を配る。シーンの切り替えは指揮役の `present` で行い、受け手には `scene` の通知 (モデルとモジュールの版・見せ方) を配る。
  形式と切り替えの規則は [docs/studio.md](../../docs/studio.md#シーンとプロファイル)。

### Tauri のコマンド

| コマンド | ウィンドウ | 内容 |
| --- | --- | --- |
| `open_file_dialog(filterName, extensions)` / `save_file_dialog(filterName, defaultName)` | 操作 | OS の開く・保存のダイアログ (プロファイルの保存は `.json` を付ける)。選ばれたパスを返し、コマンドが読み書きできるパスに加える (下の「ファイルの読み書きの許可」) |
| `restore_profile()` | 操作 | 起動して最初の 1 回だけ、最後に使ったプロファイルを開く |
| `new_profile()` / `open_profile(path)` / `save_profile(path?)` / `forget_recent_profile(path)` | 操作 | プロファイル (`save_profile` は省くと今の保存先) |
| `add_scene(name)` / `duplicate_scene(id, name)` / `move_scene(id, index)` / `remove_scene(id)` | 操作 | シーンの一覧 |
| `update_scene(scene)` / `activate_scene(id)` | 操作 | シーンの設定を置き換える (配信中なら出し直す) / 切り替える (ファイルの拡張子と大きさを確かめる) |
| `show_pose(part)` | 操作 | 出しているシーンのポーズで、要素 `part` をそのグループの表示する要素にし、シーンの `pose` に覚えて出し直す (T0842。docs/studio.md#ポーズの要素) |
| `set_output_preset(preset)` | 操作 | 出力ウィンドウの大きさのプリセットをプロファイルに持つ |
| `subscribe_status(channel)` | 操作 | プロファイル・シーン・値・負荷・出力ウィンドウの状態・受け手が読めなかったことを受け取る |
| `set_parameter(id, value)` / `reset_parameters()` | 操作 | 指揮役のパラメータを書き換える |
| `output_state()` / `set_output_visible(visible)` / `set_output_on_top(onTop)` / `set_output_size(width, height)` | 操作 | 出力ウィンドウ |
| `tick_rate()` / `set_tick_rate(rate)` | 操作 | 刻みの速さ (30 / 60 / 120。ほかの値はエラー)。プロファイルの `output.tickRate` に持つ |
| `gpu_settings()` / `set_gpu_preference(preference)` | 操作 | 使う GPU (`auto`・`lowPower`・`highPerformance`) の様子 (今の希望・起動したときの希望・グラフィックの設定を開けるか・GPU の優先度と当てられなかった理由) と選び直し。機械ごとの設定ファイル `studio-settings.json` (アプリの設定の場所) に持つ (T0860。docs/studio.md#使う-gpu) |
| `set_gpu_priority(priority)` | 操作 | GPU の優先度 (`normal`・`low`) を選び、設定ファイルに書いて、スタジオと WebView2 の GPU のプロセスにすぐ当てる。当てられなかった理由は戻り値の `priorityErrors`。WebView2 が GPU のプロセスを作り直して当て直したときは `subscribe_status` に `gpu` の通知で同じ形を送る。Windows の外では `gpu_priority.unsupported` (T0861。docs/studio.md#gpu-の優先度) |
| `open_graphics_settings()` | 操作 | Windows の「グラフィックの設定」を開く (ほかの OS ではエラー) |
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
| `report_render_stats(stats)` | 出力 | 描画の負荷 (回/秒・1 回の時間の平均と最大・捨てたフレーム・描画の API と GPU の名前) を約 1 秒ごとに送る。操作ウィンドウに出す |
| `output_power_preference()` | 出力 | 起動したときの「使う GPU」を `WebPuppet.create` の電力の希望 (`default`・`low-power`・`high-performance`) で返す |
| `report_load_failure(failure)` | 出力 | 今のシーンのモデル・モジュール・背景を読めなかったこと (版・何を・理由のコード) を送る。操作ウィンドウに出す (`subscribe_status` の `loadFailures`。docs/studio.md#受け手が読めなかったとき) |
| `virtual_camera_state()` | 操作 | 仮想カメラの状態 (止めている・始めている・動いている・止まった。始められない理由も) |
| `start_virtual_camera(settings?)` / `stop_virtual_camera()` | 操作 | 仮想カメラをプロファイルの設定で始める (動いていれば始め直す。`settings` (色は RGB の配列) を渡すと先にプロファイルの設定にする)・止める。状態の変化は `subscribe_status` の `virtualCamera` で届く |
| `set_virtual_camera_settings(settings)` | 操作 | プロファイルの `output.virtualCamera` を変える (色は `#rrggbb`)。動いていれば効かせる (解像度とフレームレートは始め直し、色は次の画から) |

- コマンドのエラーは `{ code, args, message }` の形 (下の「コマンドのエラー」)。

- 出力ウィンドウは閉じる操作で隠すだけにする (受け手を作り直さない)。操作ウィンドウを閉じるとアプリを終える (仮想カメラも止める)。
- 権限 (`src-tauri/capabilities/`): 操作ウィンドウは OS の言語、出力ウィンドウはウィンドウのドラッグだけ。ファイルのダイアログの権限は
  webview に渡さない (バックエンドが開く。下の「ファイルの読み書きの許可」。`src/acl.rs` のテストが検査する)。
  アプリのコマンドも、ウィンドウごとに呼べるものを絞る (T0820): `build.rs` が `src/acl.rs` の `COMMANDS` を
  `tauri_build::AppManifest::commands` に渡して `allow-<コマンド>` の許可を作り、出力ウィンドウ (`output.json`) は受け手のコマンド
  (`OUTPUT_COMMANDS`: 上の表の「出力」) だけ、操作ウィンドウ (`control.json`) はそれ以外のすべてを許す。コマンドを足したら
  `generate_handler!`・`COMMANDS`・どちらかの capability の 3 か所に書く (`src/acl.rs` のテストが食い違いを検査する)。
- CSP (`tauri.conf.json` の `app.security.csp`): 同梱したものだけを読む (`default-src 'self'`)。wasm (`'wasm-unsafe-eval'`)・
  Tauri の IPC (`ipc:`・`http://ipc.localhost`)・blob の画像と動画 (背景・トラッキングのファイル) を許し、ほかのページの枠には入れさせない
  (`frame-ancestors 'none'`)。カメラ (`srcObject` の MediaStream) は CSP の対象外。外への `fetch` は許さない (MediaPipe の利用状況の
  送り先 `odml.pa.googleapis.com` も。送信は `@ugoe/face-tracking` が送る前に断る。
  [packages/face-tracking](../../packages/face-tracking/README.md#利用状況の送信-t0943)、T0943)。値はエディタ・ビューアと同じ (T0024。
  [docs/architecture.md](../../docs/architecture.md#webview-の-csp-とコマンドの許可)。`src/acl.rs` のテストが 3 つの一致を検査する)。
  Svelte が書く `style` 属性のため `style-src` は `'unsafe-inline'` にし、Tauri が `style-src` に hash を足さないようにする
  (`dangerousDisableAssetCspModification`。hash があると `'unsafe-inline'` が効かなくなる)。
  カメラとマイクは、バックエンドの `on_permission_request` が操作ウィンドウの要求だけを許す (macOS は `Info.plist` と `Entitlements.plist`)。
  ホットキーはバックエンドが `tauri-plugin-global-shortcut` で登録する (JS の口は使わないので、プラグインの権限は付けない)。
- 操作ウィンドウへの通知には、ほかに `animation` (モーション・表情・トラッキングの様子)・`hotkeys` (登録できなかった割り当て)・
  `hotkey` (押されたホットキー。キャリブレーションと、行えなかったときの理由。操作 API の `tracking.calibrate` も同じ通知を使う)・
  `api` (操作 API の様子。待ち受け・承認したアプリ・承認を待つアプリ) がある。
- 出力ウィンドウの背景の透過は、macOS では `macOSPrivateApi` (tauri の `macos-private-api` feature) が要る (Mac App Store では配布できなくなる)。

### ファイルの読み書きの許可 (T0838)

webview で思わぬスクリプトが動いても、利用者が選んでいないファイルを読み書きできないようにする (ビューアの `FileAccess` (T0727) と同じ考え。
`src-tauri/src/access.rs`。ダイアログで選んだパスを覚える部分はエディタ・ビューアと共通の `ugoe_fs::FileAccess`)。ファイルのダイアログはバックエンドが開き (`open_file_dialog`・`save_file_dialog`)、選ばれたパスを覚える。

- **読める:** 開くダイアログで選んだパス (このセッションの間。覚えておかない)・今のプロファイルにあるパス (シーンのモデル・モジュール・
  背景の画像と、ホットキーの着せ替えのモジュール)・最近のプロファイル (`recent-profiles.json`。バックエンドが書いたもの)。
  プロファイルにあるパスは、利用者がそのプロファイルを選んで開いた (または自分で選んで保存した) ものとして扱う。
  プロファイルのファイルの中の相対パスは、読むときにプロファイルのフォルダを基準に絶対パスに解いてある (T0852) ので、許可も
  `dressup.*` も解いた絶対パスで比べる。フォルダの外に出られる相対パス (`..` など) は解かずに残り、`stage::read_asset` が相対パスを
  読まない (今のディレクトリを基準に読まない)。規則は [docs/studio.md](../../docs/studio.md#プロファイルのパス)。
- **書ける:** 保存のダイアログで選んだパスに 1 回だけ。今の保存先 (`save_profile` の `path` を省いたとき。自動の保存) はバックエンドが持つ。
- 許していないパスは `file.not_chosen` (読む)・`file.not_chosen_for_save` (書く) のエラーにする。

| コマンド | 受けるパス | 確かめること |
| --- | --- | --- |
| `open_profile(path)` | プロファイル | 読めるパス (選んだ・最近のプロファイル) |
| `save_profile(path?)` | 保存先 | `path` を渡したら、保存のダイアログで選んだもの (1 回だけ)。省けば今の保存先 |
| `update_scene(scene)` | シーンのモデル・モジュール・背景の画像 | すべて読めるパス (選んだ・今のプロファイルにある) |
| `set_hotkeys(hotkeys)` | 着せ替えのモジュール | すべて読めるパス |
| `module_slot(path)` | モジュール | 読めるパス |

例外 (パスを受けるが絞らないもの、パスを受けずにファイルを読み書きするもの):

- `forget_recent_profile(path)`: 最近の一覧から外すだけで、`path` は読み書きしない。
- `restore_profile`・`activate_scene`・`duplicate_scene`・ホットキーの操作・`scene_model` などの受け手のコマンド: パスを受けず、
  今のプロファイル (と最近のプロファイル) にあるパスだけを読む。
- 操作 API (`docs/studio-api.md`) はパスを受けない。シーンは ID、着せ替えのモジュールは `dressup.list` が返す ID (今のプロファイルにある
  モジュールだけ) で指す ([docs/studio.md](../../docs/studio.md#外部-api) の「着せ替え」)。
- アプリが自分で決めた場所 (最近のプロファイル・操作 API の設定) は webview からパスを受けない。

### コマンドのエラー (T0835)

コマンドは失敗すると、[ADR 0029](../../docs/adr/0029-shared-ui-and-i18n.md) の形 `{ code, args, message }` で reject する
(`src-tauri/src/error.rs` の `CommandError`。エディタ・ビューアと同じ形)。`message` は英語の文 (ログ・訳の無い画面の向け)。
ホットキーを行えなかった知らせ (`subscribe_status` の `hotkey` の `error`) も同じ形。
操作ウィンドウは reject された値をそのまま受け、`src/lib/errors.ts` の `describeError` でカタログの `errors.<code>` を `args` で
引いた今の言語の文にする (カタログに無いコードは `message`。数の引数は今の言語の書式)。コードが `scene.` で始まるエラーは、
そのシーンに出す (`scene.model_missing` なら選び直すボタンも)。文言を出さない出力ウィンドウとブラウザソースは、同じ形を
`src/lib/commandMessage.ts` で英語の文にする (@ugoe/i18n を読まないため)。Rust に訳のカタログは置かない。

コードは公開の約束として扱い、名前を変えない (意味が変わるなら新しいコードにする)。`path` はパスの文字列、`reason` は英語の文
(OS やパーサの誤り)、`asset` はシーンのファイルの種類 (`model`・`module`・`background`)。

| コード | 引数 | いつ |
| --- | --- | --- |
| `internal` | `reason` | 別のスレッドの処理が終わらなかった (パニックした)・スレッドを作れなかった。ビューアと同じコード |
| `file.not_chosen` | `path` | 読もうとしたパスを利用者が選んでいない (上の「ファイルの読み書きの許可」)。ビューアと同じコード |
| `file.not_chosen_for_save` | `path` | 書こうとしたパスを保存のダイアログで選んでいない。ビューアと同じコード |
| `dialog.failed` | `reason` | OS のファイルのダイアログが失敗した。ビューアと同じコード |
| `profile.io` | `path`・`reason` | プロファイルを読み書きできない |
| `profile.too_large` | `path`・`size` (バイト) | プロファイルが大きすぎる (16 MiB まで) |
| `profile.not_json` | `path`・`reason` | JSON でない |
| `profile.not_profile` | `path` | スタジオのプロファイルでない (`format` が違う) |
| `profile.future_version` | `path`・`version`・`supported` | このスタジオより新しい版のプロファイル |
| `profile.invalid` | `path`・`reason` | 形が壊れている |
| `scene.unknown` | `id` | そのシーンが無い |
| `scene.model_missing` / `scene.module_missing` / `scene.background_missing` | `asset`・`path` | シーンのモデル・モジュール・背景の画像が見つからない |
| `scene.unreadable` | `asset`・`path`・`reason` | シーンのファイルを読めない |
| `scene.wrong_type` | `asset`・`path` | 拡張子が違う |
| `scene.too_large` | `asset`・`path`・`size` (バイト) | シーンのファイルが大きすぎる |
| `scene.dress_up` | `reason` | モジュールを付けられない (スロットが無いなど) |
| `scene.unknown_pose` | `part` | 出しているモデル (と付けているモジュール) のポーズに、その要素が無い (`show_pose`・ホットキーのポーズの切り替え) |
| `scene_file.not_current` / `scene_file.not_found` | `revision` | 受け手が取ろうとしたファイル: 今のシーンでない・そのファイルが無い |
| `animation.no_model` | | 出しているモデルが無い |
| `animation.unknown_motion` / `animation.unknown_expression` | `name` | その名前のモーション・表情がモデルに無い |
| `animation.motion_rejected` | | 再生中のモーションより優先度が低くて始まらなかった (ホットキーの知らせだけ) |
| `tick_rate.unsupported` | `rate` | 刻みの速さが 30 / 60 / 120 でない |
| `render_stats.invalid` | | 出力の報告した描画の負荷が数でない・負 (API と GPU の名前が 256 バイトを超える) |
| `gpu_settings.save_failed` | `reason` | 使う GPU の設定 (GPU の優先度を含む) を設定ファイルに書けない |
| `gpu_priority.unsupported` | | GPU の優先度を変えられない OS (Windows の外) |
| `gpu_priority.access_denied` | `process` | プロセスの GPU の優先度を変える権限が無い |
| `gpu_priority.process_not_found` | `process` | GPU の優先度を当てる前にプロセスが終わった |
| `gpu_priority.webview_too_old` | | WebView2 Runtime が古く、GPU のプロセスを知れない |
| `gpu_priority.no_gpu_process` | | WebView2 の GPU のプロセスが無い |
| `gpu_priority.failed` | `process`・`reason` | そのほかの理由で GPU の優先度を変えられない |
| `graphics_settings.unsupported` | | Windows の外で「グラフィックの設定」を開こうとした |
| `graphics_settings.failed` | `reason` | 「グラフィックの設定」を開けない |
| `output_window.missing` | | 出力ウィンドウが無い |
| `output_window.failed` | `reason` | 出力ウィンドウの表示・最前面・大きさを変えられない |
| `output_window.unsupported_size` | `width`・`height` | 出力の大きさが 16〜8192 の外 |
| `virtual_camera.invalid_config` | `width`・`height`・`fps` | 仮想カメラに選べない解像度・フレームレート |
| `api.unsupported_port` | `port`・`min` | 操作 API のポートが 1024 未満 |
| `api.unknown_pending_client` | `id` | 承認を待つクライアントにその ID が無い (もう切れた・断った) |
| `api.too_many_clients` | `max` | 承認したクライアントが多すぎる |
| `api.random_failed` | `reason` | OS の乱数を取れず、トークン・鍵を作れない |

仮想カメラの止まった理由 (`virtual_camera::Status` の `code`。`virtualCamera.errors.*`) と、受け手が読めなかった理由
(`loadFailures` の `code`) は、コマンドのエラーではなく状態の一部なので、この形にしない。

**操作 API との関係:** 操作 API ([docs/studio-api.md](../../docs/studio-api.md)) の `-32010` の `error.data` は、
`ProfileError`・`SceneError`・`AnimationError` をそのまま書いた別の形 (`{ code: "unknownScene", id: … }` のように引数が平らで、
コードも camelCase の別の名前) にしている。外のツールとの公開の約束で、同じ版のバックエンドと画面がいつも一緒に配られる
コマンドのエラーとは事情が違うため、名前と形は変えない (`error.message` が英語の文で、`message` に当たる)。
同じエラーから両方を作るので、意味は対応する (`unknownScene` ↔ `scene.unknown`、`missing` ↔ `scene.<asset>_missing`、
`noModel` ↔ `animation.no_model` など。対応は `src-tauri/src/error.rs` の `From` の実装)。
元のフィールドはそのまま残し、コマンドのエラーと同じコードと引数を `messageCode`・`args` として足している (T0839。
`src-tauri/src/api/rpc.rs` の `RpcError::failed`)。外のツールもカタログの `errors.<messageCode>` で同じ訳を使える。

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
- Windows と macOS の Rust のコードは、Linux からも `just vcam-check` で検査できる (`cargo clippy -p ugoe-studio-vcam --all-targets -- -D warnings` を
  `--target x86_64-pc-windows-msvc` と `aarch64-apple-darwin` で。ターゲットが無ければ `rustup target add` の入れ方を出す)。
  CI でも `rust (cross)` のジョブが同じ検査を流す ([docs/testing.md#別のターゲットの検査](../../docs/testing.md#別のターゲットの検査))。
  `just ci` には含まれないので、`src/mf/`・`src/cmio.rs` を触ったら実行する。リンクまでは `cargo xwin build` (Windows)、
  Swift の拡張は macOS でしか作れない。

## GPU の優先度

Windows で、スタジオのプロセス (仮想カメラ) と WebView2 の GPU のプロセス (出力ウィンドウと操作ウィンドウ) の GPU のスケジューラの優先度を
「通常」か「低い」(`BELOW_NORMAL`) にする (T0861)。仕様は [docs/studio.md#gpu-の優先度](../../docs/studio.md#gpu-の優先度)、
`unsafe` の例外は [ADR 0034](../../docs/adr/0034-studio-gpu-priority.md)。

- `gpu-priority/` (`ugoe-studio-gpu-priority`、GPL-3.0-only) は OS の呼び出しだけを持つ。内部のクレートに依存しない。
  - `src/lib.rs`: 優先度 (`GpuPriority`) とエラー (`Error`)。どの OS でもビルドする。
  - `src/win.rs`: Windows だけ。`set_current_process_priority`・`set_process_priority` (`D3DKMTSetProcessSchedulingPriorityClass`)、
    `webview2::watch_gpu_processes` (`ICoreWebView2Environment8` の GPU のプロセスの番号と `ProcessInfosChanged`)。
  - **`unsafe` を許す** (ADR 0034)。lint は仮想カメラのクレートと同じ (`undocumented_unsafe_blocks` と `unsafe_op_in_unsafe_fn` を拒否)。
- バックエンドは `src-tauri/src/gpu_priority.rs` (当てる先と記録。OS を呼ぶ口を引数で受けるので Linux でもテストする) と、`lib.rs` の
  `set_gpu_priority`・`watch_gpu_priority` (起動したときに当て、`with_webview` の中で WebView2 の GPU のプロセスを見張る)。
- Windows のコードは `just cross-check gpu-priority` (`x86_64-pc-windows-msvc`) と `just cross-check apps` (`ugoe-studio` の Windows のコード) で検査する。

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
`UGOE_STUDIO_CHECK_NO_LIGHT` (光源を切る)・`UGOE_STUDIO_CHECK_RENDER_SCALE` (出力ウィンドウとページの描く解像度の倍率、既定 1。T0858) で条件を変えられる。ループバックの外の Origin のページ (Playwright の `route` で返したものなど) から
ループバックへ接続するときは、Chrome の Local Network Access の検査で止められることがある (`--disable-features=LocalNetworkAccessChecks`)。
OBS のページはスタジオ自身が出すので、この検査には当たらない。

## ヘッドレスのブラウザで確かめる

Tauri の外では、ページを読む前に `window.__UGOE_STUDIO_HOST__` に `StudioHost` (`src/lib/host.ts`) を置く (Playwright の `addInitScript` など)。
置かなければ「Tauri の中で動かしてください」というエラーを出す。操作ウィンドウのトラッカーは `window.__UGOE_TRACKER__`
(`startReplay(text)` で記録を流せる)。Chrome の偽のカメラ・マイク (`--use-fake-device-for-media-stream --use-file-for-fake-video-capture=<.y4m>
--use-file-for-fake-audio-capture=<.wav>`) で入力の経路を確かめられる。出力ウィンドウの `Player` は `window.__UGOE_STUDIO_OUTPUT__` で見られる
(`status`・`following`・`error`)。操作ウィンドウに直近に届いた状態 (`StatusMessage`。負荷とトラッキングの待ち) は `window.__UGOE_STUDIO_STATUS__`。WebGPU の無いヘッドレスの Chrome でも、SwiftShader の WebGL2 で描ける
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
pnpm --filter @ugoe/studio sample-profile  # 見本のプロファイル (assets/samples にコミットしたもの) を書き直す。--check は食い違いを確かめるだけ (pnpm test も確かめる)
cargo test -p ugoe-studio         # 指揮役・配信の形・一致の検査・操作 API (本物のループバックの待ち受けも使う)
cargo test -p ugoe-studio-vcam    # 仮想カメラの共有メモリの形・設定・v4l2 の構造体
cargo test -p ugoe-studio-gpu-priority  # GPU の優先度のクラスの値 (OS の呼び出しは Windows だけ)
cargo test -p ugoe-studio --features gpu-tests --test virtual_camera_gpu  # 仮想カメラの画 (just test-gpu に含まれる。CI では除く)
```

必要な道具 (Rust・Node.js・pnpm・just・wasm-bindgen-cli・各 OS の前提パッケージ) はエディタと同じ
([apps/editor/README.md#各-os-でのビルド](../editor/README.md#各-os-でのビルド))。
署名もエディタと同じ ([apps/editor/README.md#配布物の署名](../editor/README.md#配布物の署名))。署名の環境変数があれば、
Tauri が署名しない仮想カメラの DLL にも `pnpm vcam` が同じコマンドで署名する
([docs/release.md#デスクトップアプリの署名](../../docs/release.md#デスクトップアプリの署名))。無ければ DLL も署名しない。
チェックサム (`.sha256`) と成果物の証明もエディタと同じに添える ([docs/release.md#配布物を確かめる](../../docs/release.md#配布物を確かめる))。
