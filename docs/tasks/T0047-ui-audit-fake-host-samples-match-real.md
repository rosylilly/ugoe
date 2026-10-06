---
id: T0047
title: "ui-audit: 偽のホストの見本 (スタジオのポーズのグループ・プロファイルの拡張子・AI の提案の種類) を本物にそろえる"
status: done
milestone: M10
crates: [tests, docs]
depends_on: [T0045]
assignee: claude-worker-T0047
---

# T0047: ui-audit: 偽のホストの見本 (スタジオのポーズのグループ・プロファイルの拡張子・AI の提案の種類) を本物にそろえる

## 目的

`just ui-audit` の偽のホストが返す見本のデータが、本物のアプリの見本とずれている (T0919 で本物の WebView と見比べて分かった。T1643)。
ずれていると、本物では出る画面 (ポーズのパネルなど) を監査が撮らず、本物では出ない画面 (表に無い種類の出し方) を撮る。

- スタジオ: 偽のホストの見本のプロファイルに、本物の見本のポーズのグループが無い・プロファイルのファイルの拡張子が本物と違う (T0919)。
- エディタ: `tests/ui-audit/lib/editor-fixtures.mjs` の AI の提案の見本が、`ugoe_editor_core::Command` に無い種類 `set_keyform_bindings` を使っている。
  そのため提案のカードの「採用で変わるもの」が、利用者の言葉ではなく表に無い種類の出し方 (`<code>`) で出る (T1643)。

## 受け入れ条件

- [x] 偽のホストの見本と本物の見本 (スタジオの `apps/studio/scripts/sample-profile.mjs`・`ugoe sample`・エディタの見本) の違いを洗い出し、
      メモに一覧を書いた。直さないものは理由を書いた
- [x] スタジオの偽のホストの見本に、本物と同じポーズのグループと拡張子を持たせた
- [x] `editor-fixtures.mjs` の提案の見本が、実在する種類だけを使う (例: `set_keyform_cell`)。表に無い種類の出し方を撮る場面が要ると判断したら、
      別の場面として足した (要らないなら理由をメモに書く)
- [x] 見本が実在する種類だけを使うことを確かめる検査 (`hosts.test.mjs` などで、`crates/ugoe-editor-core/src/command.rs` の種類と比べる) がある
- [x] tests/ui-audit/README.md の偽のホストの説明を直した
- [x] `just ui-audit --only editor,studio` で、増えた・変わった指摘を `--compare` で確かめ、メモに書いた
- [x] `just ci` が通る

## 参考

- T0919 のメモ (起票の候補)・docs/roadmap.md の M10 の「後続の候補 (T0919)」、T1643 のメモ (`set_keyform_bindings`)
- tests/ui-audit/lib (editor-fixtures.mjs・studio-host.mjs・editor-host.mjs)、tests/e2e/README.md#ヘッドレスの-chrome-just-ui-audit-との違い

## メモ

- T0046 で起票した (元の候補: T0919 (偽のホストの見本を本物に)・T1643 (`set_keyform_bindings`))。
- T0045 (作業中) が同じ tests/ui-audit/lib を触るので、その後にした。

### やったこと

