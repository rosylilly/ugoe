---
id: T0636
title: エディタ: プレビューでのフェイストラッキング
status: done
milestone: M4
crates: [apps/editor, apps/viewer, packages, docs/manual, docs/editor.md, pnpm-lock.yaml, justfile, lefthook.yml, .github/workflows/ci.yml, .gitignore]
depends_on: [T0635, T0703]
assignee: claude-worker-T0636
---

# T0636: エディタ: プレビューでのフェイストラッキング

## 目的

エディタのプレビューモード (T0635) で、ビューアと同じ MediaPipe のフェイストラッキング (T0703) を使い、組み上げたモデルに表情が付くかを編集の場で確かめられるようにする。
T0635 では、ビューアのトラッキング (約 1300 行)・MediaPipe の同梱 (約 9 MiB)・モデルの取得と検証・カメラの権限をエディタへ持ち込む大きさから、切り出した。

## 受け入れ条件

- [x] トラッキングの部品 (MediaPipe の呼び出し・頭の向き・平滑化・キャリブレーション・記録と再生・パネル) を、ビューアとエディタで共有する場所 (例: `packages/`、GPL-3.0-only) に切り出すか、写すかを決めて書く
- [x] エディタのプレビューでトラッキングを開始・停止でき、T0635 のコントローラの差し込み口でパラメータを書く。トラッキング中の目パチの扱いはビューアと同じ
- [x] MediaPipe の wasm とモデルの同梱 (ビルド時の取得と SHA-256 の検証)、カメラの権限 (Linux・Windows・macOS)、ライセンスの表記
- [x] ヘッドレスのブラウザで、記録した値の列か画像・動画の入力で確かめる。操作マニュアルを更新する
- [x] `just ci`・`just licenses`・エディタとビューアのビルドが通る

## 参考

- `docs/tasks/T0635-editor-preview-mode.md`、`docs/tasks/T0703-viewer-face-tracking.md` のメモ

## メモ

### 決めたこと: 共有のパッケージに切り出す

- 写さずに **`packages/face-tracking` (`@ugoe/face-tracking`、GPL-3.0-only、pnpm のワークスペース) に切り出した。**
  値の約束・既定値 (平滑化・目パチの停止)・WebKitGTK の回避策などを 2 か所で保つと食い違うため。ビューアとエディタは
  `"@ugoe/face-tracking": "workspace:*"` で依存する。中身は frame / calibration / filter / recording (Vitest 21 件、ビューアから
  そのまま移した)、`FaceTracker` (Player に直接つながず、`TrackingTarget` = `setTracking` / `setAutoEyeBlink` と
  保存の関数 `saveJson` を受け取る)、`TrackingPanel.svelte` (文言は `strings.ts` の `ja` / `en`、値は props)、
  `scripts/mediapipe-assets.mjs` (bin `ugoe-mediapipe-assets <出力先> [--optional]`)、`third-party-notices.txt`
  (スクリプトが出力先に写す)。
- パネルは `index.ts` から export する (`@ugoe/face-tracking/TrackingPanel.svelte` のサブパスで import すると、svelte-check が
  node_modules の経路と実体の経路で同じ `FaceTracker` を別の型として扱い、エラーになった)。
- ビューアは import 先・`TrackingPanel` の props・package.json のスクリプトだけを替えた (文言は `ja`、保存は `host.ts`)。
  パネルの `open` を `bind:open` にした (単方向だと値の表示が変わるたびに属性が書き戻され、利用者が閉じたパネルが開き直る。
  ビューアでも起きていたはずの不具合で、直したことでだけ振る舞いが変わる)。

### エディタ

- `PreviewMode` (`apps/editor/src/lib/preview/preview.svelte.ts`) が `TrackingTarget` になり、`FaceTracker` (英語の文言) を持つ。
  値は `FrameDriver.beforeUpdate` で `set_tracking` に渡す (1 フレーム 1 回。`loaded` で `tracking_parameter_ids` を読み、最後の値を
  渡し直す)。`null` で `clear_tracking` と対象のパラメータの既定値への書き戻し。目パチはビューアと同じく既定で止め、段の
  「Eye blink」とトラッキングの指定のどちらも有効なときだけ動かす。プレビューを出るとトラッキングを止める。テスト
  `tracking.test.ts` (偽の WebPuppet で 6 件)。
- パネルは「Face tracking」(最初は閉じる) と「Third-party licenses」(`mediapipe/third-party-notices.txt` を表示する
  `LicenseDialog.svelte`)。エディタの `label` が grid なので、チェックボックスの行だけ PreviewPanel で横並びに直した。
