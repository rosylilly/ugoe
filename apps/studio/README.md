# apps/studio

ugoe のスタジオ (Tauri v2 + Svelte 5)。配信者が配信中に使うアプリで、モデルを **出力ウィンドウ** に出し、**操作ウィンドウ** から動かす。
範囲・構成・描画の場所は [ADR 0028](../../docs/adr/0028-studio.md)、画面と配信の形は [docs/studio.md](../../docs/studio.md)。
T0801 で骨組み (2 つのウィンドウ・指揮役と受け手)、T0802 でシーンとプロファイル、T0803 で入力 (カメラのトラッキング・マイクの口パク・
モーションと表情・グローバルなホットキー)、T0804 で配信向けの出力を作った。外部 API (T0805)・仮想カメラ (T0807・T0809)・ブラウザソース (T0808) は後続のタスク。

**ライセンスは GPL-3.0-only** (エディタ・ビューアと同じ)。アプリ同士で import はしない (共有するものは `packages/` に置く)。

```
index.html / src/control.ts   操作ウィンドウの入口 (@ugoe/ui のトークン・言語の設定)
src/Control.svelte            操作ウィンドウ: プロファイルのメニュー・モデルを選ぶ・出力ウィンドウの表示/最前面/大きさ・いまの状態・パラメータ・言語
src/LoadPanel.svelte          操作ウィンドウの「フレームレートと負荷」: 刻みの速さ (30 / 60 / 120)・1 刻みの時間・出力ごとの描画の負荷
src/ScenesPanel.svelte        シーンの一覧 (切り替え・追加・複製・並べ替え・削除)
src/SceneEditor.svelte        シーンの編集 (名前・モデル・モジュール・位置と大きさと回転・背景・ライティング)
src/LightingEditor.svelte     シーンの光源 (組・環境光・平行光源・点光源)
src/MotionPanel.svelte        表情とモーション (表情の付け外し・モーションの再生と優先度・待機モーション・トラッキング中のモーションの優先)
src/InputPanel.svelte         入力 (@ugoe/face-tracking のパネルでカメラのトラッキング、マイクの機器・感度・ノイズの閾値・メーター)
src/HotkeysPanel.svelte       ホットキーの割り当て (キーを押して決める・操作と引数・登録できない理由)
output.html / src/output.ts   出力ウィンドウの入口 (背景は透過。@ugoe/ui と @ugoe/i18n は読まない)
src/Output.svelte             出力ウィンドウ: @ugoe/player の受け手のモードでモデルだけを描く。シーンの見せ方を当てる。Alt + ドラッグで動かす
src/lib/host.ts               バックエンドとの口 (StudioHost)。Tauri の外では window.__UGOE_STUDIO_HOST__ を使う
src/lib/settings.ts           覚える設定 (言語・刻みの速さ。localStorage) と出力の大きさのプリセット
src/lib/load.ts               負荷の注意の判断 (刻みが遅れた・揺れた・描画が重い・フレームを捨てた・描いていない)
src/lib/profile.ts            プロファイルとシーンの形 (profile.rs と同じ)・エラーの形・背景や配置の既定値
src/lib/inputs.ts             入力・モーション・ホットキーの設定と演技の様子の形 (input.rs・hotkeys.rs・animation.rs と同じ)
src/lib/hotkeys.ts            ホットキーのキーの書き方 (押したキー → `Ctrl+Shift+KeyA`、Kbd の表記)
src/lib/lipSync.ts            マイクの音から口を開く量を作る式 (dB・ノイズの閾値・感度・開く速さと閉じる速さ)
src/lib/microphone.svelte.ts  マイク (getUserMedia → AnalyserNode)。刻みごとに口を開く量を指揮役へ送る
src/lib/pulse.ts              入力の処理を回す刻み (バックエンドの `subscribe_pulse`。requestAnimationFrame に頼らない)
src/lib/latest.ts             送っている間に届いた値は最後のものだけを次に送る (トラッキングと口パクの IPC)
src/lib/outputLayout.ts       受け手がシーンの通知ですること (planScene)・配置を舞台とビューと CSS の回転に分ける計算
src/lib/errors.ts             バックエンドのエラー (code) を文にする
src/lib/labels.ts             シーンの表示名
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
  src/follower.rs               ネイティブの受け手 (一致の検査で使う。仮想カメラ (T0807) も使う)
  src/lib.rs                    刻みのスレッド・Tauri のコマンド・出力ウィンドウの操作
  tests/follow_match.rs         一致の検査 (ADR 0028 の 10.)
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
| `subscribe_frames(channel)` | 出力 | シーンの通知 (JSON) とフレーム (バイト列) を受け取る。開き直したページは前の Channel を置き換える |
| `scene_model(revision)` / `scene_module(revision, index)` / `scene_background(revision)` | 出力 | 今のシーンのモデル・モジュール・背景の画像のバイト列 (今のシーンでなければエラー) |
| `report_render_stats(stats)` | 出力 | 描画の負荷 (回/秒・1 回の時間の平均と最大・捨てたフレーム) を約 1 秒ごとに送る。操作ウィンドウに出す |

- プロファイルとシーンのコマンドのエラーは `{ code, ... }` の形 (`profile.rs` の `ProfileError`・`stage.rs` の `SceneError`)。画面は `code` で文言を選ぶ。

- 出力ウィンドウは閉じる操作で隠すだけにする (受け手を作り直さない)。操作ウィンドウを閉じるとアプリを終える。
- 権限 (`src-tauri/capabilities/`): 操作ウィンドウはファイルを開く・保存先を選ぶダイアログと OS の言語、出力ウィンドウはウィンドウのドラッグだけ。
  カメラとマイクは、バックエンドの `on_permission_request` が操作ウィンドウの要求だけを許す (macOS は `Info.plist` と `Entitlements.plist`)。
  ホットキーはバックエンドが `tauri-plugin-global-shortcut` で登録する (JS の口は使わないので、プラグインの権限は付けない)。
- 操作ウィンドウへの通知には、ほかに `animation` (モーション・表情・トラッキングの様子)・`hotkeys` (登録できなかった割り当て)・
  `hotkey` (押されたホットキー。キャリブレーションと、行えなかったときの理由) がある。
- 出力ウィンドウの背景の透過は、macOS では `macOSPrivateApi` (tauri の `macos-private-api` feature) が要る (Mac App Store では配布できなくなる)。

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
just studio-build                 # リリースビルド (インストーラを作る)。例: just studio-build --bundles deb
pnpm --filter @ugoe/studio wasm   # ugoe-wasm をビルドして src/lib/wasm/generated/ に JS バインディングを出す
pnpm --filter @ugoe/studio mediapipe  # MediaPipe の wasm とモデルを public/mediapipe/ に用意する (dev・build でも実行される。gitignore 済み)
cargo test -p ugoe-studio         # 指揮役・配信の形・一致の検査
```

必要な道具 (Rust・Node.js・pnpm・just・wasm-bindgen-cli・各 OS の前提パッケージ) はエディタと同じ
([apps/editor/README.md#各-os-でのビルド](../editor/README.md#各-os-でのビルド))。
