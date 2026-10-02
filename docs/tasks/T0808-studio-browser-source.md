---
id: T0808
title: スタジオ: OBS のブラウザソース向けの URL をローカルで提供する
status: done
milestone: M8
crates: [apps/studio, bindings/web, packages/player, ugoe-headless, docs, Cargo.toml, Cargo.lock]
depends_on: [T0802, T0805, T0133]
assignee: claude-worker-T0808
---

# T0808: スタジオ: OBS のブラウザソース向けの URL

## 目的

OBS のブラウザソースに URL を入れるだけで、スタジオのアバターを背景の透過付きで取り込めるようにする。ウィンドウの取り込みより設定が簡単で、
OBS の中で大きさや位置を変えやすい。

## 受け入れ条件

- [x] スタジオがループバックで HTTP を待ち、ブラウザソース用のページを出す (既定は切。URL に推測できないトークンを含める)
- [x] ページは @ugoe/runtime (wasm) で描き、スタジオから状態を受け取る (ADR 0028 で決めた方式。T0805 のサーバと待ち受けを共有してよい)。
      シーンの切り替え・着せ替え・ライティングも追う
- [x] 操作ウィンドウに URL のコピーと、OBS への設定の手順 (大きさ・透過) を出す
- [x] ADR 0028 の 10. の一致の検査に、ブラウザソースのページの受け手を加える
- [x] 仮想カメラ (T0807) の画にも、シーンのライティングと背景の画像を当てる (`TODO(T0808)`)。出力ウィンドウ・ブラウザソース・仮想カメラの画を同じフレームで比べる
- [x] ヘッドレスの Chrome で、ページの画が出力ウィンドウと同じになることを確かめる (許容誤差付き)。実際の OBS での確認は利用者に残す
- [x] `just ci` が通る

## 参考

- ADR 0028、bindings/web/README.md

## メモ

- **待ち受けと経路** (`apps/studio/src-tauri/src/api/source.rs`): T0805 の待ち受けに `/source/<トークン>/` を足した。スイッチとトークンは
  `studio-api.json` の `source` (既定は切、256 ビット、定数時間で比べる)。待ち受けは操作 API とブラウザソースのどちらかが有効な間だけ動かす
  (`mod.rs` の `sync_listener`。`restart_api` を置き換えた)。`/api` は操作 API を入れているときだけ答える (切っていれば `404`)。
  経路: ページ (`source.html`)・`assets/…` (英数字と `-_.` だけ)・`frames` (WebSocket。最初に今のシーン、以後は出力ウィンドウと同じ順で
  シーン (テキスト) とフレーム (バイナリ))・`scene/<rev>/model|module/<i>|background`。切っているとき・トークンが違うときはすべて `404`。
  末尾の `/` が無ければ転送。応答に `Referrer-Policy: no-referrer`・`nosniff`・外を読まない CSP。ページからはテキストの描画の負荷だけを受け
  (負荷の表示に「ブラウザソース」「ブラウザソース (2)」として出す)、バイナリは閉じる。1 接続 256 メッセージ溜まったら閉じる (ページはつなぎ直す)。
  ページのファイルは Tauri の `AssetResolver` から読む (無いファイルに index.html を返すので、HTML でないはずのものが HTML なら無いものとする)。
  開発モードでは `dist/` を直に読むので、先に `vite build` が要る。
- **ページ:** 出力ウィンドウと同じ `Output.svelte` を、配信の口 (`feed` の prop。`src/lib/feed.ts` の `sourceFeed` = WebSocket と fetch) で動かす。
  wasm は `@ugoe/player` (中身は `@ugoe/runtime` と同じ ugoe-wasm)。`@ugoe/runtime` のパッケージそのものは使っていない (受け手の規則と見せ方の
  処理を出力ウィンドウと 1 つにするため。bindings/web は触っていない)。切れたら 1 秒後につなぎ直す。Vite の `base` を `"./"` にした
  (`/source/<トークン>/` の下でスクリプトと wasm を読むため。操作・出力ウィンドウも相対のパスになるが、Tauri でもそのまま読める)。
- **操作ウィンドウ:** `SourcePanel.svelte` (「ブラウザソース (OBS)」の節): スイッチ・URL (トークンは伏せる)・コピー・作り直し (確認あり)・
  開いているページの数・ポートは操作 API と共有の案内・OBS での設定の手順 (大きさは出力ウィンドウのプリセット、透過、シャットダウンと更新を切る)。
  ツールバーに「ソース」の印。`ApiView.listening` は操作 API を受け付けているときだけ `true` にした (待ち受けを共有するため)。
