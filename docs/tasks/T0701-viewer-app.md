---
id: T0701
title: ビューアアプリの土台 (apps/viewer)
status: done
milestone: M3
crates: [apps/viewer, ugoe-wasm, Cargo.toml, pnpm-workspace, justfile, scripts, commitlint.config.mjs, .github/workflows, docs]
depends_on: []
assignee: claude-worker-T0701
---

# T0701: ビューアアプリの土台 (apps/viewer)

## 目的

`.ugoe` を開いて表示し、パラメータを動かして確かめるだけの軽い Tauri v2 アプリ `apps/viewer` を作る。
エディタ (編集) とは別に、ランタイムを組み込むアプリの見本と、フェイストラッキング (T0703) の確認の場にする。

## 方針

- **ライセンスは GPL-3.0-only** (当初は MIT の予定だったが、作業中にユーザーの判断でエディタと同じにした。ADR 0019)。エディタのコードは写してよいが、アプリ同士で import はしない。
- 描画は `ugoe-wasm` の `WebPuppet` を webview で使う (エディタのビューポートと同じ仕組み、ADR 0003)。Rust 側 (Tauri) はファイルの読み込みなど最小限にし、依存はランタイム層だけにする (`scripts/check-layers.mjs` に `ugoe-viewer` を登録)。
- フロントエンドは Svelte 5 + Vite (エディタと同じ道具)。

## 受け入れ条件

- [x] `apps/viewer` (Tauri v2、crate 名 `ugoe-viewer`、npm パッケージ `@ugoe/viewer`、`license = "GPL-3.0-only"`。当初の条件は MIT) がワークスペースに入り、`just viewer` (開発) と `just viewer-build` がある
- [x] `.ugoe` (と `model.json`) を開く (OS のダイアログ、ドラッグ・アンド・ドロップ)。表示、Fit・ズーム・パン、背景色
- [x] パラメータのスライダーの一覧 (グループごと)、既定値に戻す
- [x] ヘッドレスのブラウザで確かめられるよう、Tauri の呼び出しを差し替えられる
- [x] `just ci` (Biome・svelte-check・Vitest を含む)・`just licenses`・`just layers` が通る。CI のエディタのビルドのワークフローにビューアも入れるか決めて書く
- [x] `apps/viewer/README.md` と `docs/architecture.md`・`AGENTS.md` の構成の説明を更新する。commitlint の scope に `viewer` を足す

## 参考

- `apps/editor/` (構成の参考。コードは写さない)
- `docs/adr/0003-editor-viewport-in-webview.md`、`docs/adr/0018-licensing-and-1.0-scope.md`

## メモ

### やったこと

- `apps/viewer` を追加した (Tauri v2 + Svelte 5 + Vite。crate `ugoe-viewer`、npm `@ugoe/viewer`、GPL-3.0-only。`apps/viewer/LICENSE` はエディタと同じ)。エディタのコードは写していない (最初は MIT の予定だったため。いまは写してよい)。
  - Rust 側はファイルを読むこと (`read_model_file`。拡張子が `.ugoe`/`.json` のものだけ、512 MiB まで) と、起動時の引数のパスを返すこと (`initial_file`) だけ。内部のクレートには依存していない。`check-layers.mjs` にはランタイム層まで依存してよいクレートとして登録した。
  - 描画とパラメータの操作は webview の `WebPuppet`。WebGPU で作れなければ新しい canvas で WebGL2。GPU を失ったら作り直して読み直す。
  - 開く: OS のダイアログ (Ctrl+O)、HTML のドラッグ・アンド・ドロップ (ウィンドウは `dragDropEnabled: false`)、起動時の引数。形式は中身 (`UGOE` か `{`) で判別する。
  - 表示: Fit (ボタン・F・ダブルクリック)、ホイールでのズーム (カーソル中心)、ドラッグでのパン、背景色と透明 (市松模様)。
  - パラメータ: グループごとの折りたたみ、絞り込み、1 つずつ・すべて既定値に戻す。
  - 外から動かす: `ParameterDriver` (`src/lib/drivers.ts`)。毎フレーム値を返すと設定され、そのスライダーは「(自動)」になって手で動かせなくなる。T0703 のトラッキングはこれで入れる。右のパネルは縦にパネルを積む作りで、トラッキングのパネルはパラメータの上に足す。
  - 差し替え: `window.__UGOE_VIEWER_HOST__` (`ViewerHost`)。Tauri の外ではファイルの選択が `<input type="file">` になり、`?model=<URL>` で起動時に開ける。表示中の `Player` は `window.__UGOE_VIEWER__`。
