---
id: T0424
title: コードレビュー: ugoe-wasm
status: done
milestone: M9
crates: [ugoe-wasm, docs]
depends_on: []
assignee: claude-worker-T0424
---

# T0424: コードレビュー: ugoe-wasm

## 目的

ugoe-wasm のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-wasm` のすべて

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - JS へ出す API の一貫性・型 (TypeScript の宣言)・エラーの形 (T0124・T0125)
  - メモリ (解放の口・コピーの回数)、`@ugoe/runtime` (bindings/web) とのずれ (bindings/ は直さず起票する)
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0433〜T0437 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 結果

`crates/ugoe-wasm` のすべて (`src/` の 22 ファイル、テストを含む) と README を読んだ。比べた相手: C ABI (`ugoe-ffi`) の入力の検査、
`bindings/web` (`src/runtime.ts`・`src/avatar.ts`・`stub/ugoe_wasm.d.ts`)、`@ugoe/player` の読み込み。
NaN は実際に確かめた (ネイティブで `Puppet` に同じ値を渡すテストを一時的に書いて走らせた。`update(NaN)` はテストが 30 分以上戻らなかった)。

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 高 | `src/web.rs` の `update` (`Puppet::update`) | `update(NaN)`・`update(Infinity)` が戻らない (タブが固まる)。JS の `(now - last) / 1000` の誤りでも起きる | SDK で直る (T0139 が負・有限でない `dt` を 0 として扱う)。wasm で二重に守らない。回帰のテストと README は T0433 |
| 高 | `src/web.rs` の `set_parameter`・`src/web/animation.rs` の `set_lip_sync_level` | NaN を渡すと次の評価でパニック (`keyform::locate`) し、wasm のモジュールが壊れる | ランタイムで直る (T0137 が NaN の書き込みを無視する)。回帰のテストは T0433 |
| 高 | `src/load.rs` の `texture_count` | JSON のメッシュの `"texture": 4294967295` で、検証の前に約 43 億枚のプレースホルダを確保しようとする (タブが落ちる) | 直した (`model.textures` の数だけにした。範囲外の番号は `Puppet::new` の検証が誤りにする。失敗するテストを先に追加) |
| 高 | `src/load.rs` の `decode_png` | PNG の見出し (IHDR) の大きさだけで `vec![0; output_buffer_size()]` を確保する。IHDR が 65535x65535 で IDAT が数バイトの PNG で約 17GB を求める (png の `Limits` はデコーダの中のバッファにしか効かず、デバイスの上限との比較はデコードの後)。T0212 (ugoe-headless のレビュー) から伝わった | 直した (確保の前に一辺を `MAX_IMAGE_SIZE` = 16384 と比べる。headless と同じ上限。失敗するテストを先に追加) |
| 中 | `src/view.rs` の `ViewState::zoom_at` | 位置が NaN だとパンが NaN になり、`fit` を呼ぶまで何も描けない (`set_view`・`pan_by` は有限でない値を無視していた) | 直した (位置が有限でなければ何も変えない。失敗するテストを先に追加) |
| 中 | `src/web.rs` の `set_background` | NaN をクランプせずにクリア色に入れる (描画の結果が決まらない) | 直した (`view::background_color` に分けてテストし、NaN なら何も変えない。`set_view` と同じ) |
| 中 | `src/web.rs` の `create` | canvas がデバイスのテクスチャの上限より大きいと、`resize` と違って上限に収めずにサーフェスを作る (検証のエラー) | 直した (`view::surface_size` に分けてテストし、`create` と `resize` で使う) |
| 中 | `src/web.rs` の `load_json`・`load_bundle` | 失敗が英文だけの `JsError` で `code` が無い。ビューアがその英文を利用者に見せている (ADR 0029) | T0434 を起票 |
| 中 | 入口の NaN の扱い全体 | C ABI は NaN・有限でない `dt` を `UGOE_ERROR_INVALID_ARGUMENT` で拒むが、wasm は無視する (`follow` だけは SDK の検査で例外)。どちらとも README に書いていない | T0433 を起票 (上の 2 つの回帰のテストと一緒に) |
| 中 | `bindings/web/stub/ugoe_wasm.d.ts` ほか 4 か所 | `WebPuppet` の型の手書きの写しが 5 か所にあり、`bindings/web` の写しに `avatar_slots_json` が無い | T0435 を起票 (bindings/ は直さない) |
| 低 | `src/web.rs`・`src/web/animation.rs`・`src/web/tracking.rs` | `has_model`・`width`・`height`・`zoom`・`pan_x`・`pan_y`・`is_motion_playing`・`has_physics`・`is_motion_over_tracking` に doc コメントが無い | 直した |
| 低 | `src/web.rs` の `unload` | モデルを外してもレンダラに登録したテクスチャは次の読み込みまで残る (GPU のメモリ) | 見送り (次の `load_*` で置き換わる。外したまま長く置く使い方は今は無い) |
| 低 | `src/web.rs` の `load` | `Wardrobe::load_base` のテクスチャの登録に失敗すると、本体のテクスチャは新しいものに変わったまま前の `Puppet` が残る | 見送り (大きさは先に `check_texture_sizes` で検査するので、登録の失敗はデバイスを失ったときくらい) |
| 低 | `src/web/tracking.rs` の `set_tracking` | 毎フレーム、ブレンドシェイプの名前 (約 52 個) を JS の文字列から `String` に写す | 起票の候補 (C ABI のように決まった順の `Float32Array` を受ける口を足す。公開 API の追加なので今は見送り) |
| 低 | `src/lighting.rs` | JSON の大きすぎる数 (`1e300`) は `f32` の無限大になり、向き・位置・色の検査を通る | 見送り (レンダラが有限でない光源を捨てる。`spec/rendering.md#ライティング`) |
| 低 | `src/animation.rs` の `sample_motion_json` | `t` が NaN だと終わりの値を返す | 見送り (タイムラインのシークだけで使い、結果が有限なので害が無い。T0433 の表に書く) |

