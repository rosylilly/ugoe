# apps/studio

ugoe のスタジオ (Tauri v2 + Svelte 5)。配信者が配信中に使うアプリで、モデルを **出力ウィンドウ** に出し、**操作ウィンドウ** から動かす。
範囲・構成・描画の場所は [ADR 0028](../../docs/adr/0028-studio.md)、画面と配信の形は [docs/studio.md](../../docs/studio.md)。
T0801 で骨組み (2 つのウィンドウ・指揮役と受け手) を作った。シーンとプロファイル (T0802)・入力 (T0803)・配信向けの出力 (T0804)・
外部 API (T0805)・仮想カメラ (T0807・T0809)・ブラウザソース (T0808) は後続のタスク。

**ライセンスは GPL-3.0-only** (エディタ・ビューアと同じ)。アプリ同士で import はしない (共有するものは `packages/` に置く)。

```
index.html / src/control.ts   操作ウィンドウの入口 (@ugoe/ui のトークン・言語の設定)
src/Control.svelte            操作ウィンドウ: モデルを開く・出力ウィンドウの表示/最前面/大きさ・いまの状態・パラメータ・言語
output.html / src/output.ts   出力ウィンドウの入口 (背景は透過。@ugoe/ui と @ugoe/i18n は読まない)
src/Output.svelte             出力ウィンドウ: @ugoe/player の受け手のモードでモデルだけを描く。Alt + ドラッグで動かす
src/lib/host.ts               バックエンドとの口 (StudioHost)。Tauri の外では window.__UGOE_STUDIO_HOST__ を使う
src/lib/settings.ts           覚える設定 (言語・出力の大きさのプリセット。localStorage)
src/lib/i18n.ts               文言 (@ugoe/i18n の名前空間 `studio`。カタログは locales/)
src/lib/wasm/                 ugoe-wasm の読み込み。generated/ は `pnpm wasm` の出力 (gitignore 済み)
src-tauri/                    バックエンド (Rust クレート `ugoe-studio`)
  src/conductor.rs              指揮役: 物理を切ったネイティブの Puppet。刻みごとにフレームを作る
  src/stream.rs                 配信の形 (フレームのバイト列・シーンの JSON)
  src/follower.rs               ネイティブの受け手 (一致の検査で使う。仮想カメラ (T0807) も使う)
  src/lib.rs                    刻みのスレッド・Tauri のコマンド・出力ウィンドウの操作
  tests/follow_match.rs         一致の検査 (ADR 0028 の 10.)
```

## 仕組み

- **指揮役 (バックエンド):** 刻みのスレッドが 60 回/秒で `dt` を測り (0.1 秒で頭打ち)、物理を切った `Puppet` を `update(dt)` して、
  `{seq, revision, dt, values, lip_sync}` のフレーム (`values` は物理の段に入る直前の値。T0134) を出力ウィンドウへ Tauri の `Channel` で配る。WebView の外で回すので、ウィンドウの抑制を受けない。
  操作ウィンドウへは、シーン (パラメータの定義) と、毎秒 10 回に間引いた値・1 刻みの時間・受け手の数を別の `Channel` で配る。
- **受け手 (出力ウィンドウ):** `@ugoe/player` の受け手のモード (`Player.startFollowing`)。シーンの通知を受けたら `scene_model(revision)` で
  モデルのバイト列を取って読み、そのシーンのフレームを順にすべて `follow` で適用して (物理・リップシンク・ポーズだけが動く)、最後に 1 回描く。
- **操作ウィンドウはモデルを描かない** (GPU の負荷を出力の分だけにする)。様子は値と数値で示す。
- パラメータのスライダーは指揮役に書き込む (`set_parameter`。次の刻みから基準値になる)。動かしてから 0.5 秒は、指揮役からの値で表示を上書きしない。
- 言語は「OS に合わせる」が既定で、OS の言語は `tauri-plugin-os` の `locale()` で取る (WebView の `navigator.languages` は OS と違うことがある。
  `packages/i18n/README.md`)。選んだ言語は `ugoe-studio.locale` に覚える。
- 出力ウィンドウの大きさは物理ピクセルのプリセット (1920×1080・1280×720・1080×1920・1080×1080・自由)。選んだものは `ugoe-studio.output-size`
  に覚え、起動したときに合わせる。プロファイル (T0802) ができたらそちらへ移す。

### Tauri のコマンド

| コマンド | ウィンドウ | 内容 |
| --- | --- | --- |
| `open_model(path)` | 操作 | `.ugoe` か JSON のモデルを開いて新しいシーンにする (拡張子と大きさを確かめる) |
| `subscribe_status(channel)` | 操作 | シーン・値・負荷・出力ウィンドウの状態を受け取る |
| `set_parameter(id, value)` / `reset_parameters()` | 操作 | 指揮役のパラメータを書き換える |
| `output_state()` / `set_output_visible(visible)` / `set_output_on_top(onTop)` / `set_output_size(width, height)` | 操作 | 出力ウィンドウ |
| `subscribe_frames(channel)` | 出力 | シーンの通知 (JSON) とフレーム (バイト列) を受け取る。開き直したページは前の Channel を置き換える |
| `scene_model(revision)` | 出力 | 今のシーンのモデルのバイト列 (今のシーンでなければエラー) |

- 出力ウィンドウは閉じる操作で隠すだけにする (受け手を作り直さない)。操作ウィンドウを閉じるとアプリを終える。
- 権限 (`src-tauri/capabilities/`): 操作ウィンドウはファイルを開くダイアログと OS の言語、出力ウィンドウはウィンドウのドラッグだけ。
- 出力ウィンドウの背景の透過は、macOS では `macOSPrivateApi` (tauri の `macos-private-api` feature) が要る (Mac App Store では配布できなくなる)。

## ヘッドレスのブラウザで確かめる

Tauri の外では、ページを読む前に `window.__UGOE_STUDIO_HOST__` に `StudioHost` (`src/lib/host.ts`) を置く (Playwright の `addInitScript` など)。
置かなければ「Tauri の中で動かしてください」というエラーを出す。出力ウィンドウの `Player` は `window.__UGOE_STUDIO_OUTPUT__` で見られる
(`status`・`following`・`error`)。WebGPU の無いヘッドレスの Chrome でも、SwiftShader の WebGL2 で描ける
(`--use-angle=swiftshader --enable-unsafe-swiftshader`)。

## コマンド

```sh
just studio                       # 開発モードで起動 (vite の 1440 番ポート + tauri dev)
just studio-build                 # リリースビルド (インストーラを作る)。例: just studio-build --bundles deb
pnpm --filter @ugoe/studio wasm   # ugoe-wasm をビルドして src/lib/wasm/generated/ に JS バインディングを出す
cargo test -p ugoe-studio         # 指揮役・配信の形・一致の検査
```

必要な道具 (Rust・Node.js・pnpm・just・wasm-bindgen-cli・各 OS の前提パッケージ) はエディタと同じ
([apps/editor/README.md#各-os-でのビルド](../editor/README.md#各-os-でのビルド))。
