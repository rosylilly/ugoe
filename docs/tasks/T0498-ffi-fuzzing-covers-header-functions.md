---
id: T0498
title: "ugoe-ffi: C ABI のヘッダの関数と fuzzing.rs が呼ぶ関数の一覧を比べ、fuzz の入れ忘れを検査で止める"
status: done
milestone: M11
crates: [ugoe-ffi, scripts, docs]
depends_on: []
assignee: claude-worker-T0498
---

# T0498: ugoe-ffi: C ABI のヘッダの関数と fuzzing.rs が呼ぶ関数の一覧を比べ、fuzz の入れ忘れを検査で止める

## 目的

T0497 で `ugoe_ffi::fuzzing::run` が C ABI の関数を一通り呼ぶ fuzz の入口を足したが、C ABI に関数を足したときに fuzzing.rs に入れ忘れても
気づけない (T0497 の候補)。

## 受け入れ条件

- [x] ヘッダ (`crates/ugoe-ffi` の生成されたヘッダ) の関数と、fuzzing.rs が呼ぶ関数を比べ、呼んでいない関数があれば名前を出して失敗する検査を足した (`just refs` の ffi の検査 (`scripts/check-ffi-bindings.mjs`) に足すのが素直)。呼ばなくてよい関数 (`ugoe_abi_version` など) は理由付きの除外の一覧に置く
- [x] 一時的に関数を 1 つ外して、検査が見つけることを確かめた
- [x] crates/ugoe-ffi/README.md (関数を足すときの手順) に書いた
- [x] `just refs`・`just ffi-check`・`just ci` が通る

## 参考

- T0497 のメモ、crates/ugoe-ffi/src/fuzzing.rs、scripts/check-ffi-bindings.mjs

## メモ

- T0497 のワーカーの報告から起票した (2026-10-07)。
- やったこと (claude-worker-T0498、2026-10-07):
  - `scripts/check-ffi-bindings.mjs` の呼ぶ側の一覧 (`BINDINGS`) に `fuzz` (`crates/ugoe-ffi/src/fuzzing.rs`) を足した。Rust のコードから
    コメントと文字列を除き、`ugoe_*` の識別子 (呼び出しと、`self.count(.., ugoe_puppet_part_count, ..)` のような関数のポインタ) を集める
    (`rustFunctions`)。`self.check("ugoe_x", code)` の文字列は呼んだことにしない。呼ぶ側ごとに `parse` と誤りの文の言い方 (`verb`) を持たせた。
  - 入れた時点で漏れていたのは 7 つ。`ugoe_error_field` (`check` で、あるキー・無いキー・空・NULL で引く) と
    `ugoe_module_create_with_limits` (`create_module` で小さな上限で読み、読めたら破棄してふつうに読み直す) を fuzzing.rs に足した。
    入力を取らない `ugoe_abi_version`・`ugoe_version`・`ugoe_read_limits_default`・`ugoe_image_limits_default` と、プロセスに 1 つの
    登録を変えるだけの `ugoe_set_panic_handler` は `omit` に理由付きで置いた。
  - 一時的に `ugoe_error_field` の呼び出しを外し、`error: fuzz: ヘッダの ugoe_error_field が crates/ugoe-ffi/src/fuzzing.rs で呼ばれていない`
    で失敗することを確かめた (戻した)。`scripts/check-refs.test.mjs` に `rustFunctions` と fuzz の漏れのテストを足した。
  - crates/ugoe-ffi/README.md・docs/testing.md (参照の検査の表と fuzz の節)・justfile の `refs` のコメントに書いた。範囲の外の小さな変更として、
    AGENTS.md のコマンドの表の `just refs` の説明も直した。
  - `just refs`・`just ffi-check`・`just fuzz-lint`・`just test-group bindings`・`just lint-group bindings`・`just ci` が通った。
- やらなかったこと: 検査は関数を呼ぶ (ポインタとして渡す) かどうかだけを見る。台本のどの操作で届くか、実際の fuzz で届いたかは見ない。
  nightly での `just fuzz ffi` は回していない (fuzzing.rs の変更は stable の結合テスト `tests/fuzzing.rs` で回った)。
- 起票の候補: なし。