- **`ugoe-wasm` の変更 (オーケストレーターの承認済みで `crates:` に足した):** スライダーの値域・既定値・グループを得る手段が `WebPuppet` に無かったので、`ugoe-wasm` に `WebPuppet::parameters_json()` を足した (値域・既定値・グループ。これまで ID しか取れなかった)。`serde`・`serde_json` を直接の依存にした (もともと推移的に入っていた)。ネイティブのテストを足し、`just wasm-check` も通した。エディタの `ugoe-wasm-stub.d.ts` にも 1 行足した (stub を web.rs と揃えるという約束のため)。T0308 も `web.rs` を触るので、マージで衝突するかもしれない (`parameter_ids` の直後に足した)。
- **ライセンス:** 最初は MIT で作ったが、Tauri 自体が MPL-2.0 のクレート (ビルド時だけの `cssparser`・`selectors`・`dtoa-short`・`cssparser-macros` と、バイナリに入る `option-ext`) に依存するので、MIT のクレートとしては `just licenses` を通らなかった。オーケストレーター経由でユーザーに確かめ、ビューアを **GPL-3.0-only** にすることになった。`check-licenses.mjs` の `EDITOR_CRATES` に `ugoe-viewer` を入れた (エディタと同じ規則)。ADR 0019 (Accepted) に記録した。
- CI: `ci.yml` の web のジョブでビューアの Vitest・svelte-check・vite build も行う。**Editor build のワークフローにはビューアも入れた** (アプリ × OS の行列。成果物は `ugoe-viewer-<OS>`)。ビューアは小さく、同じ手順で作れて、タグで配布するときに一緒にあると便利なため。手動実行とタグでしか動かないので、実行時間が倍になっても普段の CI には影響しない。
- `just viewer` / `just viewer-build`、`just web-check` にビューア、commitlint の scope `viewer`、`.gitignore` (wasm の生成物、`gen/schemas`) を足した。アイコンは自作 (`icon.png` から `pnpm tauri icon`)。

### 確かめたこと

- `just ci`・`just licenses`・`just layers`・`just wasm-check`・`cargo check -p ugoe-viewer` が通る。
- `just viewer-build --bundles deb` が通り (約 4 MiB の .deb)、WSLg (WebKitGTK、WebGL2) で `ugoe-viewer c.ugoe` として起動すると、character.psd から作った `.ugoe` がテクスチャ付きで表示された (`initial_file` → `read_model_file` → `load_bundle` の経路)。
- ヘッドレスの Chrome (Playwright、SwiftShader の WebGL2) で、ビルドした `dist/` に対して次を確かめた (スクリプトはリポジトリの外):
  - `?model=/c.ugoe` で開く → `ready`、バックエンドは `webgl2`、警告なし。スクリーンショット: 左に 320x400 のキャラクター (髪・顔・目・頬・口・服) が薄い青のキャンバスごとフィットして表示され、右に「パラメータ」のパネル (このモデルにはパラメータがない)、下のステータスバーに「読み込みました」と `webgl2`。
  - 差し替えた `ViewerHost` で「開く」→ `pose_and_effects` のフィクスチャにグループを足した JSON。区切りは「顔 3 / 目 6 / 体 4 / その他 9」の順で、スライダーが既定値で並ぶ (EyeLOpen・EyeROpen は 1)。
  - `DataTransfer` でのドロップ → `rotation_chain` (グループ付き)。Angle のスライダーを 20 にすると値が 20 になり、長方形が回転して描かれる。背景色の変更 (#203040)・ホイールでのズーム (157%)・Fit・「すべて戻す」も動いた。
  - ドライバで Flip を 1 にし続けると、Flip は「(自動)」になりスライダーが無効になる。「すべて戻す」の後も 1 のまま (ドライバが勝つ)。

### やらなかったこと・後続の候補

- エディタとビューアで共有したい部品 (wasm の読み込み、ビューポートの操作) はまだ無い。両方とも GPL になったので写してよい。増えてきたら共有の場所 (`packages/` など) に切り出すタスクを起票する。
- 物理・モーション・表情の再生の UI (バンドルの物理は `update(dt)` で毎フレーム動く)。自動の目パチ・呼吸の切り替え。
- ヒットエリアの表示 (T0308 の後)。
- `lefthook.yml` の pre-push の web-check はエディタだけ (範囲外なので触っていない)。`just ci` と CI ではビューアも検査される。
- Windows・macOS でのビルドと動作は確かめていない (Editor build のワークフローで作れるはず)。
