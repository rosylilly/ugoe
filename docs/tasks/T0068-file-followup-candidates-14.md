---
id: T0068
title: M9 の後続のタスクの「起票の候補」をまとめて起票する (14 回目)
status: done
milestone: M9
crates: [docs]
depends_on: []
assignee: claude-worker-T0068
---

# T0068: M9 の後続のタスクの「起票の候補」をまとめて起票する (14 回目)

## 目的

M9 の後続のタスクを並列で進めたワーカーは、新しいタスクを起票せずに「起票の候補」をメモに書いた (番号の衝突を避けるため)。
T0066 の後に集めた候補の一覧を読み、重複をまとめ、既存のタスクに含まれるものは除いて、タスクとして起票する。

## 受け入れ条件

- [x] オーケストレーターの一覧 (下の「参考」) と、T0066 の後に done になった M9・M10 のタスクのメモの「起票の候補」を読んだ
- [x] 同じことを言っている候補を 1 つのタスクにまとめ、既存のタスク (todo のもの) に含まれる候補はそのタスクのメモに書き足して起票しない
- [x] 残りを docs/tasks/README.md の形でタスクにした (milestone は元のタスクに合わせて M9・M10・M11。受け入れ条件は検証できる形。depends_on を正しく。`crates:` は狭く)
- [x] 公開 API の変更や仕様の判断が要るものは、タスクの本文に「判断が要る」と書いた
- [x] 番号: docs/tasks/README.md の番号帯の規則に従う (エディタは T1677〜、ほかは空いている番号)
- [x] 起票したタスクの一覧 (ID・題名・元の候補) をメモに書いた
- [x] `node scripts/tasks.mjs` が通る (未知の依存が無い)
- [x] `just ci` が通る

## 参考

- オーケストレーターの集計: /tmp/claude-1000/-home-rosylilly-src-github-com-rosylilly-ugoe/026b491e-8c79-47ed-ac14-04f23c5addc0/scratchpad/run2/candidates-for-T0068.md
- T0019・T0026・T0032・T0036・T0038・T0040・T0041・T0043・T0046・T0052・T0058・T0063・T0066 (1〜13 回目)。リリース前なので移行の説明や互換の別名は要らない (利用者の方針)
- docs/tasks/README.md、docs/workflow/multi-agent.md#タスクの書き方-オーケストレーター向け

## メモ

- T0066 (13 回目) の後に、M10・M11 の後続のタスクのワーカーが出した候補。

### やったこと

- オーケストレーターの一覧 (`candidates-for-T0068.md`、8 行) と、T0066 の後に done になったタスク (M11: T0067・T0186〜T0188・T0493〜T0495、
  M10: T0981) のメモの「起票の候補」を読んだ。一覧と同じで、ほかに候補は無かった (T0186・T0187・T0494・T0981 は候補なし)。
- todo・blocked のタスク (T0005・T0030・T0407・T0417・T0421・T0443・T0619・T0731・T0830・T0973・T0974) に含まれる候補は無かった。
  8 つの候補は互いに別のことなので、まとめずに 7 つのタスクにし、「CI で確かめる」の 1 つはタスクにせずに下に残した。
- 今のコードを見て残っていることを確かめた: scripts/ffi.mjs の `system` に Windows が無い。scripts/nostd-check.mjs のターゲットは
  `aarch64-unknown-none` だけ。ci.yml に fuzz のクレートのコンパイルが無い。`fuzz/Cargo.toml` は `unsafe_code = "deny"`。
- T0493 (a) は、`crates/ugoe-ffi/clippy.toml` を一時的に `crates/ugoe-wasm/` に写して clippy を流して確かめた (コミットしていない):
  出たのはテストの中の `f32::sin_cos` の 2 か所 (gizmo.rs・physics_view.rs) だけで、ライブラリのコードは通った。結果が変わらないので
  「入れるか決める」を判断の要らない「入れる」タスク (T0496) にした。
- **判断が要るもの:** T0497 (C ABI の fuzz のターゲットは `unsafe` が要り、`fuzz/` は `unsafe` を禁じている。案 A 例外を足す・
  B `ugoe-ffi` に feature の安全な入口・C 足さない。推奨 B)。T0189 は外部 (image-rs/image-png) への投稿なので「利用者の承認が要る」と本文に書き、
  承認までは報告の文を用意するだけにした。
- **マイルストーン:** すべて M11 の後続 (元は M11 のタスク)。docs/roadmap.md の M11 の「後続のタスク」に足した (fuzz の項を新しく作った)。
- 番号は各帯の空いている番号から: 基盤 T0069〜T0072、モデル・ランタイム・形式 T0189、バインディング T0496・T0497。エディタのタスクは無い。
- 依存: T0071→T0070 (同じ scripts/nostd-check.mjs)。ci.yml を触る T0069・T0070・T0072 は別のジョブなので付けていない。

### 起票したタスク

| ID | 題名 | 元の候補 |
| --- | --- | --- |
| T0069 | CI: Windows で just ffi-check の std がある形 (C の見本のリンクと実行) を流す | T0067 (Windows で ffi-check) |
| T0070 | just nostd-check: 64 ビットのアトミック操作の無い tier 2 のターゲット (thumbv7em-none-eabihf) でもビルドを確かめる | T0495 (thumbv7em-none-eabihf) |
| T0071 | CI (任意): nightly の -Zbuild-std で tier 3 のターゲットの std の無い静的ライブラリを作る検査を、定期のジョブで回す | T0495 ((任意) nightly の定期の検査) |
| T0072 | CI: PR ごとに fuzz のターゲットが stable でコンパイルできるかを確かめる | T0188 (PR の CI で fuzz の cargo check) |
| T0189 | png クレートの上流に、PLTE の長さが 3 の倍数でないときのパニックを報告する (外部への投稿。利用者の承認が要る) | T0188 (png の上流に報告) |
| T0496 | ugoe-wasm: std の数学関数の禁止 (MATH_CRATES の clippy.toml) を有効にする | T0493 (a) |
| T0497 | fuzz: C ABI (ugoe-ffi) の入口に直接入力を渡す fuzz のターゲットを足す (判断が要る) | T0188 (C ABI の fuzz のターゲット) |

### タスクにしなかったもの (CI で確かめる)

- T0188: fuzz.yml を初めて手で起動 (workflow_dispatch、秒数を小さく) したとき、`cargo-fuzz` の入り方 (taiki-e/install-action から
  cargo-binstall に落ちる)・`actions/cache/restore@v6`・`save@v6` と corpus の持ち越し・`Swatinem/rust-cache` の `workspaces: fuzz -> target`・
  bundle のジョブの `just fuzz-check`・見つかったときの成果物 `fuzz-artifacts-<target>` を確かめる。

### 検査

- `git rebase main` (追従は不要だった) の後に `node scripts/tasks.mjs` (未知の依存なし) と `just ci` (前面、終了コード 0) が通った。

### 起票の候補

- なし。
