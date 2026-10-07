---
id: T0489
title: "C ABI: 失敗の中身を呼び出し側の UgoeError で返し、スレッドローカルの ugoe_last_error* を廃止する"
status: done
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

- [x] `UgoeError` の作成・破棄・読み出し (`ugoe_error_create`・`ugoe_error_destroy`・`ugoe_error_message`・`ugoe_error_kind`・`ugoe_error_field`・
      `ugoe_error_field_count`・`ugoe_error_field_key_at`・`ugoe_error_field_value_at`) を足した。読める値は今の `ugoe_last_error*` と同じ
- [x] 失敗しうるすべての関数が `UgoeError*` を取る (NULL なら書かない)。引数の位置と名前をすべての関数で揃え、形を README に書いた。
      成功した呼び出しは渡された `UgoeError` を空にするか、前の中身を残すかも決めて書いた
- [x] パニック (`UGOE_ERROR_PANIC`) のメッセージも `UgoeError` に書く
- [x] `ugoe_last_error*` とスレッドローカルの状態を消した。`crates/ugoe-ffi` にスレッドローカル・グローバルな状態が残っていない
- [x] `ABI_VERSION` を上げた
- [x] 失敗させたのとは別のスレッドで `UgoeError` を読むテストを足した
- [x] リポジトリの中の利用者を移した: C の見本 (`examples/c/sample.c`)、適合テスト (`tests/conformance/c/runner.c`・`src/frame.rs`)、
      Unity (`Native.cs`・`UgoeException` の作り方。例外の種類と補足の値は今と同じ)。`just refs` (ヘッダと Unity の宣言の過不足) が通る
- [x] `ugoe_last_error*` に触れている文書とコメント (docs/portability.md#c-abi-ugoe-ffi-の規約・crates/ugoe-ffi/README.md・docs/spec/modules.md・
      bindings/unity/README.md・bindings/godot/README.md・crates/ugoe-wasm/README.md・crates/ugoe/README.md、`ugoe`・`ugoe-wasm` の doc コメント) を
      新しい名前に直した。docs/portability.md にスレッドについての約束 (ADR 0031) を書いた
- [x] ヘッダ (`just ffi-header`) を作り直し、`just ffi-check` が通る。Unity のテスト (bindings/unity/tests) が通る
- [x] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、crates/ugoe-ffi/src/error.rs、crates/ugoe-ffi/README.md#abi-の版、
  bindings/unity/README.md

## メモ

- 2026-10-06 起票。同日、利用者の判断で「`ugoe_last_error*` を残して足すだけ」から「廃止して置き換える」に改めた。
- 2026-10-07 (claude-worker-T0489) 実装してレビューに出した。
  - 形: `UgoeResult` を返す関数は、すべて **最後の引数** に `UgoeError *error` を取る (NULL なら書かない)。`void` を返す破棄の関数と、
    失敗しない関数 (`ugoe_abi_version`・`ugoe_version`・`*_default`・`ugoe_error_*`) は取らない。成功した呼び出しは中身を **変えない**
    (毎フレームの呼び出しで書き込みをしないため。前の `ugoe_last_error` と同じ振る舞い)。crates/ugoe-ffi/README.md#失敗の受け取り方-t0489 に書いた。
  - 読み出しの関数は NULL の `UgoeError` も受け付ける (NULL か 0)。`ABI_VERSION` は 1 → 2。
  - `crates/ugoe-ffi` に残っていたグローバルな状態 (`ugoe_blend_shape_name` の `OnceLock` で作る名前の表) も、コンパイル時に作る
    変わらない表 (`static` の NUL 終端のバイト列) に替えた。
  - 別のスレッドで失敗させて読むテストは `crates/ugoe-ffi/tests/api.rs` の `error_is_readable_from_another_thread`。
    Unity は並行に失敗させて、それぞれの例外が自分の種類を持つことを `tests/ResultTests.cs` で確かめる。
  - Unity: マネージドなスレッドごとに `UgoeError` を 1 つ作り (`NativeError.Current`、`[ThreadStatic]` と `SafeHandle` のファイナライザで破棄)、
    呼び出しと `UgoeException.Check` を同じスレッドで続けて行う。例外の種類と補足の値は前と同じ。
  - Rust のテスト (`crates/ugoe-ffi/tests/`) は、テストのスレッドごとの `UgoeError` (`tests/common/mod.rs`) を渡す形に移した。
  - 通した検査: `just crate ugoe-ffi`・`just ffi-check`・`just refs`・`just unity-check`・`just conformance` (c・wasm・unity・godot すべて ok)・
    `just godot-check`・`just cross-check`・`just wasm-check`・`just ci`。
  - 確かめていないこと: 実際の Unity (IL2CPP を含む) と、Windows・macOS での実行 (`just cross-check` の clippy まで)。
  - Godot のバインディングは C ABI を使わない (SDK を直接使う) ので、README の名前だけを直した。
  - 起票の候補: なし (T0490 の毎フレームの状態をまとめて受け取る形は、この形 (最後の引数 `error`) に合わせる)。
