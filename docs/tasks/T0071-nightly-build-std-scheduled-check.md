---
id: T0071
title: "CI (任意): nightly の -Zbuild-std で tier 3 のターゲットの std の無い静的ライブラリを作る検査を、定期のジョブで回す"
status: done
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

- [x] 64 ビットのターゲット (`aarch64-nintendo-switch-freestanding`) と、64 ビットのアトミック操作の無い 32 ビットのターゲット
      (`armv6k-nintendo-3ds` など) の 2 つ以上で、`ugoe-ffi` の静的ライブラリを `-Zbuild-std` で作り、未解決のシンボルが決めた一覧と同じかを
      確かめる入口を作った (scripts/nostd-check.mjs の部分コマンドか、`just` のレシピ。`llvm-nm` などの道具の入れ方も)
- [x] 定期 (と手動) で流す GitHub Actions のジョブを足した (nightly が壊れても PR の CI は止めない。`rust-src` の入れ方も)。
      新しいワークフローにするか、ほかの定期のジョブ (fuzz.yml) に入れるかを決め、理由を書いた
- [x] 手元で入口を流した結果と、push した後の CI で確かめることをメモに書いた
- [x] crates/ugoe-ffi/README.md (core と alloc が配られないターゲット) と docs/testing.md に、検査があることを書いた
- [x] `just ci` が通る

## 参考

- T0495 のメモ (手順・未解決のシンボル・`panic_immediate_abort` の扱い・起票の候補)、crates/ugoe-ffi/README.md#std-の無い環境
- .github/workflows/fuzz.yml (定期のジョブの例。T0188)、docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md

## メモ

- T0068 で起票した (元の候補: T0495 ((任意) nightly の `-Zbuild-std` の検査を、PR を止めない定期の CI ジョブで回す))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
- 同じ scripts/nostd-check.mjs を触る T0070 の後にした。

### やったこと (T0071)

- **入口:** scripts/nostd-check.mjs に部分コマンド `build-std [<target>...]` と `just nostd-build-std [ターゲット...]` を足した。
  `BUILD_STD_TARGETS` = `aarch64-nintendo-switch-freestanding` (64 ビット) と `armv6k-nintendo-3ds` (32 ビット、64 ビットのアトミック操作が無い)。
  `cargo +nightly rustc -p ugoe-ffi --lib --crate-type staticlib --no-default-features --profile nostd --target <t> --target-dir target/build-std
  -Zbuild-std=core,alloc -Zbuild-std-features=compiler-builtins-mem` で作り、nightly の llvm-tools の `llvm-nm -j` で、静的ライブラリの
  どのメンバも定義しない未解決のシンボルを集めて `HOST_SYMBOLS` (6 つ) と完全に同じかを確かめる (一覧に無いもの・一覧にあって無いものの
  どちらも失敗)。ターゲットが失敗しても残りを続ける。
  - 道具: nightly・`rust-src`・`llvm-tools` (`rustup toolchain install nightly --profile minimal --component rust-src,llvm-tools`)。
    無ければ入れ方を出して止まる。`LLVM_NM`・`UGOE_NIGHTLY_TOOLCHAIN` で変えられる。llvm-nm をシステムの LLVM でなく llvm-tools に
    したのは、rustc と同じ LLVM で読めることが保証され、CI でも apt が要らないため。
  - リンク (rust-lld と C の見本) ではなく llvm-nm にしたのは、32 ビットのターゲットごとに clang の浮動小数点の ABI を合わせる必要が
    無いため (T0495 の PSP の件)。使われない関数の参照も残るが、`compiler-builtins-mem` 付きでは 2 つのターゲットとも 6 つと同じだった。
  - `target/build-std` に分けたのは、stable の `nostd` プロファイルの成果物 (ホストのビルドスクリプトなど) と rustc の版の違いで作り直し
    合わないため。mbx は nightly の `-Z` 付きのビルドで `metadata probing failed` で止まるので `MBX_DISABLE=1` で通さない (fuzz/run.mjs と同じ)。
  - scripts/nostd-check.test.mjs に、ターゲットの選択・引数・未解決のシンボルの集め方・一覧との違いのテストを足した。
- **ワークフロー:** 新しい `.github/workflows/nostd-build-std.yml` (週に 1 度 月曜 04:47 UTC と `workflow_dispatch`。PR の CI の外)。
  fuzz.yml に入れなかった理由: fuzz.yml はターゲットごとの matrix と corpus のキャッシュの形で、混ぜると失敗の意味 (fuzz で見つかった入力か
  nightly・依存の変更か) が分かりにくく、片方だけを手動で流せなくなるため。`rust-src` と `llvm-tools` は `rustup toolchain install nightly
  --profile minimal --component rust-src,llvm-tools` で入れる。rust-cache は nightly が毎回変わってキャッシュが当たらないので使わない。
- **文書:** docs/testing.md (種類の表・`just ci` に入らないものの一覧・CI のジョブの節に定期のジョブの説明・新しい節「tier 3 のターゲットの
  build-std」)、crates/ugoe-ffi/README.md (core と alloc が配られないターゲット)、docs/portability.md (ゲーム機の行の「CI では検査しない」)。
- **手元の結果 (nightly 1.101.0-nightly 8d1a76430 2026-10-06):** `just nostd-build-std` が 2 つのターゲットで通り、どちらも未解決のシンボルは
  `abort, aligned_alloc, calloc, free, malloc, realloc`。冷えた状態から約 60 秒 (ターゲットごとに約 30 秒)。
  - 効き目: 一時的に `compiler-builtins-mem` を外すと `armv6k-nintendo-3ds` で `一覧に無いもの: memcmp, strlen` を出して失敗した
    (戻した変更はコミットしていない)。ツールチェインが無い・llvm-nm が無い・知らないターゲットのときに入れ方やメッセージを出して止まることも確かめた。
- `just ci` は通った。

### CI で確かめること (push していない)

- nostd-build-std.yml を手動 (`workflow_dispatch`) で流し、nightly と `rust-src`・`llvm-tools` が入り、2 つのターゲットで
  `build-std ok` が出て成功すること。ubuntu-latest で 30 分の制限に十分収まること (手元では約 1 分)。
- 定期の実行 (月曜) が起動すること。失敗しても PR の必須の検査 (ci.yml の `rust`) に影響しないこと。

### やらなかったこと

- AGENTS.md のコマンドの表への `just nostd-build-std` の追記 (`crates:` の外。`just fuzz` は載っているので、載せるならオーケストレーターの判断)。
- C の見本とのリンク・Vita と PSP のターゲット (受け入れ条件は 2 つ以上。時間を抑えた)。nightly の日付の固定はしていない (最新の nightly で
  壊れたことに気づくのが目的のため)。

### 起票の候補

- なし