- 偽のホストの見本と本物の見本の違い (洗い出した一覧):
  | アプリ | 違い (前) | 本物 | 直したか |
  | --- | --- | --- | --- |
  | スタジオ | プロファイルの場所 `assets/sample-profile.ugoestudio` | 拡張子は `json` (lib.rs の `PROFILE_EXTENSION`)。マニュアルの例は `ugoe-sample-profile.json` | 直した (`assets/ugoe-sample-profile.json`) |
  | スタジオ | 「雑談」のポーズのグループが偽物 (`ArmsDown`/`ArmsUp`/`Wave` とモジュール `Hat` の 2 つ) | avatar.ugoe は `earwear` / `no_earwear` の 1 つ。選択と表示は先頭 (`earwear`) | 直した |
  | スタジオ | パラメータ 6 つ (名前は `Angle X` など) | 27 (sample.rs の `PARAMETERS`。名前は ID、値域は標準の値) | 直した |
  | スタジオ | 表情 3 つ (`angry` が無い)、モーションのフェードが全部 0.2 / 0.3 | 表情 4 つ、`idle` は 0.5 / 0.5 | 直した (sample/animations の JSON から読む) |
  | スタジオ | 何も再生していない (`current`・`idleMotion` が null) | 既定の設定 (`autoIdle`) では `idle` を待機として流す | 直した (「Playing idle (Idle)」が出る) |
  | スタジオ | 最初のプロファイルの通知が `dirty: true` | 開いた直後は `false` | 直さない: 「保存していない」の印を撮るための状態 (場面を減らさないため) |
  | スタジオ | 着せ替えのシーン (dress-up.ugoe の `HandsDown` / `Wave` と Sailor の追従) | — | 直さない: 偽のホストは配信中のシーン (雑談) だけを送り、シーンの切り替えは何もしない (場面が無い) |
  | エディタ | AI の提案の見本が実在しない種類 `set_keyform_bindings` を使う | rig.rs は `create_warp_deformer`・`add_keyform_key` (既定値のキーを先に)・`set_keyform_cell` | 直した |
  | エディタ | エージェントの分類の直し (`set_layer_correction`) に残すラベルが無い (一覧が「分類の答えに戻す」になる) | 残すラベルと左右 (`correction`) を持つ | 直した |
  | エディタ | 分類のラベルに実在しないもの (`eyelid`・`brow`・`clothes`) | classify.rs の `Label::ALL` の 40 個 | 直した (選択欄も 40 個に) |
  | エディタ | 見本のプロジェクト・モデル | `ugoe sample dress-up --project` そのもの (グルーを 1 つ足すのは T0044 の意図した差) | 直すものなし |
- 表に無い種類の出し方 (`<code>`) を撮る場面は足さなかった: `CHANGE_KINDS` は proposals.test.ts が command.rs のすべての種類と
  照らし合わせていて、フロントエンドとバックエンドは同じアプリに入るので、本物では出ない。
- 検査: hosts.test.mjs に 3 つ (提案の見本の種類が command.rs にある・前のコマンドにだけ頼る / 分類のラベルが classify.rs にある /
  スタジオの見本が 27 パラメータ・`earwear` のグループ・4 つのモーションと表情・拡張子 `json` を持ち、ホットキーのモーションと表情が見本にある)。
  スタジオの見本はソースから読むので、`ugoe sample avatar` が変われば偽のホストも追う。
- `--compare` の結果 (前 = このブランチの最初のコミットの前の `tests/ui-audit` で撮ったもの。アプリのビルドは同じ):
  - スタジオ (すべての組、183 場面): `+0 -0 ~0`。見つけたものは前後とも 0。画面では雑談のシーンにパラメータ 27・表情 4・
    ポーズ `earwear` / `no_earwear`・「Playing idle」が出て、ツールバーに `ugoe-sample-profile.json` が出る。
  - エディタ (時間の都合で `--variants en-light-wide,ja-dark-narrow,en-dark-forced-wide,ja-light-zoom200` の 4 組、162 場面): `+0 -0 ~0`。
    「採用で変わるもの」は `add 5 parameters · create 1 warp deformer · add 7 keyform keys · place 5 keyforms` で、`<code>` は出なくなった。
    前後とも `en-dark-forced-wide-tab` の `div[tree] "Parts"` の `faint focus ring` が 1 件 (この変更とは関係が無い。前から出ている)。
- `just ci` が通った。画面のコードとクラス名は変えていないので e2e は流していない。

### 起票の候補

- エディタの `en-dark-forced-wide-tab` でパーツのツリー (`div[tree] "Parts"`) が `faint focus ring` (8.73:1 at most) と出る。
  比が 3:1 を超えるのに faint になる理由 (はっきり変わった画素が要る量に届かない) を確かめる。
