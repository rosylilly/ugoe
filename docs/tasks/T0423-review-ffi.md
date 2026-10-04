---
id: T0423
title: コードレビュー: ugoe-ffi
status: done
milestone: M9
crates: [ugoe-ffi, docs]
depends_on: []
assignee: claude-worker-T0423
---

# T0423: コードレビュー: ugoe-ffi

## 目的

ugoe-ffi のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-ffi` のすべてと、生成するヘッダ

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - すべての `unsafe` に `// SAFETY:` があるか、NULL・長さ・アライメント・所有権と解放の対
  - 境界を越えるパニック (`catch_unwind`)・スレッドの前提・エラーコードの網羅
  - ヘッダ (`just ffi-header`) と実装・Unity の C# の宣言とのずれ (bindings/ は直さず起票する)
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0428〜T0432 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 高 | `ugoe-motion` `EyeBlinkPlayer::advance` (C ABI からは `ugoe_puppet_update`) | 巨大な有限の `dt` (1e20 など) で `dt` が減らず無限ループ。C ABI の `dt` の検査 (有限・非負) を通り、返ってこない (`pose_and_effects` で確認) | T0428 を起票 (T0317 と重なればそちらで)。オーケストレーターにも連絡した |
| 高 | `src/follow.rs` `ugoe_puppet_follow` | 当てるリップシンクの強さが無限大だと `blend` が `0 × ∞ = NaN` をパラメータに書く (`MouthOpenY` が NaN になるのを確認)。パーツの不透明度の値と重みの無限大も通していた | 直した (境界で有限でなければ `UGOE_ERROR_INVALID_ARGUMENT`。先に失敗するテストを足した)。SDK と wasm は T0429 を起票 |
| 中 | `tests/` | NaN の入口のテストの穴: `ugoe_puppet_set_parameter`・`ugoe_puppet_set_lip_sync_level` の NaN、作る関数の `out` が NULL、`attach_module` の差し替え (同じスロット・同じ名前・両方に当たる場合) が `Puppet::attach_module` と同じか | テストを足した (`api.rs`・`extended.rs` の `attaching_replaces_modules_like_the_puppet`) |
| 中 | `bindings/unity` `Native.cs` | ヘッダと C# の構造体の並び・型は一致。ただし 9 つの関数 (`follow`・`set_base_values`・トラッキング優先の 3 つ・本体の物理の 2 つ・`part_evaluated_opacity`・`positions`) の宣言が無い | T0430 を起票 (T0426 と重なればそちらで) |
| 低 | `src/puppet.rs` `ugoe_puppet_set_parameters_by_id` | UTF-8 でない ID のエラーが何番目かを示さず、文字どおり `` `ids[i]` `` と出ていた | 直した (エラーのときだけ番号を文字列にする。テストを追加) |
| 低 | `src/drawable.rs` | `UGOE_CHANGED_*` は `DrawableChanges::bits()` をそのまま渡し、頂点位置と UV は `Vec2` の並びを float の並びとして貸すが、どちらも一致を確かめていなかった | 直した (コンパイル時の `assert!`) |
| 低 | `src/error.rs` `UGOE_ERROR_BUFFER_TOO_SMALL` | 「何も書き込んでいない」とあるが、可変長の結果では要る数・長さを書く | 直した (doc とヘッダ) |
| 低 | `src/lighting.rs` `light_uniform` | 平行光源の長さを f32 の 2 乗和で求めるので、有限で 0 でない向きでも 1e30 や 1e-30 の大きさでは「有限でない・0」として拒む。また検査と変換が `ugoe-render` の `LightRaw::new` と重複 | 見送り (参照レンダラも同じ式で同じ光源を捨てるので、C ABI だけ変えると食い違う。直すなら T0211 で両方を) |
| 低 | `src/dress_up.rs` `ugoe_puppet_attach_module` | 差し替えの規則が `ugoe::Puppet::attach_module` と重複 | 見送り (画像の `ModuleData` を同じ並びで持つため。上のテストで一致を確かめる) |
| 低 | 全体 | 引数の `bool` (`set_stage_enabled` など) は、ホストが 0・1 以外のバイトを渡すと Rust では未定義動作 | 見送り (C の `_Bool` と同じ ABI。Unity は `byte` で 0・1 を渡している。変えるなら ABI の版を上げる変更) |
| 低 | 全体 | 数を `len() as u32` で返す (2^32 を超えると切り捨て) | 見送り (現実のモデルでは起きない) |
| 低 | `src/error.rs` | `ugoe_last_error*`・`ugoe_version` などは `guard` を通らない | 見送り (パニックしうるのはスレッドの終了中の TLS だけ) |

確かめたこと: すべての `unsafe` に `// SAFETY:` がある。NULL と長さ (`count` 0 なら NULL を許す) は一貫している。生成するポインタは `Box::into_raw` と `Box::from_raw` の対で、
`destroy` も `guard` の中で落とす。公開する関数はバージョンと最後のエラーを除いて `guard` を通る。ヘッダは `just ffi-header` と一致し、`just ffi-check` (C の見本) が通る。

NaN の入口 (T0137 の補足への答え): パラメータ (`set_parameter`・`set_parameters_by_id`・`set_base_values`・`follow` の値) と音量
(`set_lip_sync_level`) は NaN を拒み、無限大は範囲に丸める。トラッキングは頭とブレンドシェイプの非有限を拒み、体の NaN は省略。
光源は非有限を拒む。`follow` の強さとパーツの不透明度は今回から非有限を拒む。C ABI にはパーツの不透明度やオフセットを直接書く口は無い
(パーツの不透明度は `follow` だけ)。`dt` は `update`・`follow` とも非有限と負を拒むが、巨大な有限の値で止まらない (T0428)。

### 横断

- 「NaN だけを拒み、無限大は通す」検査は、無限大が `0 × ∞` や `∞ − ∞` で NaN になる経路 (`blend`・`lerp`) を持つ値では足りない。
  SDK・wasm・ほかの受け口でも、重みや強さは有限かを見るのがよい (T0429)。
- 段階を `dt` で進めるループ (`while dt > 0 { dt -= len }` の形) は、f32 で巨大な `dt` が減らずに止まらない。ほかの再生器・物理の分割にも同じ形が無いか (T0428)。
- 参照レンダラとバインディングが同じ変換 (光源のユニフォーム) を別々に持っている。片方だけ直すと食い違う。
- バインディング (Unity など) が C ABI の新しい関数に追いついているかを機械で確かめる仕組みが無い (ヘッダと `Native.cs` の関数名の差分は簡単に取れる)。

### 結果

- `just ci` は通った (rebase 後、前面で実行)。`just ffi-check` も通った。
- 起票: T0428 (目パチの巨大な dt)・T0429 (SDK と wasm の follow の非有限)・T0430 (Unity の宣言の不足)。T0431・T0432 は使っていない。
