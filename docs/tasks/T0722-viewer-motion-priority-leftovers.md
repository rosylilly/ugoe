---
id: T0722
title: ビューア: 待機モーションをモデルごとに覚える・優先をパラメータごとに・モーションの優先を C ABI と npm に出す
status: done
milestone: M4
crates: [apps/viewer, ugoe, ugoe-motion, ugoe-wasm, ugoe-ffi, bindings/web, spec, docs]
depends_on: [T0721]
assignee: claude-worker-T0722
---

# T0722: ビューアのモーションの優先の残り

## 目的

T0721 の後続の候補をまとめる。

## 受け入れ条件

- [x] 待機モーションの選択をモデルごとに覚え、覚えたものの一覧に出す (`modelMemory.ts`)
- [x] トラッキング中のモーションの優先をパラメータごとに切り替えられる (頭だけ・目は除くなど)
- [x] モーションの優先 (`set_motion_over_tracking`) を C ABI と npm パッケージ (`<ugoe-avatar>`) にも出す。ヘッダと README を揃える
- [x] `just ci`・`just wasm-check`・`just conformance --only c,wasm`・ビューアのビルドが通る

## 参考

- `docs/tasks/T0721-viewer-motion-and-tracking.md` のメモ

## メモ

- **待機モーションをモデルごとに覚える**: `animation.ts` の `idleMemory` (`ModelMemory<{ motion: string | null }>`、キー
  `ugoe-viewer.idle-motion.models`、50 体)。選択欄で選んだときだけ覚え (「(なし)」も)、モデルを開くと覚えた選択、無ければ既定
  (`initialIdleMotion`。覚えたモーションがモデルから消えていたら既定で、覚えたものは消さない)。選択欄の横に「覚えた選択を忘れる」。
  `rememberedModels.ts` の `mergeRemembered` を入れ物 3 つ (`{ lighting, dressUp, idle }`) を受ける形に変え、「覚えたもの」の一覧に
  「待機モーション」の列と、2 つ以上覚えた行の「すべて忘れる」(前は「両方を忘れる」) を足した。
- **優先をパラメータごとに (決めたこと)**: T0721 の方式 (wasm が `Puppet::update` の前にトラッキングをホストの書き込みとして入れ、入力を外して
  進める) では「全部モーション優先」しか作れない (一部だけトラッキングを勝たせるには、モーションの後・基準値の保存の前に書く必要があり、
  外から割り込めない)。そこで `Puppet` に `set_motion_over_tracking(bool)`・`set_tracking_kept(ids)` を足し、オンならモーションの前に
  「残す」以外へ、モーションの後に「残す」パラメータへトラッキングを書く (`ugoe_motion::TrackingMap::apply_where` を足した)。既定はオフで
  適用順は今のまま。wasm の `update_with_motion_over_tracking` は消し、`WebPuppet` は設定を持って読み直した `Puppet` に渡し直す
  (`set_tracking_kept`・`tracking_kept` を足した)。`docs/spec/motion.md#トラッキング中のモーションの優先` を足し、英語の
  `spec/motion.md` の Order of operations に MAY の 1 項を足した (`public-spec.md` の対応も)。
- ビューア: 「トラッキング中もモーションを優先」の下に「優先する部分」(頭の向き・体・肩・目・眉・口・その他。`trackingGroup` が ID の先頭で
  分ける。一部だけなら中間の表示) と「パラメータごとに選ぶ」。外した ID を `ugoe-viewer.motion-over-tracking.kept` に覚える (モデルによらない)。
  外したパラメータと重なるモーションには「一部トラッキング」の印。
- **C ABI**: `ugoe_puppet_set_motion_over_tracking`・`ugoe_puppet_motion_over_tracking`・`ugoe_puppet_set_tracking_kept` (パラメータの番号。
  範囲外は `UGOE_ERROR_OUT_OF_RANGE` で何も変えない)。`just ffi-header` でヘッダを作り直した。ABI の版は足しただけなので上げていない。
- **npm (`<ugoe-avatar>`)**: 属性 `motion-over-tracking` (付けるとオン) と `tracking-kept` (`"EyeLOpen EyeROpen"`。`parseIdList`)。
  作った WebPuppet に渡す (WebPuppet は読み直しても保つ)。スタブに `set_tracking_kept`・`tracking_kept`。
- テスト: Rust 2 件 (`ugoe::tests::motion_over_tracking_prefers_the_motion_except_kept_parameters`、
  `ugoe_motion::tracking::tests::apply_where_writes_only_the_filtered_parameters`) と C ABI 1 件 (`motion_over_tracking_follows_the_puppet`)。
  wasm の T0721 の 2 件は `Puppet` のテストに移したので消した。Vitest: ビューア 6 件 (待機の記憶 3・パラメータごと 3) と一覧 1 件、bindings/web 1 件。
- **確かめたこと**: `just ci`・`just wasm-check`・`just conformance --only c,wasm`・`just ffi-check`・ビューアの `vite build`。
  ヘッドレス Chrome (SwiftShader の WebGL2、`vite preview`、スクリプト `scratchpad/T0722/check.mjs`、出力 `run.log`): アバターで待機を tilt に →
  読み直して開くと tilt・着せ替えは既定の idle → 「(なし)」を覚える → アバターに戻ると tilt・一覧に両方の行 → 一覧と パネルから忘れる。
  トラッキング (ヨー 20、eyeBlinkLeft 0.75) 中に shake で AngleX 12.5 → 頭の向きを外すと 20・tilt で EyeBallX 0.5 → 目を外すと 0 →
  AngleZ も外すと 0 (印は「トラッキングで隠れる」)・選択は読み直しても残る。スクリーンショットは
  `<scratchpad>/T0722/shots/` (`01-idle-remembered.png`・`02-memory-dialog.png`・`03-groups-head-kept.png`・`04-eyes-angley-kept.png`)。
  読み込み時の 404 が 1 件 (前のタスクと同じ)。
- **確かめていないこと**: `just test-gpu` (描画は触っていない)、Tauri 版 (WebKitGTK) と `just viewer-build` の deb、本物のカメラ、
  `<ugoe-avatar>` の属性を実際のブラウザで (単体テストと型検査だけ)。
- **範囲外の変更**: `ugoe` (Puppet) と `ugoe-motion` (`apply_where`)、英語の `spec/motion.md`。team-lead に相談を送ったが、この時点で返事は
  無い (承認されたら crates: に足す)。エディタの `ugoe-wasm-stub.d.ts` は触っていない。
- **後続の候補**: bindings/unity・bindings/godot に `set_motion_over_tracking`・`set_tracking_kept` を出す。`<ugoe-avatar>` に JS の
  プロパティ (属性の写し) を足す。ビューアで優先の選択をプリセット (「頭だけ」など) として名前を付けて選べるようにする。

- **範囲の追加 (team-lead が承認)**: `ugoe` (Puppet に優先の設定を置き、wasm と ffi で同じ実装を共有するため)、`ugoe-motion`
  (`TrackingMap::apply_where` の小さな追加)、`spec` (英語の `spec/motion.md` の適用順に MAY の 1 項)。
