---
id: T0489
title: "C ABI: 失敗の中身を、呼び出し側が渡すオブジェクト (UgoeError) で返す"
status: todo
milestone: M11
crates: [ugoe-ffi, tests/conformance, docs]
depends_on: []
assignee:
---

# T0489: C ABI: 失敗の中身を、呼び出し側が渡すオブジェクト (UgoeError) で返す

## 目的

C ABI のエラー (`ugoe_last_error*`) はスレッドローカルにあるので、失敗した呼び出しとエラーを読む呼び出しが別の OS のスレッドで動くホスト
(Go の goroutine・C# の async の続き・スレッドプール) では、違うスレッドの値を読む。また `std` の無い環境ではスレッドローカルを使えない。
呼び出し側が持つ `UgoeError` に失敗の中身を書く関数を足す ([ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md#c-abi-のエラーはスレッドローカルに置かない))。

## 受け入れ条件

- [ ] 形を決めて README に書いた。候補: (a) 失敗しうる関数すべてに `UgoeError*` を最後の引数に取る別名を足す、
      (b) 失敗しうる関数が `UgoeError*` を取る新しい関数の組 (`*_e` など) を足す、(c) 他。関数の数・cbindgen での生成・Unity などの写しやすさで選ぶ
- [ ] `UgoeError` の作成・破棄・読み出し (`ugoe_error_message`・`ugoe_error_kind`・`ugoe_error_field`・`ugoe_error_field_count`・
      `ugoe_error_field_key_at`・`ugoe_error_field_value_at`) を足した。読める値は今の `ugoe_last_error*` と同じ
- [ ] 失敗とエラーの読み出しを別のスレッドで行うテストを足した (新しい形では正しく読め、`ugoe_last_error` は今どおりスレッドごと)
- [ ] 今の `ugoe_last_error*` は残し、同じ値を記録する。足しただけなので ABI の版は上げない
- [ ] ヘッダ (`just ffi-header`)・C の見本・docs/portability.md#c-abi-ugoe-ffi-の規約 を更新した。`just ffi-check`・`just refs` が通る
- [ ] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、crates/ugoe-ffi/src/error.rs、crates/ugoe-ffi/README.md

## メモ

- 2026-10-06 起票。後続の候補: Unity の `Ugoe.Puppet` を新しい形に移す (`UgoeException` の作り方。async から呼ぶ利用者のため)。
