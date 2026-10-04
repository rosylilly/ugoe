---
id: T0010
title: ホストから来る数 (NaN・無限大・負の dt) の扱いを入口ごとの表にして公開仕様に書く
status: done
milestone: M9
crates: [ugoe-runtime, ugoe, ugoe-ffi, ugoe-wasm, bindings/godot, bindings/unity, bindings/web, docs, spec]
depends_on: [T0433]
assignee: claude-worker-T0010
---

# T0010: ホストから来る数 (NaN・無限大・負の dt) の扱いを入口ごとの表にして公開仕様に書く

## 目的

M9 のコードレビューで最も多かった横断の所見 (T0137・T0139・T0317・T0423・T0424・T0427。まとめは docs/workflow/code-review.md#m9-の結果)。
ホストが渡す数 (`dt`・パラメータの値・不透明度・リップシンクの音量・`follow` の強さ・トラッキングの値) の扱いが入口ごとに違った:
C ABI は拒む、wasm は無視する、SDK の `follow` は誤り、`update` は 0 として扱う、Godot は検査していなかった。
各レビューで個別に塞いだ (T0137 の `ParameterValues::set`、T0139 の `sanitize_dt`、T0317 のプレイヤー、T0423・T0427 の入口、T0433 の wasm) が、
全体の表は無く、次に足す入口で同じ漏れが起きる。

## 受け入れ条件

- [x] SDK (`ugoe::Puppet`)・C ABI・wasm・Godot・Unity の、ホストから数を受け取るすべての入口を表にし (値・NaN・±無限大・負・範囲外の扱い)、
      「前提は下の層 (`Puppet`・`ugoe-runtime`) で決め、上の層は表し方 (エラーにするか) だけを持つ」(T0139) に沿って食い違いを直した
- [x] ランタイムの残りの入口 (パーツの不透明度の上書き `set_part_opacity_override`・`DynamicOffsets`・物理とモーションの出力の合成) の NaN を確かめ、
      止める場所を決めてテストを足した
- [x] 表を docs/portability.md (または仕様書) と英語の公開仕様 (`spec/`) の対応する節に書いた。バインディングの README から表へリンクする
- [ ] `just ci`・`just wasm-check`・`just ffi-check`・`just godot-check` が通る (Unity は `just unity-check` か、動かせなければメモ)

## 参考

- docs/workflow/code-review.md#m9-の結果
- T0137・T0139・T0317・T0423・T0424・T0427 のメモの「横断」、T0433

## メモ

- T0019: T0323 の起票の候補「`Puppet::start_motion` などで、渡されたモーション・表情の数値を検査する (デバッグ時の assert か `Result`)」も、
  ホストから来る値の入口としてこの表で扱う。検査するなら公開 API の変更になるので、判断が要る点として書く。

### T0010 の結果

- **表:** `docs/portability.md#ホストから来る数` に方針 (前提は `ugoe-runtime`・`Puppet` が決め、上の層は「拒む」か「下の層の扱いをそのまま出す」かの表し方だけ) と、
  「下の層の扱い」(値ごと) と「入口ごとの表し方」(C ABI・Unity / wasm / Godot) の 2 つの表を書いた。英語の公開仕様は `spec/motion.md#host-supplied-numbers`
  (新しい節。値ごとの扱いと「エラーを返せる実装は拒んでもよい」)、`spec/evaluation.md` の Parts と Dynamic physics offsets に MAY を足した。
  日本語は `docs/spec/motion.md#ホストから来る数`・`docs/spec/evaluation.md`。`docs/spec/public-spec.md` に対応の行を足した。各 README (ffi・wasm・ugoe・runtime・
  godot・unity・web) から表へリンクした。`docs/workflow/code-review.md` の M9 の行も直した。
- **ランタイムの残りの入口 (止める場所と決めたこと):**
  - `ModelInstance::set_part_opacity_override`: 値か重みが有限でない書き込みは無視する (前の上書きのまま)。`ParameterValues::set` の「NaN は書かない」と同じ形。
    テスト `non_finite_part_opacity_overrides_are_ignored` (修正前に失敗することを確かめた)。
  - `DynamicOffsets`: 評価で足すところ (4 か所) で、有限でない加算量を 0 とみなす。`warp_points_mut`・`mesh_positions_mut` は可変のスライスを返すので
    書くときには止められず、評価の側を 1 つの止める場所にした。テスト `non_finite_offsets_are_not_added` (修正前に失敗)。
  - 物理とモーションの出力の合成: 物理は T0320 で済み。モーション・表情は `ParameterValues::set`・`blend` を通るので NaN は書かれず、無限大はクランプ
    される。食い違いが 1 つあった: `Puppet::start_motion` に有限でない値のパーツの不透明度のカーブ・エフェクトグループ `LipSync` の強さを持つ
    モーションを渡すと、`update` は動くが、指揮役が配る `part_opacities_applied`・`lip_sync_applied` に有限でない値が入り、受け手の `follow` が
    毎フレーム拒んでいた。`Puppet::update` で、有限でないパーツの不透明度のカーブは無いものとし (上書きにもポーズの要求にも使わない)、
    有限でない強さではリップシンクを当てないようにして、記録にも入れない (修正前の挙動とほぼ同じ: NaN の強さは書き込みが無視され、当てないのと同じだった)。
    テスト `crates/ugoe/tests/host_numbers.rs` (NaN・±∞ のカーブ・長さ・フェード・表情で、1 つの `Puppet` と指揮役 + 受け手が有限で一致する。修正前は失敗)。
- **入口の間の食い違いを揃えたもの:**
  - C ABI の `ugoe_puppet_follow` が、SDK の `Puppet::follow` と同じ有限の検査 (強さ・パーツの不透明度の無限大) を写して持っていたので外した
    (結果コードは同じ `UGOE_ERROR_INVALID_ARGUMENT`。文が SDK のものになる)。
  - Unity の `UgoePuppet` の毎フレームの `dt` (`Time.deltaTime × speed`) は `dt > 0` だけを見ていたので、`speed` が無限大だと C ABI が拒んで毎フレーム例外に
    なっていた。正の有限でなければ 0 で進める (`FrameState.ClampDeltaTime`、`FrameStateTests` に無限大を足した)。
- **揃えずに表に書いたもの (表し方の違いで、前提の違いではないと判断):** C ABI・Godot・Unity は拒み、wasm の `update`・`set_parameter`・`set_lip_sync_level` は
  無視する (T0433 の判断のまま)。リップシンクの音量の負は C ABI では「入力をやめる」、SDK・wasm・Godot では 0 にクランプ (C ABI は `Option` を持てないため
  負を「無し」に使う。Godot は `clear_lip_sync_level` がある)。Godot のブレンドシェイプは有限でない値を飛ばす (C ABI は入力全体を拒む。辞書で 1 つずつ渡す形のため)。
- **判断が要る点 (止めてはいない): `Puppet::start_motion`・`set_expression` で数値を検査するか (T0019 から入った T0323 の候補)。**
  - 案 A (推奨): 今のまま検査しない。上のとおり、有限でない値を持つモーション・表情でもパラメータ・パーツの不透明度・頂点は有限のままで、受け手とも一致する
    (`host_numbers.rs`)。ファイルから読むものは `ugoe-format` が検証済み。検査したいホストは `ugoe_model::validate::validate_motion` を自分で呼べる。
    doc に書いた。
  - 案 B: `debug_assert!` で検査する。公開 API は変わらないが、テストやデバッグビルドでだけパニックするのは「ホストの数で止まらない」方針と食い違う。
  - 案 C: `Result` を返す (`start_motion` は今 `bool`)。公開 API の変更で、C ABI・wasm・Godot・Unity は検証済みのものしか渡さないので得るものが小さい。
  - 安全の面では案 A で足りるので、このタスクでは何も変えていない。B か C にするなら別のタスクで。
- **確かめたこと:** `just ci` は、`apps/editor` の `src/lib/viewport/ToolOverlay.test.ts > deletes the selected vertices with Delete` が 1 件失敗する
  (`expected [ +0 ] to deeply equal []`)。この変更は `apps/` を触っておらず、rebase した main の上でも同じテストだけで失敗するので無関係
  (T0699 のテストと T0693 の変更の組み合わせと思われる)。`just ci` の残りの手順 (`i18n`・`app-build-check`・`script-check` と、web-check の editor 以外の
  check と test) は個別に流して通った。`just wasm-check`・`just ffi-check`・`just godot-check`・`just unity-check` (88 件) は通った (2026-10-04)。
  実際の Unity エディタでは確かめていない (`unity-check` は dotnet の単体テスト)。
- **起票の候補:**
  - `ParameterValuesError::LipSyncNaN`・`PartOpacityNaN` の改名 (無限大も含む。T0139 の見送りのまま。1.0 の前に公開 API をまとめて直すとき)。
  - `set_effects_json` (C ABI・wasm・Godot) が設定を検証しない (JSON の `1e39` は `f32` の無限大になる)。ランタイムは安全 (表のとおり) だが、作者の誤りを
    見逃す。`validate` にエフェクトの検査があれば通すとよい。
  - AGENTS.md の「NaN と無限大」の項から `docs/portability.md#ホストから来る数` へリンクする (AGENTS.md はこのタスクの範囲の外なので触っていない)。

