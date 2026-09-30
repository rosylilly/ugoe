---
id: T0412
title: SDK: 着せ替えのエラーと読み込みの警告の code を SDK にまとめ、wasm と C ABI で揃える
status: in-progress
milestone: M7
crates: [ugoe, ugoe-wasm, ugoe-ffi, docs]
depends_on: [T0409, T0410]
assignee: claude-worker-T0412
---

# T0412: SDK: エラーと警告の code の共有

## 目的

T0409 の後続の候補をまとめる。

## 受け入れ条件

- [ ] 着せ替えのエラーの種類 (code) と補足の値の対応が `ugoe-wasm` と `ugoe-ffi` に重複している。SDK (`ugoe` の `DressUpError::code` など) に移して 1 か所にする
- [ ] 読み込みの警告の code を揃える: C ABI だけが出す `unknown_optional_feature`・`unknown_value` を wasm の `load_warnings_json` にも足す (ビューアとエディタのプレビューの知らせも足す)
- [ ] モーションのイベントと物理の設定の差し替えを C ABI に足す
- [ ] 既存の code の文字列は変えない (組み込み先が照らし合わせているため)。`just ci`・`just wasm-check`・`just ffi-check` が通る

## 参考

- `docs/tasks/T0409-ffi-extended-api.md`・`docs/tasks/T0125-wasm-load-warnings-structured.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
