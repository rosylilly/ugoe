---
id: T1000
title: "CI: ランナーのイメージの stable の rustc が上がっても、rust-cache のキャッシュがすべて外れないようにする"
status: done
milestone: Release
crates: [.github/workflows, rust-toolchain.toml, docs]
depends_on: []
assignee: claude-worker-T1000
---

# T1000: rust-cache の鍵とランナーの stable

## 目的

`Swatinem/rust-cache` は環境のハッシュに `rustc` の版を入れる。ランナーのイメージが stable の rustc を上げると (10-11 に 1.99.0)、
`rust-toolchain.toml` で固定した版を使っていてもすべてのキャッシュが外れる (T0091 の run 38098330681 で rust (apps) が約 6 分 → 約 26 分)。
固定した版だけが鍵に入るようにし、イメージの更新でキャッシュが外れないようにする。

## 受け入れ条件

- [x] 原因を確かめた: rust-cache が鍵に入れる版が、ランナーの既定の `rustc` (イメージの stable) か、`rust-toolchain.toml` の版か
      (rust-cache の action.yml と実装を読み、run のログの鍵で確かめる)
- [x] 直した。案: (1) rust-cache の前に `rustup show` (か `rustup toolchain install`) で `rust-toolchain.toml` の版を入れ、作業ディレクトリでの
      `rustc` がその版になる順にする、(2) `add-rust-environment-hash-key: false` にし、`rust-toolchain.toml` の版を `prefix-key` か `key` に自分で入れる。
      どちらにしたかと理由をメモに書く
- [x] `ci.yml`・`release-checks.yml` ほか rust-cache を使うすべてのワークフローで同じ形にした (`shared-key` を共有するジョブの鍵が食い違わない)
- [ ] actionlint が通る。実際の run で、イメージの stable と固定の版が違うときもキャッシュが当たることを確かめた (Actions が使えなければ、
      確かめていないこととしてメモに書き、ログで鍵の文字列を確かめる手順を書く)
- [x] docs/testing.md#ci-のジョブ のキャッシュの説明を直した
- [x] `just ci` が通る

## 参考

- T0091 のメモ (測定・キャッシュの鍵・起票の候補)、docs/testing.md#ci-のジョブ
- Swatinem/rust-cache の README (`add-rust-environment-hash-key`・`prefix-key`)

## メモ

- T0095 で起票した (元の候補: T0091 (ランナーのイメージの stable の rustc が上がると rust-cache の鍵が変わってキャッシュがすべて外れる))。
- Release のマイルストーン。T0091 と同じ。
- 番号: 基盤の帯 T00xx を使い切ったので、2 つ目の範囲 T10xx の最初。
- **原因 (確かめた):** 鍵に入るのは `rust-toolchain.toml` の版とイメージの stable の両方。rust-cache v2 (今は v2.9.2) は v2.9.0 から
  (Swatinem/rust-cache#293)、環境のハッシュに作業ディレクトリの `rustc -vV` に加えて `rustup toolchain list --quiet` のすべての
  ツールチェインの `rustup run <tc> rustc -vV` を入れる (src/config.ts の `getRustVersions`)。run のログ (`rust (apps)`) の
  「Cache Configuration」の「Rust Versions」で確かめた:
  - 37738790150 (10-08、当たった): `1.98.1 … 48a229ce` だけ。Restore Key `v0-rust-group-apps-Linux-x64-447c32e2`。
  - 38098330681 (10-11、外れた): `1.98.1 … 48a229ce` と `1.99.0 … b940084d` (イメージの stable)。Restore Key
    `v0-rust-group-apps-Linux-x64-52846f42` で「No cache found」。
  10-08 はイメージの stable がたまたま固定と同じ 1.98.1 で、1 行にまとまっていた。
- **直し方:** 案 (1) (rust-cache の前に固定の版を入れる) はすでにそうなっていて、上のとおり効かない。案 (2)
  (`add-rust-environment-hash-key: false`) は `Cargo.toml`・`Cargo.lock` のハッシュも鍵から外す (action.yml の説明と実装) ので、鍵が
  固定になって完全一致では保存されず、キャッシュが新しくならない。自分で `hashFiles` と `CARGO_*` の値を鍵に入れ直すことになり、
  shared-key を共有するジョブで揃えるのも難しい。そこで案 (3) にした: 複合アクション `.github/actions/rust-toolchain` が
  `rustup toolchain install` (rust-toolchain.toml の版)・`rustup default <固定の版>`・ほかのツールチェインの `rustup toolchain uninstall`
  を行い、rust-cache を使うすべてのジョブ (ci.yml の rust (グループ)・web、release-checks.yml の cross・rust (windows/macos)・gpu・
  godot・conformance、e2e.yml、perf.yml、fuzz.yml、crates-release.yml の 2 つ、web-runtime-release.yml) で rust-cache の前に置いた。
  shell は bash (Windows と macOS のジョブでも動く)。shared-key を共有するジョブはどれも同じ手順なので鍵が揃う。
  fuzz.yml は、この手順の後に nightly を入れる (nightly の版は今までどおり鍵に入る。fuzz の target は nightly でビルドするため)。
- `.github/actions/` は `crates:` の外だが、ワークフローから使う小さな複合アクションなのでここに置いた。
- rust-cache を使わないジョブ (ci.yml の rust-checks・rust-groups、unity.yml、editor-build.yml、ffi-release.yml) は今の
  `rustup show active-toolchain || rustup toolchain install` のまま (stable を消す時間をかける理由が無い)。
- actionlint 1.7.12 が通る (手元の shellcheck 0.9.0 を `-shellcheck` で渡した)。複合アクションのスクリプトは shellcheck で検査し、
  rustup を真似た小さなスクリプトで、固定の版以外だけを消すことを確かめた。
- **確かめていないこと:** Actions が止まっているので、実際の run でイメージの stable と固定の版が違うときにキャッシュが当たるかは
  確かめていない。確かめ方: main に入れた後の最初の ci.yml の run (保存する側) と次の run で、`rust (apps)` などのジョブのログの
  rust-cache の手順の「Cache Configuration」を開き、(1) 「Rust Versions」が固定の版 (`1.98.1 …`) の 1 行だけ、(2) 2 回目の run で
  「Restored from cache key … full match: true」(か前方一致の hit) になっていることを見る (`gh run view <run> --job <job> --log | grep -A3
  -E 'Rust Versions|Restore Key|Restored from|No cache found'`)。最初の run は鍵の環境のハッシュが変わるので一度だけ外れる。
  Windows・macOS のジョブ (release-checks.yml の rust-os) で `rustup toolchain uninstall` が通ることも確かめていない。
- **起票の候補:** 無し。
