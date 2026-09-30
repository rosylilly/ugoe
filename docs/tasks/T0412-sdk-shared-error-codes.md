---
id: T0412
title: SDK: 着せ替えのエラーと読み込みの警告の code を SDK にまとめ、wasm と C ABI で揃える
status: done
milestone: M7
crates: [ugoe, ugoe-wasm, ugoe-ffi, apps/viewer, apps/editor, docs]
depends_on: [T0409, T0410]
assignee: claude-worker-T0412
---

# T0412: SDK: エラーと警告の code の共有

## 目的

T0409 の後続の候補をまとめる。

## 受け入れ条件

- [x] 着せ替えのエラーの種類 (code) と補足の値の対応が `ugoe-wasm` と `ugoe-ffi` に重複している。SDK (`ugoe` の `DressUpError::code` など) に移して 1 か所にする
- [x] 読み込みの警告の code を揃える: C ABI だけが出す `unknown_optional_feature`・`unknown_value` を wasm の `load_warnings_json` にも足す (ビューアとエディタのプレビューの知らせも足す)
- [x] モーションのイベントと物理の設定の差し替えを C ABI に足す
- [x] 既存の code の文字列は変えない (組み込み先が照らし合わせているため)。`just ci`・`just wasm-check`・`just ffi-check` が通る

## 参考

- `docs/tasks/T0409-ffi-extended-api.md`・`docs/tasks/T0125-wasm-load-warnings-structured.md` のメモ

## メモ

- **範囲外の変更 (team-lead 承認済み):** `apps/viewer` (`lib/loadWarnings.ts` とテスト・README) と `apps/editor`
  (`src/lib/preview/loadWarnings.ts` とテスト)。理由: wasm に足した 2 つの警告を、ビューア (日本語) とエディタのプレビュー (英語) でも
  言い換えるため (受け入れ条件)。`docs/editor.md` の 1 か所も直した。
- **SDK:** `DressUpError::code()`・`fields()`・`INVALID_FILE` (`crates/ugoe/src/dress_up.rs`) と、読み込みの警告の `ugoe::LoadWarning`
  (`code()`・`fields()`。補足の値は `FieldValue` (番号か文字列)。`From<&ReadWarning>`。`crates/ugoe/src/warning.rs`)。wasm の
  `LoadWarning` は SDK のものを再エクスポートし、JSON は `load::warning_json` が作る (番号は数のまま)。ffi も同じ型から code と補足の値を作る。
  wasm だけの `no_model`・`not_loaded`・`texture` は wasm の `error_info` に残した。ffi の `texture_missing` の文は、白で代用しないので
  T0409 のまま (`no data for texture N`)。
- **wasm:** `from_bundle` / `from_module_file` が `read_*_with_report` の警告を一覧の先頭に並べる (C ABI と同じ順)。戻り値の文の警告にも入る。
- **C ABI (ABI の版は 1 のまま):** `ugoe_puppet_motion_event_count` / `_event` (`UgoeMotionEvent { time, value }`。次の update まで有効)、
  `ugoe_puppet_set_physics_json` (NULL で外す。読めなければ `UGOE_ERROR_FORMAT`、検査を通らなければ `UGOE_ERROR_INVALID_MODEL`)、
  `ugoe_puppet_physics_json` (ホストのバッファ。物理が無ければ長さ 0)、`ugoe_puppet_reset_physics`。`src/animation.rs`。C の見本でも呼ぶ。
- **テスト:** SDK に code と補足の値の件、ffi に T0409 と同じ警告の出力 (未来の版のバンドルとモジュール)・イベント・物理の差し替え
  (Rust の Puppet と同じ揺れ)、wasm に未来の版の警告が C ABI と同じ形になる件。Vitest はビューアとエディタの loadWarnings。
- **確認:** `just ci`・`just wasm-check`・`just ffi-check`・ビューアとエディタの `vite build`・`cargo check -p ugoe-editor` (main 6b214e2 に rebase 後)。
- **確かめていないこと:** ビューアとエディタの画面 (Vitest だけ)。Windows と macOS での C の見本。
- **後続の候補:** 着せ替えのモジュールを付けている間に差し替えた物理の設定が、次の付け外しで本体の設定から作り直されて失われる
  (wasm と同じ。README に書いた)。差し替えを本体の設定にも反映するかは SDK の `Puppet::set_physics` の決めごととして検討する。
  C ABI からパーツの不透明度を読む API (T0409 の候補のうち残ったもの)。
