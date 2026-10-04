---
id: T0139
title: コードレビュー: ugoe (ランタイム SDK)
status: done
milestone: M9
crates: [ugoe, docs]
depends_on: []
assignee: claude-worker-T0139
---

# T0139: コードレビュー: ugoe (ランタイム SDK)

## 目的

ugoe (ランタイム SDK) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe` のすべて (Puppet・着せ替え・ホストの操作)

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - 公開 API の一貫性と doc コメント (crates.io に出す面)
  - 各層 (runtime・physics・motion・render) の呼び順と `dt` の扱い、エラーコード (T0412) の網羅
  - ffi・wasm・各バインディングと重複している処理
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0155〜T0159 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

読んだもの: `crates/ugoe` のすべて (`src/lib.rs`・`follow.rs`・`dress_up.rs` (+ `tests.rs`)・`warning.rs`・`lighting.rs`・`tests/m3_puppet.rs`・`tests/spec_timelines.rs`)、
README、docs/spec/motion.md (#フレーム内の適用順・#受け手として進める)・modules.md (#puppet-での着せ替え)。重なりの確認に ugoe-ffi・ugoe-wasm・スタジオの `Puppet` の呼び出し側。

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 高 | `src/lib.rs` `Puppet::update` | `dt` が `+inf` だとループするモーションの `advance` が終わらない (止まる)。NaN だとモーション・目パチ・呼吸の時刻が NaN になり戻らない。C ABI は拒み、スタジオはクランプするが、wasm の `WebPuppet::update` はそのまま渡す | 直した (負・非有限の `dt` を 0 とみなす `sanitize_dt` を `update`・`follow` に。先に止まる/NaN になるテストを書いた。docs/spec/motion.md に追記)。根は T0158 |
| 中 | `src/dress_up.rs` `set_modules` | 初めての付け外しが失敗しても本体 (`Wardrobe`) を覚えてしまう (「失敗したら何も変えない」に反する)。以後の `set_physics` が `base_physics()` に届かず、次の付け外しは古い本体の物理から合成する (wasm の読み直しも古い設定を引き継ぐ) | 直した (成功したときだけ覚える。先に失敗するテストを書いた) |
| 中 | `src/follow.rs` `Puppet::follow` | リップシンクの強さ・音量、パーツの不透明度の値・重みは NaN だけを拒み、無限大を通す。強さが無限大で音量と今の値が等しいと `lerp` が `0 × ∞` で NaN をパラメータに書く (T0423 の再現から伝えられた) | 直した (有限でないものを拒む。先に失敗するテストを書いた。docs/spec/motion.md に追記)。C ABI は T0423 が境界で拒む |
| 低 | `src/follow.rs` `ParameterValuesError::LipSyncNaN`・`PartOpacityNaN` | 上の修正で無限大も含むようになり、名前が意味より狭い | 見送り (改名は公開 API の変更。doc と文を直した。1.0 の前にまとめて直すなら T0008 で扱う) |
| 中 | `src/dress_up.rs` `DressUpError::code`・`fields` | 組み込み先が照らし合わせる値なのに、テストが 16 種のうち 2 種しか固定していない | 直した (すべての種類の `code` と `fields` を固定するテスト) |
| 中 | `ugoe-motion` `LipSyncPlayer::set_level` (`Puppet::set_lip_sync_level`) | NaN の音量が `clamp` を通り、口が NaN になる。指揮役の `lip_sync_applied()` が NaN になり、受け手の `follow` がすべて拒む | T0155 を起票 |
| 中 | `ugoe-motion` `MotionPlayer::advance` | 有限でも巨大な `dt` (`duration` が `dt` の ulp の半分未満) で終わらない。`ugoe-motion` を直接使うと `+inf`・NaN も残る | T0158 を起票 |
| 低 | `src/dress_up.rs` `DressUpError::fields` | `ComposeError` の `mask`・`rig`・`collider`・`part`・`texture`・レベル、検査のエラーを落としている (利用者に分かる文が作れない) | T0157 を起票 |
| 低 | `src/lib.rs` (クレートの doc) | 「`render` feature 相当」と書くが feature は無い。`ugoe` は wgpu に無条件に依存する (独自のレンダラの組み込み先も wgpu をビルドする) | 文言を直した。feature にするかは T0156 を起票 |
| 低 | `src/lib.rs`・`dress_up.rs`・`follow.rs`・`warning.rs` | 公開の関数・`Toggles` の各段・`DressUpError` とその種類・エラーと警告の各値に doc コメントが無い (crates.io に出す面) | 直した (doc を足し、`#![warn(missing_docs)]` を入れた) |
| 低 | `src/lib.rs` | `motions()` はあるが `expressions()` (不変の参照) が無い | 見送り (API の追加。要る組み込み先が出たら足す) |
| 低 | `src/lib.rs` `apply_part_opacities` | 毎フレーム `requests` と `curves` の `Vec` を確保する | 見送り (要素はカーブのあるパーツの数だけで、1 体 1ms の目標に効かない) |
| 低 | テスト (`follow.rs`・`dress_up/tests.rs`) | 浮動小数点の完全一致の比較 (pedantic の `float_cmp`) | 見送り (引き継いだ値・触らない値・クランプした値で、ビット一致が保証される箇所) |
| 低 | `ugoe-wasm` `load::carry_over_parameters` | `ParameterValues::carry_over` (runtime) と同じ処理を自前で持つ | 見送り (範囲外。T0424 へ横断に書く) |

