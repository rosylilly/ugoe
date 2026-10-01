---
id: T0002
title: CI: Rust のテストをクレートのまとまりごとのジョブに分け、ランナーのディスク不足を防ぐ
status: done
milestone: M7
crates: [.github/workflows, justfile, scripts, docs, crates/ugoe-ffi/README.md]
depends_on: []
assignee: claude-worker-T0002
---

# T0002: CI の Rust のテストを分ける

## 目的

GitHub Actions の CI の `rust` ジョブが、push のたびに失敗している。

- run 36464556766 (2026-09-28): テストのビルド中に `No space left on device` (リンカーも Bus error) で失敗。
- run 36801608426 (2026-10-01): Test のステップが約 15 分で取り消され、ジョブが止まったまま。

`cargo test --workspace` を 1 つのジョブで行うので、ワークスペースが大きくなるとランナーのディスク (約 14GB) と時間を超える。テストをクレートのまとまりごとの別のジョブに分けて、今後クレートが増えても 1 つのジョブが肥大化しないようにする。

## 受け入れ条件

- [x] ワークスペースのクレートを、いくつかのまとまり (例: ランタイム層・ツール層・アプリ (Tauri のエディタとビューア)・バインディング/適合テスト) に分け、まとまりごとに clippy とテストを行うジョブにする (matrix など)。まとまりの定義は 1 か所 (scripts の JSON や mjs、justfile のレシピ) に置き、CI とローカルで同じものを使う
- [x] すべてのクレートが、どれか 1 つのまとまりに入っていることを検査する (クレートを足したときの入れ忘れを防ぐ。scripts/check-layers.mjs の登録と同じように)
- [x] ディスクの対策: 各ジョブの最初にランナーの不要なもの (.NET・Android SDK・Haskell・CodeQL など) を消す。CI ではデバッグ情報を減らす (`CARGO_PROFILE_DEV_DEBUG` など) か、そうしない理由を書く。rust-cache の key をまとまりごとに分ける
- [x] Tauri のシステムの依存 (webkit など) は、アプリのまとまりのジョブだけで入れる
- [x] fmt・生成物・layers・licenses・publish-check・tasks・wasm32・C ABI の検査は、どれかのジョブに残す (今の rust ジョブの検査を落とさない)
- [x] `just ci` (ローカル) の振る舞いは変えない (全部を走らせる)。まとまりごとに走らせる `just test-group <名前>` のようなレシピを足してよい
- [x] actionlint で検査する。各ジョブに timeout-minutes を付ける
- [x] docs/testing.md (と必要なら AGENTS.md) に、CI のジョブの分け方と、クレートを足したときの手順を書く
- [x] `just ci` が通る。実際の CI は利用者の push の後に確かめる (CI は未確認)

## 参考

- `.github/workflows/ci.yml`、`justfile` の lint・test、`scripts/check-layers.mjs`

## メモ

### やったこと

- グループの定義を `scripts/ci-groups.mjs` の `GROUPS` に置いた (runtime・bindings・assets・tools・apps の 5 つ)。
  `check` (すべてのクレートがちょうど 1 つのグループに入っているか、`required-features` の feature が書かれているか)、
  `args` (cargo の引数)、`matrix` (CI の matrix の JSON)、`list` を持つ。検査のテストは `scripts/ci-groups.test.mjs`。
- justfile: `just groups` (テストと検査。`just ci` に足した)、`just lint-group <g>`、`just test-group <g>`。`just ci` はワークスペース全体のまま。
- ci.yml: `rust` ジョブを `rust-checks` (fmt・layers・groups・licenses・publish-check・tasks)、`rust-groups` (matrix を作る)、
  `rust-group` (matrix。clippy とテスト。bindings で wasm32 と C ABI、apps で生成物の検査と Tauri の依存)、`rust` (結果のまとめ) に分けた。
  各ジョブに timeout-minutes。`rust-group` はランナーの .NET・Android SDK・Haskell・CodeQL を消し、`CARGO_PROFILE_DEV_DEBUG=0`、
  rust-cache の key をグループごとにし、保存は main だけにした。gpu・godot・conformance・web は変えていない。
- `-p` で選ぶとワークスペース全体の feature の統合が効かない。`ugoe-model` の `spec_files` テストは `schema` が要り、ワークスペースでは
  `ugoe-editor-core` が有効にしていたので、runtime の features に `ugoe-model/schema`・`ugoe-model/ts` を書いた (書き忘れは check が見つける)。
- docs/testing.md に「CI のジョブ」と「クレートを足したとき」を書き、AGENTS.md の新しいクレートの手順とコマンドの表に足した。
- actionlint (1.7.12) で検査した。新しいジョブには指摘なし (gpu と conformance の既存の `ls` に SC2012 の info が出るが、変えていない)。

### ディスクの見積もり (手元。グループごとに CARGO_TARGET_DIR を分け、`CARGO_INCREMENTAL=0` で lint-group と test-group を実行した後の target)

| グループ | debug=0 (CI の設定) | debug=line-tables-only | 既定 (debug=true) |
| --- | --- | --- | --- |
| runtime | 0.84GB | 1.5GB | - |
| bindings | 0.92GB (+wasm32 の clippy で 1.2GB) | 1.6GB | - |
| assets | 0.72GB | 1.1GB | - |
| tools | 1.4GB | 2.9GB | 6.0GB |
| apps | 2.7GB (+`just gen` で 2.8GB) | 5.3GB | - |
| (参考) ワークスペース全体 (以前の rust ジョブ) | - | - | 17.1GB |

以前の rust ジョブは手元で 17GB になり、ランナーの空き (約 14GB) を超えていた。測った target は消した。
レジストリ (~/.cargo) と apt の分は含まない。手元の `just ffi-check` は `target/` を決め打ちで読むので、CARGO_TARGET_DIR を分けた測定では失敗した (CI では既定の target なので問題ない)。

### 確かめていないこと

- 実際の GitHub Actions での実行 (push は利用者が行う)。Free disk space の効き目と時間、各グループの所要時間は CI のログ (`Disk usage` のステップ) で確かめる。
- timeout-minutes は新しい Rust のジョブに付けた。commitlint と web のジョブは指示どおり変えていない (付いていない)。
- ブランチの保護で `rust` を必須にしているなら、名前は `rust` (まとめのジョブ) のまま残している。

### 範囲外

- `crates/ugoe-ffi/README.md:50` の「CI の `rust` ジョブで別に実行する」は `rust (bindings)` ジョブになった。範囲外なので team-lead に相談し、承認を受けて team-lead が直した。

### 後続の候補

- `scripts/ffi.mjs` が CARGO_TARGET_DIR を見ない。
- apps のグループが一番大きい (2.8GB)。エディタとビューアを分けることもできる。
- lefthook の pre-push は今もワークスペース全体の clippy とテスト (手元は変えていない)。
