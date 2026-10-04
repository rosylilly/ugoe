---
id: T0433
title: wasm の入口の NaN と dt の扱いを揃えて書き、回帰のテストを足す
status: done
milestone: M9
crates: [ugoe-wasm, docs]
depends_on: [T0424, T0137, T0139]
assignee: claude-worker-T0433
---

# T0433: wasm の入口の NaN と dt の扱いを揃えて書き、回帰のテストを足す

## 目的

T0424 (ugoe-wasm のレビュー) で、JS から有限でない値を渡したときの振る舞いを確かめた (main の 05c82cf の時点):

| 入口 | 振る舞い |
| --- | --- |
| `update(dt)` に NaN・`Infinity` | **戻らない** (`Puppet::update` が終わらない。タブが固まる) |
| `set_parameter(id, NaN)`・`set_lip_sync_level(NaN)` | 次の `update` / 評価で **パニック** (`keyform::locate`)。wasm ではモジュールが壊れる |
| `set_tracking` の NaN | 書き込まない (仕様どおり) |
| `follow` の NaN | 例外 (SDK が検査する) |
| `set_view`・`pan_by`・`zoom_at`・`set_background` の NaN | 何も変えない (`zoom_at` の位置と `set_background` は T0424 で直した) |
| `sample_motion_json(json, NaN)` | 終わりの値を返す |

前の 2 つは SDK とランタイムで直る (T0137: パラメータへの NaN の書き込みを無視する。T0139: 負・有限でない `dt` を 0 として扱い、
`follow` は有限でないリップシンクとパーツの不透明度を誤りにする)。どちらも main に入った後で、wasm の入口ごとの振る舞いを
README に表で書き、`WebPuppet` が使う `Puppet` の経路 (`crate::animation` などネイティブでテストできる所) に回帰のテストを足す。

C ABI は同じ入力を `UGOE_ERROR_INVALID_ARGUMENT` で拒む (`ugoe_puppet_update`・`ugoe_puppet_set_parameter`・`ugoe_puppet_set_lip_sync_level`)。
wasm は無視する (JS の `void` の関数で例外を増やさない) という違いを、README に理由と一緒に書く。揃えるべきと判断したら、その変更は別のタスクにする。

## 受け入れ条件

- [x] T0137・T0139 が main に入っている
- [x] `update(NaN)`・`update(Infinity)`・`update(-1)` が戻り、状態が有限のままであることをテストする (物理のあるフィクスチャで)
- [x] `set_parameter` 相当の NaN の書き込みと `set_lip_sync_level(Some(NaN))` の後の `update` で、値と頂点が有限のままであることをテストする
- [x] `crates/ugoe-wasm/README.md` に入口ごとの NaN・無限大・負の `dt` の扱いを書く (C ABI との違いを含む)
- [x] `just ci` が通る

## 参考

- `crates/ugoe-wasm/src/web.rs`・`src/web/follow.rs`・`src/web/tracking.rs`・`src/web/animation.rs`
- `spec/evaluation.md#parameter-values`・`spec/physics.md` (dt の扱い)・`spec/avatar-profile.md` (トラッキングの有限でない値)

## メモ

### やったこと

- T0137・T0139 (と T0155・T0158) が main で `done` なのを確かめた。wasm の入口は SDK の扱いをそのまま出すので、コードの挙動は変えていない。
- `WebPuppet::set_parameter` の中身を `load::set_parameter` に移し、ネイティブでテストできるようにした (振る舞いは同じ)。
- 回帰のテスト (`src/animation.rs`):
  - `update_with_non_finite_dt_returns_and_stays_finite`: `physics_hair` と物理の設定、ループするモーションで、途中に NaN・`+∞`・`-∞`・`-1` の
    `update` を挟んでも戻り、パラメータと頂点が有限で、挟まない場合とビット列で同じになる (0 とみなす)。
  - `non_finite_parameter_and_lip_sync_writes_keep_state_finite`: リップシンクを物理の入力 `AngleZ` に当て、`set_parameter` の NaN (値は変わらず
    `true`)・`set_lip_sync_level(Some(NaN))` の後の `update` で値と頂点が有限。無限大は値域の端・0..1 にクランプする。
  - `motion_json_is_sampled_without_fades` に `sample_motion_json` の NaN・無限大の時刻を足した。
- `README.md` に「有限でない値と `dt`」の節 (入口ごとの表と C ABI との違いの理由) を書いた。`update`・`set_parameter`・`set_lip_sync_level` の doc にも書いた。

### やらなかったこと

- C ABI と揃える変更はしていない。wasm は無視・C ABI は拒む、の違いは README に理由を書いて残した (揃える必要は無いと判断)。
- ブラウザ上の実際の `WebPuppet` での確かめはしていない (ネイティブでテストできる経路だけ)。

### 起票の候補

- なし。

### 検査

- `just ci` と `just wasm-check` が通った (`git rebase main` の後)。