- **仮想カメラ:** シーンの光源 (`src/lighting.rs`。`@ugoe/player` の `sanitizeLighting`・`toLightingJson` を写したもの。光源が無ければ既定の組) と、
  本体とモジュールの法線マップをレンダラに渡す。画像の背景は `ugoe_headless::image::cover` (CSS の cover と同じ置き方、バイリニア) で出力に合わせ、
  透過のところを設定の色で埋めて、毎フレーム `Resident::set_backdrop` の画を描画先に写してから描く (GPU の中の写し)。
  `ugoe-headless` に `Resident::set_lighting`・`set_backdrop`・`image::cover`・`over_color` を足し、描画先を `COPY_DST` にした。
  GPU のテスト `the_camera_feed_uses_the_scene_lighting_and_background_image` (光源・法線マップ・背景の画像付きの画が、同じ光源で描いた
  1 つの Puppet の画を背景に重ねたものと一致する)。
- **背景の画像のデコード (外部依存):** 仮想カメラの背景の JPEG と WebP を読むため、`zune-jpeg` (0.5、MIT OR Apache-2.0 OR Zlib) と
  `image-webp` (0.2、MIT OR Apache-2.0) を足した (**利用者の判断 (2026-10-02)**。純 Rust で軽く、`image` クレートより依存が小さい)。
  ルートの `[workspace.dependencies]` に 1 行ずつ、依存するのは `apps/studio` (`ugoe-studio`) だけ。増えたクレートは `zune-jpeg`・`zune-core`・
  `image-webp`・`byteorder-lite`・`quick-error` の 5 つ (`Cargo.lock`)。そのため `Cargo.toml`・`Cargo.lock` を crates: に足した。
  `zune-jpeg` の既定の features (`std` と SIMD の `x86`・`neon`) はそのまま使う。デコードの前に大きさを確かめ、一辺 8192 画素・3355 万画素を
  超えるものは読まない (PNG は IHDR、JPEG はヘッダと `set_max_width/height`、WebP は `dimensions`)。テストは `virtual_camera.rs` の
  `decodes_png_jpeg_and_webp_backgrounds` (JPEG は Chrome で書いた 16x8 の `tests/fixtures/red-blue.jpg`) と `huge_backgrounds_are_refused_before_decoding`。
- **T0911 に合わせた:** `source.html` の `<style>` に、`output.html` と同じ文字の選択とドラッグを止める指定を入れた。
- **確かめたこと (画の一致):** `api/source_check.rs` (ignored・`gpu-tests`) の待ち受けと、ヘッドレスの Chrome (SwiftShader の WebGL2・Playwright。
  スクリプトはスクラッチ領域の `T0808/check.mjs`・`run.sh`) で、見本の `avatar-lit.ugoe` (法線マップと動的物理) に光源・配置・回転・背景の画像を付け、
  頭を振る 150 フレームを配って止めた画を比べた (480×270)。ブラウザソースのページと出力ウィンドウのページ (`output.html` を同じ Origin で開き、
  StudioHost を同じ WebSocket に差し替え) は完全に一致。仮想カメラとは、回転なしで平均の差 0.45・差 16 超 0.3% (縁と背景の拡大の違い)、
  回転 8 度で平均 2.2・3.9% (CSS の回転で縁の標本が違う)。光源を切っても差は同じ。スクリーンショット: スクラッチ領域の `T0808/out/`
  (`source-r0.png`・`vcam-r0.png`・`diff-r8.png` など) と `T0808/shots/` (操作ウィンドウの節。日英・ダークの狭い幅で横のはみ出しなし)。
  Rust のテスト (`api/tests.rs`): スイッチとトークンが別・`404`・転送・ファイルと CSP・Origin と Host の `403`・`405`・API を入れて切っても出し続ける・
  作り直しで古い URL が `404`・どちらも切れば待ち受けない・WebSocket で最初にシーン、刻むとフレーム、シーンの切り替えはフレームより先・負荷の受け取りと
  ラベル・バイナリで閉じる・切ると閉じて負荷の表示から外す・溢れたら外す。
- **確かめていないこと:** 実際の OBS (CEF) での表示 (利用者に残す。手順は docs/studio.md#obs-での設定)。Tauri のアプリの中での動作
  (本物の `AssetResolver`・開発モードの `dist/` 読み)。Windows・macOS。wasm の受け手とネイティブの受け手の **頂点** の比べ合わせ (T0801 の後続の候補) は
  していない (画で比べた)。
- **範囲外の変更:** なし (`bindings/web`・`packages/player` は触っていない)。
- **後続の候補:** wasm とネイティブの受け手の頂点の自動の比べ合わせ (Node で wasm を動かす)。
  ブラウザソースのポートを操作 API のパネルに行かずに変える口。OBS の CEF の版での WebGL2 と CSP (`'wasm-unsafe-eval'`) の確認。

