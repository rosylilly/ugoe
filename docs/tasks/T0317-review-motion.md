---
id: T0317
title: コードレビュー: ugoe-motion
status: done
milestone: M9
crates: [ugoe-motion, docs]
depends_on: []
assignee: claude-worker-T0317
---

# T0317: コードレビュー: ugoe-motion

## 目的

ugoe-motion のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-motion` のすべて

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - `docs/spec/motion.md` との一致 (カーブの区間・ループ・フェード・優先度)
  - 目パチ・呼吸・リップシンク・ポーズ・トラッキングの対応づけの境界、乱数と決定性
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0323〜T0327 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)。使ったのは T0323・T0324

### 所見

範囲 (`crates/ugoe-motion` の `curve.rs`・`player.rs`・`manager.rs`・`expression.rs`・`effects.rs`・`pose.rs`・`tracking.rs`・`lib.rs` とテスト) をすべて読んだ。
仕様 (`docs/spec/motion.md`・`spec/motion.md`・`docs/spec/avatar-profile.md#トラッキングの対応の上書き`) と照らした。カーブの区間・ループ・フェード・優先度・
目パチの乱数 (SplitMix64 と上位 24 ビット)・呼吸・リップシンク・ポーズ・トラッキングの対応づけの式は仕様と一致していた。食い違いは下の NaN の扱いだけ。

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 高 | `src/player.rs` `MotionPlayer::advance` | ループするモーションで、NaN の `dt`・巨大な `dt` (1e30 など)・極端に短い `duration` (1e-30)・NaN の `duration` を与えると、周回のループから抜けない (残りの時間が NaN のまま、または桁落ちで減らない)。`WebPuppet::update` は `dt` を検査しないので、JS から NaN が来ると固まる | 直した (`dt` が正の有限値でなければ何もしない。NaN の長さは 0 と同じ。1 回の周回を 1000 までにし、超えた分は剰余で位置を合わせる。テストを追加) |
| 高 | `src/player.rs` `MotionPlayer::advance` | NaN の `dt` で、ループしないモーションが `dt.min(end - elapsed)` により終わりまで飛んで終わる | 直した (同上) |
| 高 | `src/effects.rs` `EyeBlinkPlayer::advance` | NaN・無限大・巨大な `dt`、または極端に短い `interval` と長さ 0 の段階で、段階のループから抜けない。`interval` が NaN でもまばたきを続けた | 直した (`dt` の検査、`interval` は `> 0` でなければ止める、1 回の段階を 4000 までにして超えた分は捨てる。テストを追加) |
| 高 | `src/effects.rs` `BreathPlayer::advance` | NaN か無限大の `dt` で位相が NaN になり、以後ずっと呼吸の対象が NaN になる | 直した (`dt` の検査。テストを追加) |
| 中 | `src/player.rs`・`src/expression.rs` | 経過時間が `f32` で、何日もループさせた (60fps で約 6 日、144fps で約 1.5 日) 後は `dt` を足しても値が変わらず、`stop()` したモーション・表情が終わらない (フェードアウトの途中で残り続ける)。T0316 の物理の累積と同じ形 | 直した (経過時間と終了時刻を `f64` で持つ。公開の `elapsed()` は `f32` のまま。テストを追加) |
| 中 | `src/effects.rs` `LipSyncPlayer::set_level` | NaN の音量は `clamp` を通っても NaN で、対象のパラメータに NaN を書く (`WebPuppet::set_lip_sync_level` は検査しない) | 直した (NaN は `None` と同じ。仕様にも書いた。テストを追加) |
| 中 | `src/tracking.rs` `Formula::MouthOpenY` | `f32::max` が NaN を捨てるので、`jawOpen` が NaN のとき 0 を書いた (仕様は有限値でなければ書かない) | 直した (テストを追加) |
| 中 | `ugoe-model`・`ugoe-format` | モーション・表情の数値 (点・長さ・フェード・イベントの時刻・表情の値) をどこでも検証しない。MessagePack の NaN がそのままパラメータに入る | T0323 を起票 |
| 中 | `src/manager.rs` `effect_group_weight`・`part_opacities` | `curve_values()` が対象で絞る前にすべてのカーブを評価するので、パラメータのカーブを毎フレーム 4 回評価していた | 直した (対象で先に絞る `curve_values_where` を crate 内に足した) |
| 中 | `src/curve.rs` `evaluate` | 毎フレーム先頭から線形にセグメントを探す (未知のセグメントの検査も毎回)。長いカーブで性能の目標に響きうる | T0324 を起票 |
| 中 | `src/curve.rs` のテスト | セグメントが空・最初が `start` でない・2 つ目の `start` (仕様の例) のテストが無かった。`player.rs` に長さ 0 のモーションのテストが無かった | テストを追加 |
| 低 | `src/player.rs`・`expression.rs`・`manager.rs`・`effects.rs`・`pose.rs` | 公開の `new`・`config` などに doc コメントが無かった | 足した |
| 低 | `src/manager.rs` `part_opacities` | 毎フレーム `Vec` を確保し、パーツを線形に探す | 見送り (パーツの不透明度のカーブは少なく、呼び出し側 (`Puppet`) も `Vec` を受け取る API) |
| 低 | `ugoe-ffi` と `ugoe-wasm`・`ugoe` の違い | C ABI は NaN の音量と NaN の `dt` をエラーにし、`Puppet`・`WebPuppet` は (T0317 の後) 無視する | 見送り (どちらも安全。C ABI はエラーを返せるので今のままでよい。横断に書いた) |

### 仕様の更新

- `docs/spec/motion.md`・`spec/motion.md`: `dt` が正の有限値でなければモーション・表情・目パチ・呼吸は何も進めない。NaN の音量は音量なし。
  ランタイムは 1 回の進めで扱うループの周回と目パチの段階に上限を設けてよい (公開仕様は値を定めない。参照実装は 1000 周と 4000 段階)。
  ループするモーションの長さが NaN なら長さ 0 と同じ。経過時間を `f64` で持つこと (日本語版だけ)。

### 横断 (T0008 向け)

- **時間の累積を `f32` で持つ箇所:** 何日も動かすスタジオでは、`elapsed += dt` 型の累積が 2^19 秒前後で進まなくなる。周期で折り返すもの (呼吸) は安全。
  T0316 の物理と T0317 のモーション・表情は直した。ほかのクレート (アプリのアニメーション、`ugoe-headless` の時刻など) も `f64` か折り返しにする。
- **`while`/`loop` で時間を消費するループ:** 1 回で引く量が残りに比べて小さいと桁落ちで止まらない (巨大な `dt`・極端に短い長さ・NaN)。回数の上限を必ず持たせる。
- **NaN の通り道:** `f32::max`/`min`/`clamp` は NaN を捨てたり残したりする (`max(NaN, 0) = 0`、`clamp(NaN) = NaN`)。NaN を含めて弾くには `x.is_nan() || x <= 0.0` か `!(x > 0.0 && x.is_finite())` と書く (単独の `!(x > 0.0)` は clippy が拒む)。
  外から来る `dt`・音量・トラッキングの値は、`ugoe-ffi` では入口で検査しているが、`ugoe`・`ugoe-wasm` では検査しない。下のクレートで安全にするか、入口の方針をそろえる。


- `just ci` は通った (exit 0)。ランタイム層を触ったので `just wasm-check` も通した。
