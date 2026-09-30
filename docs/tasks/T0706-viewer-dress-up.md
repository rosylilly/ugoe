---
id: T0706
title: ビューア: 着せ替えのモジュールを付け外しする
status: done
milestone: M4
crates: [apps/viewer, docs]
depends_on: [T0123]
assignee: claude-worker-T0706
---

# T0706: ビューア: 着せ替えのモジュールを付け外しする

## 目的

ADR 0024。ビューアで `.ugoe` を開いた後に `.ugoemod` を読み込み、付け外しできるようにする。

## 受け入れ条件

- [x] `.ugoemod` を開く (ファイルの選択とドラッグアンドドロップ)。付けているモジュールの一覧と、外す・差し替える操作。スロットごとの表示
- [x] 合成のエラーを分かりやすく出す。トラッキングと一緒に動く
- [x] ヘッドレスのブラウザで確かめる。ビューアの README とマニュアルを更新する
- [x] `just ci`・ビューアのビルドが通る

## 参考

- `docs/adr/0024-module-authoring.md`

## メモ

### やったこと

- `modelFile.ts`: `.ugoe` と `.ugoemod` はコンテナが同じなので、チャンク表のタグ (`MODL` / `MODU`) で判別する (`detectModelKind` に
  `"module"`)。表が読めなければ拡張子で決める。
- `dressUp.ts`: `modules_json`・`slots_json` の読み取り、スロットごとのまとめ (`groupBySlot`。本体のスロットを定義の順に並べ、本体に無い
  スロットのモジュールは `missing` として後ろに)、`DressUpError` の `code` ごとの日本語の文 (`describeDressUpError`。合成・コンパイル・
  物理の検証などは wasm の英語の文を括弧で添える。知らないコードは wasm の文)、開き直しで外れたモジュールの警告の日本語化
  (`describeLoadWarning`。wasm の `module `X` was detached: ...` の文を読む)。
- `Player`: `modules`・`slots`・`dressUpError`、`loadModule` (読み込んで、モデルがあれば付ける)・`attachModule`・`detachModule`・
  `unloadModule`・`setSlotModule`。`load` は `.ugoemod` なら `loadModule` に回す。付け外しのたびに一覧と `uses_lighting` を読み直す。
  GPU を作り直したときはモジュールのファイルを読み直して、付けていたもの (合成の順) を付け直す。描画の準備中に開いたモジュールは後で付ける。
- `DressUpPanel.svelte` (畳める `<details>`、トラッキングとライティングの間): 「モジュールを開く…」(複数)、スロットごとに付けているもの・
  付ける / 差し替える / 外す / ×(一覧から除く)、モデルに無いスロットの印、エラーの表示。スロットのあるモデルか、モジュールを読み込んだとき
  (またはエラーがあるとき) だけ出す。
- `App.svelte`: 複数のファイルのドロップ (モデルを先に開き、モジュールを後で読み込む)。「開く」は `.ugoemod` も選べる。
- `host.ts`: `ViewerHost.pickModuleFiles` (Tauri は複数選択のダイアログ、ブラウザは `<input multiple>`)。
- バックエンド: `read_model_file` が `.ugoemod` も読む (`is_readable_path`)。起動時の引数 (`initial_file`) はモデルだけのまま。
- Vitest: `dressUp.test.ts` (10 件)・`modelFile.test.ts` に判別の 2 件。バックエンドに `reads_modules_too`。
- マニュアル `docs/manual/dress-up.md` の「ビューアで付け外しする」(手順・ボタン・付けられないときのメッセージの表。T0647 が作った「準備中」を置き換えた)。
- README (構成・仕組み・使い方の表・「着せ替え (T0706)」の節)、`docs/spec/modules.md#puppet-での着せ替え` にビューアの 1 文。

### 確かめたこと

- `just ci`・`just viewer-build --bundles deb` が通る (main に rebase した後も `just ci`)。
- ヘッドレスの Chrome (SwiftShader の WebGL2) でビルドした `dist/` を開いて確かめた。本体とモジュールは `ugoe_model::fixtures` の
  `dress_up_base`・`long_hair_module`・`dress_module` を 1x1 の単色の PNG のテクスチャで書き出したもの (リポジトリの外、scratchpad の
  小さな Rust のプログラム。コミットしていない)。加えて、`LongHair` を名前だけ変えた `ShortHair` (同じ `Hair`)、スロットが `Hat` の `Hat`、
  `requires.parameters` に `Tail` を足した `Cape` (`Outfit`)、`Outfit` のスロットを除いた本体 `hair-only.ugoe`。
  - 2 つのモジュールのドロップで両方付く (髪と服の色の画素が出る)。「モジュールを開く…」で `ShortHair` を読むと `LongHair` と差し替わり、
    「差し替える」で戻る。「外す」で服の色が消える。
  - `Hat`: 「「Hat」を付けるスロット「Hat」が、このモデルにありません」、一覧では「モデルに無い」の印で付けられない。
    `Cape`: 「「Cape」に必要なパラメータ「Tail」が、このモデルにありません」。壊れたファイル: 「着せ替えのモジュール (.ugoemod) として読めません (unexpected end of input)」。
  - トラッキング (`setTracking` に頭のヨー 20°) を渡したまま付け外し・差し替えしても `AngleX` は 20 のまま、新しいフレーム (−15°) も効く。
  - 開き直し: `hair-only.ugoe` を開くと `Dress` が外れ、ステータスバーに「このモデルに合わないため、着せ替えの「Dress」を外しました (…)」。
    髪は付いたまま。スロットの無い見本のモデルでは付けていたものが外れ、パネルに「このモデルにはスロットがありません」。
  - × で一覧から除く。モデルとモジュールを一緒にドロップするとモデルを先に開いて付ける。

### やらなかったこと・後続の候補

- Tauri (WebKitGTK) のウィンドウでは動かしていない (`.deb` は作れた)。GPU を失ったときの付け直しは、コードを読んで確かめただけ。
- 見本の着せ替えのモデルとモジュールが無い。ビューアの「サンプル」ですぐ試せるよう、`assets/samples/` に小さな本体と衣装があるとよい
  (T0707 の法線マップ付きの見本と同じ形で)。
- 付けているモジュールの組をモデルごとに覚えること、合成の順を並べ替える操作、モジュールのパラメータ (`BodySway` など) を
  スロットの見出しの近くに出すこと。
- 開き直しの警告は wasm の英語の文を正規表現で読んでいる。wasm が外したモジュールの名前と `code` を構造で返すと、文に頼らずに済む。