- 記録の保存: `trackingHost.ts` が OS の保存のダイアログ (`fileDialogs`) → `ipc.writeJsonFile` (拡張子が無ければ `.json` を付ける)。
- Tauri: `src-tauri/src/tracking.rs` (`write_json_file` は `.json` だけ、`permission_response` はカメラだけ Allow。テスト 2 件)、
  `on_permission_request`、macOS の `Info.plist` (`NSCameraUsageDescription`) と `Entitlements.plist`
  (`com.apple.security.device.camera`、`tauri.conf.json` の `bundle.macOS.entitlements`)。ビューアと同じ設定。
- MediaPipe: `pnpm dev` (`--optional`)・`pnpm build`・`pnpm mediapipe` の前に `ugoe-mediapipe-assets public/mediapipe`。
  モデルは版を固定した URL から取って SHA-256 を検証、`public/mediapipe/` は gitignore。
- 範囲外の変更 (team-lead の承認済み。`crates:` に足した): `pnpm-lock.yaml` (`pnpm install` の結果)、`justfile` の web-check に
  パッケージの check / test の 2 行、`.github/workflows/ci.yml` の web に同じ 2 行、`lefthook.yml` の pre-push の check、
  `.gitignore` に `/apps/editor/public/mediapipe/`。`docs/editor.md` のプレビューモードの節も更新した。
- main への追従で T0315 の `check_physics` と `lib.rs`・`ipc.ts` が衝突し、両方を残した。

### 確かめたこと

`ugoe sample avatar --project` のプロジェクト (目パチの対象 `EyeLOpen`・`EyeROpen`、間隔 2 秒を `model.json` に手で足した) を、
ビルドした画面 (`just wasm`・`vite build`) と本物の `Document` への HTTP ブリッジ (リポジトリ外) でヘッドレス Chrome
(SwiftShader・Playwright) で開いた。素材はリポジトリ外: 合成した記録の JSON (2 秒、ヨー ±20°・ロール 5°・`jawOpen` 0.6・
0.8〜1.2 秒に目を閉じる) と MediaPipe の試験用の `portrait.jpg`。

- プレビューに入ると目パチで EyeLOpen が 0〜1。「File…」で記録を入れると AngleX −19.8〜19.8、AngleZ 5、MouthOpenY 0.6、
  EyeLOpen が記録に合わせて 0〜0.95 (目パチは止まる)、状態は「1.2 / 2.0 s」。
- 「Stop」で AngleX 0・AngleZ 0・EyeLOpen 1・MouthOpenY 0 (既定値) に戻り、目パチが戻る (0〜1)。
- 画像: 検出 (9 fps) でヨー 1.8°・ピッチ −4.2°・ロール −0.5°、AngleX 1.807・EyeLOpen 0.772・MouthOpenY 0.09。2.5 秒の間
  EyeLOpen 0.772〜0.773 (目パチは止まっている)。映像とランドマークが出る。
- 「Record」→「Stop recording」→「Save (19)」: 保存のダイアログ (差し替え) の拡張子の無いパスに `.json` が付き、
  `write_json_file` で 19 フレームの JSON が書かれた。
- カメラ: Chrome の偽のカメラで「Start」→ カメラの一覧 (`fake_device_0`)・映像・「No face found」(顔が無い)。
- 「Third-party licenses」で MediaPipe の表記 (Apache-2.0) が出る。「Exit preview」でトラッカーが止まる (`idle`)。
- ビューア (ビルドした画面で `?model=samples/avatar.ugoe`): 文言は日本語のまま、記録の再生で AngleX が動き MouthOpenY 0.6、
  「停止」で既定値に戻る、画像でエディタと同じ値 (AngleX 1.78・EyeLOpen 0.77)。閉じたパネルは値が変わっても閉じたまま。
- `just ci` が通る。ビューアとエディタの `pnpm build` (MediaPipe の取得と検証を含む)・`cargo check -p ugoe-editor` が通る。
- マニュアルの画像 `docs/manual/images/preview-tracking.png` は記録の再生の画面 (人の顔は入れていない)。

### 利用者に確かめてほしいこと

- 実際の Tauri のエディタで、実際のカメラ: Windows (WebView2。「設定」のデスクトップアプリのカメラの許可)・macOS
  (初回の OS の許可、署名したビルド)・Linux (WebKitGTK) で「Start」→ 映像 → モデルに表情が付く。WSL にはカメラが無いため、
  エディタの Tauri のウィンドウは確かめていない (ビューアの T0703 では同じ設定で `getUserMedia` まで届いた)。
- マニュアルに書いた Windows・macOS の設定の場所。

### 後続の候補

- `AGENTS.md` と `docs/architecture.md` のリポジトリ構成に `packages/` (アプリ間で共有する GPL の TS パッケージ) を書く (範囲外なので触っていない)。
- ビューアの `public/third-party-notices.txt` を、パッケージの表記 (`mediapipe/third-party-notices.txt`) に寄せる (今は 2 か所に同じ Apache-2.0 の全文がある)。
- プレビューで付いた表情を、表情やキーとして取り込む操作。リップシンクの音量をマイクから与える。
