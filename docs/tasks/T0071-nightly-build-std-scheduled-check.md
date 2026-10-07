---
id: T0071
title: "CI (任意): nightly の -Zbuild-std で tier 3 のターゲットの std の無い静的ライブラリを作る検査を、定期のジョブで回す"
status: in-progress
milestone: M11
crates: [scripts, .github/workflows, docs, crates/ugoe-ffi/README.md]
depends_on: [T0070]
assignee: claude-worker-T0071
---

# T0071: CI (任意): nightly の -Zbuild-std で tier 3 のターゲットの std の無い静的ライブラリを作る検査を、定期のジョブで回す

## 目的

T0495 で、core と alloc が配られない tier 3 のターゲット (`aarch64-nintendo-switch-freestanding`・`armv6k-nintendo-3ds` など) で、
nightly の `-Zbuild-std=core,alloc -Zbuild-std-features=compiler-builtins-mem` を使って `ugoe-ffi` の std の無い静的ライブラリを作り、
未解決のシンボルが `just nostd-check` と同じ 6 つ (`abort`・`aligned_alloc`・`calloc`・`free`・`malloc`・`realloc`) だけになることを手元で確かめた。
nightly が要るので `just nostd-check` には入れていない。この手順が nightly や依存の変更で壊れたことに気づけるよう、PR を止めない定期の
CI ジョブ (fuzz.yml と同じく週に 1 度・手動) で回す。急がない (任意)。

## 受け入れ条件

- [ ] 64 ビットのターゲット (`aarch64-nintendo-switch-freestanding`) と、64 ビットのアトミック操作の無い 32 ビットのターゲット
      (`armv6k-nintendo-3ds` など) の 2 つ以上で、`ugoe-ffi` の静的ライブラリを `-Zbuild-std` で作り、未解決のシンボルが決めた一覧と同じかを
      確かめる入口を作った (scripts/nostd-check.mjs の部分コマンドか、`just` のレシピ。`llvm-nm` などの道具の入れ方も)
- [ ] 定期 (と手動) で流す GitHub Actions のジョブを足した (nightly が壊れても PR の CI は止めない。`rust-src` の入れ方も)。
      新しいワークフローにするか、ほかの定期のジョブ (fuzz.yml) に入れるかを決め、理由を書いた
- [ ] 手元で入口を流した結果と、push した後の CI で確かめることをメモに書いた
- [ ] crates/ugoe-ffi/README.md (core と alloc が配られないターゲット) と docs/testing.md に、検査があることを書いた
- [ ] `just ci` が通る

## 参考

- T0495 のメモ (手順・未解決のシンボル・`panic_immediate_abort` の扱い・起票の候補)、crates/ugoe-ffi/README.md#std-の無い環境
- .github/workflows/fuzz.yml (定期のジョブの例。T0188)、docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md

## メモ

- T0068 で起票した (元の候補: T0495 ((任意) nightly の `-Zbuild-std` の検査を、PR を止めない定期の CI ジョブで回す))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
- 同じ scripts/nostd-check.mjs を触る T0070 の後にした。
