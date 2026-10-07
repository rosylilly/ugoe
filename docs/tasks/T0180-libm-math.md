---
id: T0180
title: ランタイム層の数学関数を ugoe_core::math (常に libm) に寄せる
status: done
milestone: M11
crates: [ugoe-core, ugoe-runtime, ugoe-physics, ugoe-motion, ugoe, Cargo.toml, Cargo.lock, tests/conformance, tools/spec-check, assets/fixtures, docs]
depends_on: [T0179]
assignee: claude-worker-T0180
---

# T0180: ランタイム層の数学関数を ugoe_core::math (常に libm) に寄せる

## 目的

`f32` の `sin`・`cos`・`powf`・`exp`・`atan2` などは `std` にしか無い。ランタイム層が数学関数を `ugoe_core::math` から呼ぶようにし、
その中身は `std` の有無にかかわらず常に `libm` にする ([ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md#数学関数は-libm-に寄せる))。
no_std で使えるようになるのと同時に、OS の数学ライブラリの違いによる出力の差が無くなる。

## 受け入れ条件

- [x] `ugoe-core` に `math` モジュールを足し、ランタイム層 (`ugoe-runtime`・`ugoe-physics`・`ugoe-motion`・`ugoe`) の `f32`・`f64` の数学関数の呼び出しを
      すべて置き換えた。`glam` は `libm` の feature を有効にした
- [x] 置き換えの漏れを検査する仕組みがある (clippy の `disallowed_methods` などで、ランタイム層のクレートで `f32::sin` などを直接呼ぶと警告になる)
- [x] 適合テスト (`just conformance`)・spec-check・スナップショットが許容誤差の中で通る。収まらないものは理由を確かめてから期待値を作り直した
- [x] 置き換えの前後で `Puppet::update` の速さを見本のモデルで測り、メモに書いた (目に見えて遅い関数があれば対策した)
- [ ] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、docs/testing.md、tests/conformance/README.md

## メモ

- 2026-10-06 起票。`sqrt`・`abs` などは IEEE で結果が決まるので、どちらを使っても値は同じ。
- 2026-10-07 (claude-worker-T0180) やったこと:
  - `ugoe_core::math` を足した (`sin`・`cos`・`sin_cos`・`tan`・`atan2`・`sqrt`・`hypot`・`powf`・`exp`・`ln`・`floor`・`ceil`・`round`・`trunc`・
    `rem_euclid`・`rem_euclid_f64`)。中身は常に `libm` (`rem_euclid` は std と同じ式を `%` で書いた)。std があるときは std の結果と比べるテストを持つ
    (IEEE で決まる関数はビット単位で同じ、超越関数は 1 ULP 以内)。`glam` は `nostd-libm` を `libm` に替えた (ワークスペースの全体で glam の中の数学関数が libm になる)。
    依存 `libm = "0.2"` を `[workspace.dependencies]` に足した (`glam` の libm と同じクレート。既定の feature `arch` は sqrt・fma などの正確に丸まる命令だけを使うので、結果は変わらない)。
  - `ugoe-runtime`・`ugoe-physics`・`ugoe-motion`・`ugoe` の数学関数の呼び出し (テスト・example を含む) を `ugoe_core::math` に替えた。
    `abs`・`signum`・`to_radians`・`to_degrees` は core にあり IEEE で決まるのでそのまま。
  - 漏れの検査: `ugoe-core`・`ugoe-runtime`・`ugoe-physics`・`ugoe-motion`・`ugoe` に同じ `clippy.toml` を置き、std の `f32`・`f64` の数学関数
    (std にしか無いもの 36 個ずつ) を `disallowed-methods` にした。clippy は最も近い clippy.toml だけを読むので、ルートの設定 (`allow-unwrap-in-tests` など) も写した。
    写しが揃っているか (存在・同じ内容・ルートの設定を含む) は `scripts/nostd-check.mjs` の `MATH_CRATES` と `check` (`just groups`) が検査する。
  - 出力の変化: 置き換えの前 (main) と後で、見本のモデル (`assets/samples/avatar.ugoe`、物理と 4 つのモーションを入れ替えながら) と `dynamic_hair`・`physics_relay` を
    `Puppet::update` で 900 フレームずつ進め、パラメータ・頂点・不透明度を比べた。261 万個の値のうち 401 個が変わり、最大の差は 6.1e-5 (533.67 の 1 ULP)、ULP では最大 256 (値 -1.30)。
  - 期待値で変わったもの: `ugoe-physics` の `golden_tests` の 4 つのハッシュ (ビット単位の比較なので、数学関数の最後の桁で変わる)。上の比較で差が小さいことを確かめてから記録し直し、
    理由をテストのコメントに書いた。x86_64 Linux だけで比べる制限は残した (libm でほかのプラットフォームと揃うはずだが、確かめていない)。
  - 変わらなかったもの: 適合テスト (`just conformance`。参照もバインディングも同じビルドの libm を使うので、許容誤差の中で通る)、spec-check (`just spec`。TypeScript の独立した実装で、
    期待値は手で導いた `spec/examples/` のもの)、`just gen` (公開仕様の例の再生成。差分なし)、互換性テスト (`assets/compat`)、スナップショット (insta) は作り直し無しで通った。
  - 速さ (リリースビルド、24 スレッドの機械で他のワーカーと並行。5 回の中央値): 見本のモデルの `Puppet::update` は前 15.6〜15.8 us/フレーム、後 15.4〜15.7 us/フレームで差は誤差の範囲。
    `ModelInstance::update` だけでパラメータを掃いても 14.9〜16.9 us で差は見えない。`physics_timing` の example も前後で同じ (physics_hair 0.16、physics_relay 0.24、dynamic_hair 0.77〜0.78 us/フレーム)。対策は要らなかった。
  - 範囲 (`crates:`) の外: `scripts/nostd-check.mjs` (検査の追加。小さいので行った)、`bindings/godot/Cargo.lock` (`ugoe-core` の直接の依存に `libm` が増えた。godot のビルドは `--locked`)。
- やらなかったこと: `ugoe-format`・`ugoe-ffi` (`ugoe-ffi` に `f32::sqrt` が 1 つある。IEEE で値は同じ) には `clippy.toml` を置いていない (T0182・T0183/T0491 で no_std にするときに `MATH_CRATES` に足す)。
  `ugoe-model` はすでに no_std なので std の数学関数を呼べない。ツール層 (`ugoe-render`・`ugoe-mesher` など) の `f32::sin` などはそのまま (glam の中だけ libm になる)。
- 起票の候補: なし。
