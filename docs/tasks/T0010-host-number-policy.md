---
id: T0010
title: ホストから来る数 (NaN・無限大・負の dt) の扱いを入口ごとの表にして公開仕様に書く
status: todo
milestone: M9
crates: [ugoe-runtime, ugoe, ugoe-ffi, ugoe-wasm, bindings/godot, bindings/unity, bindings/web, docs, spec]
depends_on: [T0433]
assignee:
---

# T0010: ホストから来る数 (NaN・無限大・負の dt) の扱いを入口ごとの表にして公開仕様に書く

## 目的

M9 のコードレビューで最も多かった横断の所見 (T0137・T0139・T0317・T0423・T0424・T0427。まとめは docs/workflow/code-review.md#m9-の結果)。
ホストが渡す数 (`dt`・パラメータの値・不透明度・リップシンクの音量・`follow` の強さ・トラッキングの値) の扱いが入口ごとに違った:
C ABI は拒む、wasm は無視する、SDK の `follow` は誤り、`update` は 0 として扱う、Godot は検査していなかった。
各レビューで個別に塞いだ (T0137 の `ParameterValues::set`、T0139 の `sanitize_dt`、T0317 のプレイヤー、T0423・T0427 の入口、T0433 の wasm) が、
全体の表は無く、次に足す入口で同じ漏れが起きる。

## 受け入れ条件

- [ ] SDK (`ugoe::Puppet`)・C ABI・wasm・Godot・Unity の、ホストから数を受け取るすべての入口を表にし (値・NaN・±無限大・負・範囲外の扱い)、
      「前提は下の層 (`Puppet`・`ugoe-runtime`) で決め、上の層は表し方 (エラーにするか) だけを持つ」(T0139) に沿って食い違いを直した
- [ ] ランタイムの残りの入口 (パーツの不透明度の上書き `set_part_opacity_override`・`DynamicOffsets`・物理とモーションの出力の合成) の NaN を確かめ、
      止める場所を決めてテストを足した
- [ ] 表を docs/portability.md (または仕様書) と英語の公開仕様 (`spec/`) の対応する節に書いた。バインディングの README から表へリンクする
- [ ] `just ci`・`just wasm-check`・`just ffi-check`・`just godot-check` が通る (Unity は `just unity-check` か、動かせなければメモ)

## 参考

- docs/workflow/code-review.md#m9-の結果
- T0137・T0139・T0317・T0423・T0424・T0427 のメモの「横断」、T0433

## メモ

- T0019: T0323 の起票の候補「`Puppet::start_motion` などで、渡されたモーション・表情の数値を検査する (デバッグ時の assert か `Result`)」も、
  ホストから来る値の入口としてこの表で扱う。検査するなら公開 API の変更になるので、判断が要る点として書く。
