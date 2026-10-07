---
id: T0072
title: "CI: PR ごとに fuzz のターゲットが stable でコンパイルできるかを確かめる"
status: done
milestone: M11
crates: [.github/workflows, fuzz, justfile, docs]
depends_on: []
assignee: claude-worker-T0072
---

# T0072: CI: PR ごとに fuzz のターゲットが stable でコンパイルできるかを確かめる

## 目的

T0188 で、ワークスペースの外の `fuzz/` (ugoe-fuzz、独自の Cargo.lock) に cargo-fuzz のターゲットを置き、週に 1 度の fuzz.yml で回すようにした。
`fuzz/` はルートのワークスペースの外なので、`ugoe`・`ugoe-format` の公開 API を変えてターゲットが壊れても、PR の CI では気づかず、
次の週の fuzz.yml で初めて落ちる。PR ごとの CI で、stable で `cargo check --manifest-path fuzz/Cargo.toml` (か clippy) を流し、早く気づく。

## 受け入れ条件

- [x] PR ごとに流れる CI のジョブで、fuzz のクレートを stable でコンパイルする手順を足した (どのジョブに置くか (rust-checks はビルドの
      キャッシュを持たない・rust-group の bindings など) を、時間とキャッシュから決めて理由をコメントに書く。`libfuzzer-sys` の
      ビルドスクリプトが C++ の libFuzzer をコンパイルすることと、`fuzz/Cargo.lock` の `--locked` も考える)
- [x] 入口を `just` のレシピにするか (`just fuzz-check` の一部だけ、など) を決め、justfile と fuzz/README.md を直した
- [x] ランタイムの公開 API を一時的に変えると、その手順が失敗することを手元で確かめ、メモに書いた (戻した変更はコミットしない)
- [x] docs/testing.md#fuzz と CI のジョブの表を直した。push した後の CI で確かめること (時間の延び・キャッシュ) をメモに書いた
- [x] `just ci` が通る

## 参考

- T0188 のメモ (起票の候補)、docs/adr/0032-fuzzing-with-cargo-fuzz.md、fuzz/run.mjs (`--check`)、.github/workflows/fuzz.yml
- docs/testing.md#fuzz、docs/testing.md#ci-のジョブ

## メモ

- T0068 で起票した (元の候補: T0188 (PR ごとの CI に `cargo check --manifest-path fuzz/Cargo.toml` を足す))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
- ci.yml は T0069 (rust (windows)) も触るが、別のジョブなので依存は付けていない。fuzz/ にターゲットを足す T0497 とも依存は付けていない
  (どちらが先でも、後のタスクが新しいターゲットを検査に含める)。

### 作業の記録 (claude-worker-T0072)

- **stable で通るか:** 通る。`libfuzzer-sys` 0.4 は stable の `cargo check`・`clippy` でビルドできる (nightly が要るのは
  `cargo fuzz build` の計装 (`-Zsanitizer` など) だけ)。ただしビルドスクリプトが同梱の libFuzzer (C++) を `cc` で check でも
  コンパイルするので、C++ のコンパイラが要る (ubuntu-latest には g++ がある)。
- **入口:** `just fuzz-lint` (`node fuzz/run.mjs --lint`) を足した。`just fuzz-check` (`--check`) のうち nightly のビルドを除いたもの
  (依存のライセンス `check-licenses.mjs --fuzz`・`cargo fmt --check`・`cargo clippy --locked -- -D warnings`)。
  ライセンスと整形は数秒もかからないので、コンパイルだけに絞らず fuzz-check と同じ stable の検査を流す。
- **置き場所:** ci.yml の `rust (runtime)` (rust-group の matrix の runtime) に「Fuzz targets」の手順を足した。理由はコメントに書いた:
  ターゲットが使う `ugoe`・`ugoe-format` はランタイム層で、その API を変える PR は必ずこのジョブを流す。rust-checks は
  「ワークスペースをビルドしない」ジョブなので置かない。bindings はランタイムと関係が薄い。
- **キャッシュ:** 足さない。ビルドは `fuzz/target` (rust-cache が持ち越す `target` の外) で、依存は約 50。手元で新しい target から
  `CARGO_BUILD_JOBS=4` で clippy が約 10 秒 (libFuzzer の C++ を含む。target は約 200MB)。ランナーでも 1 分を超えないと見込んだ。
  `CARGO_TARGET_DIR=target` で共有しても、`ugoe` などは feature が違い、rust-cache は fuzz だけの依存 (libfuzzer-sys など) を
  ルートの metadata に無いので掃除する見込みで、得が小さい。
- **`--locked`:** clippy とライセンスの検査 (`cargo metadata --locked`) は `--locked`。手元で ugoe-core に `base64` を一時的に足すと、
  `just fuzz-lint` が「cannot update the lock file fuzz/Cargo.lock because --locked was passed」で落ちた (ライセンスの検査が先に落ちる。
  Node のスタックが出るが文言は分かる)。`cargo update --manifest-path fuzz/Cargo.toml --workspace` で `base64` だけが足された。
  直し方を fuzz/README.md と docs/testing.md#fuzz に書いた。
- **壊れることの確認:** `crates/ugoe-format/src/bundle.rs` の `read_bundle` に引数を 1 つ足すと、`just fuzz-lint` が
  `error[E0061]: this function takes 2 arguments but 1 argument was supplied --> src/lib.rs:61:5` で終了コード 101 になった。
  どちらの一時的な変更も `git checkout` で戻し、コミットしていない。
- **文書:** docs/testing.md (#fuzz と CI のジョブの表の `rust (runtime)`)、fuzz/README.md、ADR 0032 の決定 3 (PR ごとの CI は
  stable の検査だけを流す、に直した。決定を覆してはいないので新しい ADR は書いていない)、fuzz.yml のコメント、justfile。
- **範囲の外の小さな変更:** `scripts/check-licenses.mjs` の先頭のコメント (`--fuzz` を `--check` と `--lint` が呼ぶ) だけ。
- **`just ci` に入れるか:** 入れていない。C++ のコンパイラが要るのと、ADR 0032 の「`just ci` に入れない」に合わせた。
  手元ではランタイムの公開 API を変えたら `just fuzz-lint` を流すよう docs/testing.md#fuzz に書いた。
- **push の後の CI で確かめること (push していないので未確認):** (1) `rust (runtime)` の「Fuzz targets」が通り、かかる時間
  (見込みは 1 分以内。libFuzzer の C++ を含む) とジョブ全体の延び。(2) ランナーの g++ で libFuzzer がコンパイルできること。
  (3) `Disk usage` の手順で、fuzz/target の分 (約 200MB) を足してもディスクに余裕があること。(4) 延びが大きければ、rust-cache の
  `workspaces` に `fuzz -> fuzz/target` を足すか考える。actionlint (shellcheck 無し) は ci.yml と fuzz.yml で通った。
- T0069 が ci.yml の rust-os を並行して触っている。こちらは rust-group の runtime の手順だけを足したので、衝突は手順の並びの付近に限られる見込み。

### 起票の候補

- `scripts/check-licenses.mjs` の `cargoMetadata` が `--locked` で落ちたとき、Node のスタックではなく「fuzz/Cargo.lock が古い。
  `cargo update --manifest-path fuzz/Cargo.toml --workspace` で直す」と短く出す (godot も同じ)。
- `fuzz/Cargo.lock` の glam (0.33.12) などがルートの `Cargo.lock` と版が違う。揃えるか (揃える仕組みを持つか) を決める。

