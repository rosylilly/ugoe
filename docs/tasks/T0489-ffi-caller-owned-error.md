---
id: T0489
title: "C ABI: 失敗の中身を呼び出し側の UgoeError で返し、スレッドローカルの ugoe_last_error* を廃止する"
status: in-progress
milestone: M11
crates: [ugoe-ffi, bindings/unity, tests/conformance, ugoe, ugoe-wasm, bindings/godot, docs]
depends_on: []
assignee: claude-worker-T0489
---

# T0489: C ABI: 失敗の中身を呼び出し側の UgoeError で返し、スレッドローカルの ugoe_last_error* を廃止する

## 目的

C ABI のエラー (`ugoe_last_error*`) はスレッドローカルにあるので、失敗した呼び出しとエラーを読む呼び出しが別の OS のスレッドで動くホスト
(Go の goroutine・C# の async の続き・スレッドプール) では、違うスレッドの値を読む。また `std` の無い環境ではスレッドローカルを使えない。
失敗の中身は呼び出し側が渡す `UgoeError` に書く形だけにし、`ugoe_last_error*` は廃止する
([ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md#c-abi-のエラーはスレッドローカルに置かない))。
1.0 より前で、C ABI の利用者はリポジトリの中だけなので、ABI を壊して同じ PR で移す。

## 受け入れ条件

- [ ] `UgoeError` の作成・破棄・読み出し (`ugoe_error_create`・`ugoe_error_destroy`・`ugoe_error_message`・`ugoe_error_kind`・`ugoe_error_field`・
      `ugoe_error_field_count`・`ugoe_error_field_key_at`・`ugoe_error_field_value_at`) を足した。読める値は今の `ugoe_last_error*` と同じ
- [ ] 失敗しうるすべての関数が `UgoeError*` を取る (NULL なら書かない)。引数の位置と名前をすべての関数で揃え、形を README に書いた。
      成功した呼び出しは渡された `UgoeError` を空にするか、前の中身を残すかも決めて書いた
- [ ] パニック (`UGOE_ERROR_PANIC`) のメッセージも `UgoeError` に書く
- [ ] `ugoe_last_error*` とスレッドローカルの状態を消した。`crates/ugoe-ffi` にスレッドローカル・グローバルな状態が残っていない
- [ ] `ABI_VERSION` を上げた
- [ ] 失敗させたのとは別のスレッドで `UgoeError` を読むテストを足した
- [ ] リポジトリの中の利用者を移した: C の見本 (`examples/c/sample.c`)、適合テスト (`tests/conformance/c/runner.c`・`src/frame.rs`)、
      Unity (`Native.cs`・`UgoeException` の作り方。例外の種類と補足の値は今と同じ)。`just refs` (ヘッダと Unity の宣言の過不足) が通る
- [ ] `ugoe_last_error*` に触れている文書とコメント (docs/portability.md#c-abi-ugoe-ffi-の規約・crates/ugoe-ffi/README.md・docs/spec/modules.md・
      bindings/unity/README.md・bindings/godot/README.md・crates/ugoe-wasm/README.md・crates/ugoe/README.md、`ugoe`・`ugoe-wasm` の doc コメント) を
      新しい名前に直した。docs/portability.md にスレッドについての約束 (ADR 0031) を書いた
- [ ] ヘッダ (`just ffi-header`) を作り直し、`just ffi-check` が通る。Unity のテスト (bindings/unity/tests) が通る
- [ ] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、crates/ugoe-ffi/src/error.rs、crates/ugoe-ffi/README.md#abi-の版、
  bindings/unity/README.md

## メモ

- 2026-10-06 起票。同日、利用者の判断で「`ugoe_last_error*` を残して足すだけ」から「廃止して置き換える」に改めた。