確かめたこと (直すものが無かった):

- **呼び順と `dt`:** `update` は docs/spec/motion.md の 1.〜11. のとおり。`follow` は 8.〜11. を同じ順で通し、2.〜7. の状態も時刻も進めない。
  `dt` は表情・目パチ・呼吸・物理・ポーズへ同じ値を渡す (今回、負・非有限を 0 にそろえた)。
- **エラーコード (T0412):** `DressUpError::info` は `ComposeError` の全種類をワイルドカード無しで引くので、種類が増えればコンパイルで気づく。
  `LoadWarning::code` も全種類。wasm・C ABI は自前の表を持たずにこれを使っている。
- **ffi・wasm との重なり:** 付け外し・受け手・警告の code は `ugoe` に一本化されている。重なりは `dt` の検査 (C ABI は拒む、スタジオは `MAX_DT` に
  クランプ、wasm は無し → `Puppet` で 0 とみなすようにした) と、wasm の `carry_over_parameters` だけ。

### 横断 (T0008 向け)

- **NaN と非有限の値の入口:** 1 回の NaN が基準値や再生の時刻に入ると戻らない。`ParameterValues::set` は NaN をクランプで落とさず通す
  (ホストが `parameters_mut().set(i, NaN)` すると基準値に残り続ける。runtime、T0137)、`LipSyncPlayer::set_level` (T0155)、各プレイヤーの `advance` の
  `dt <= 0.0` の検査 (NaN を通す。T0158)。`if x <= 0.0 { return }` 型の検査は NaN を通すので、`!(x > 0.0)` か `is_finite` で書く。
- **入口ごとの検査の重複:** 同じ前提 (`dt` が有限で負でない) を C ABI・wasm・スタジオがそれぞれ別の扱い (拒む・何もしない・クランプ) で持っていた。
  前提は下の層 (`Puppet`) で決め、上の層は表し方 (エラーにするか) だけを持つ。
- **固定の値のテスト:** 組み込み先と照らし合わせる文字列 (エラーの code・キー) は、全種類を表で固定するテストがあると、変えたときに気づける。
- **`missing_docs`:** crates.io に出すクレートは `#![warn(missing_docs)]` を入れると、doc コメントの不足を CI で止められる (`ugoe` は入れた)。

`just ci` と `just wasm-check` は通った (2026-10-04、main の 05c82cf の上。`follow` の有限の検査を足した後にも流し直した)。