起票: T0433 (wasm の入口の NaN と dt の扱いを揃えて書き、回帰のテストを足す)、T0434 (wasm の読み込みのエラーにコードを付ける)、
T0435 (ugoe-wasm の手書きの型の写しを生成物と比べる)。枠の T0436・T0437 は使っていない。

確かめたが問題の無かったこと: JS へ出す API の型 (`Option` → `undefined`、`Vec<f32>` → `Float32Array`、`Result` → 例外) は一貫している。
エラーの形は `DressUpError` (`name`・`code`・補足の値) と T0124・T0125 の警告の構造が README と一致する。メモリはどの戻り値も JS へ写す
(`parameter_values`・`drawable_positions` は毎回 `Vec` を作る) だけで、解放の口は wasm-bindgen の `free()` (`<ugoe-avatar>` は外すときに呼ぶ)。
`read_pixels` は待ちに上限があり、デバイスを失えば誤りで終わる。`unsafe`・`unwrap()` (テスト以外) は無い。

`just ci` と `just wasm-check` は、`decode_png` の修正を足した後にも通った。

### 横断

- **NaN・有限でない値の方針が入口ごとに違う。** C ABI は拒む、wasm は無視する、SDK の `follow` は誤り、`update` は 0 として扱う (T0139)。
  バインディング (wasm・C ABI・Godot・Unity) で「ホストから来た数」の扱いを 1 つの表にまとめ、仕様 (`spec/`) に書くとよい。
- **外から来た画像の見出しの大きさで確保する。** `decode_png` は headless の写しで、同じ問題が両方にあった (T0212 と T0424 で直した)。PNG のデコードを 1 か所 (ランタイム層で使えるクレート) にまとめれば、上限も 1 つで済む。
- **検証の前にモデルの値で確保する。** wasm の `texture_count` と同じ形 (メッシュの `texture` の最大 + 1) が `ugoe-ffi/src/puppet.rs` の `texture_count`、
  `ugoe-headless/src/textures.rs` の `texture_count`、`bindings/godot/src/model.rs` の `texture_count` にある。JSON のモデルを検証の前に数えるなら同じ問題が起きる。
- **wasm の型の手書きの写しが 5 か所** (T0435)。公開 API を足すたびに漏れる。
- **ホストに見せる失敗に `code` が無い所が残る** (wasm の読み込み。T0434)。着せ替えと警告は T0124・T0125・T0412 でコードになったが、読み込みの失敗は文だけ。
