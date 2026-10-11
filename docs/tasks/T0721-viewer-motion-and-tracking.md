---
id: T0721
title: ビューア: トラッキング中にモーションを優先する・待機モーションを自動で流す・使っている表情の一覧
status: done
milestone: M4
crates: [apps/viewer, ugoe-wasm, bindings/web, docs]
depends_on: [T0720]
assignee: claude-worker-T0721
---

# T0721: ビューアのモーションとトラッキングの残り

## 目的

T0720 の後続の候補をまとめる。見本のモーションは頭と体の向きを動かすので、フェイストラッキングを使っている間はほとんど見えない。

## 受け入れ条件

- [x] トラッキング中にモーションを優先する切り替え (モーションが動かすパラメータをトラッキングが書かない)
- [x] 待機モーションを自動で流し、ほかのモーションが終わったら待機に戻す
- [x] wasm から、いま使っている表情の一覧を返す (今は画面の操作から求めている)。bindings/web のスタブも揃える
- [x] Vitest とヘッドレスのブラウザで確かめる。`just ci`・`just wasm-check`・ビューアのビルドが通る

## 参考

- `docs/tasks/T0720-viewer-motion-playback.md` のメモ

## メモ

- **トラッキング中にモーションを優先する (決めたこと)**: ランタイム (`Puppet`) の適用順は変えず、wasm に口を足した。
  `WebPuppet.set_motion_over_tracking(true)` の間、`update` がトラッキングの値を `Puppet::update` の前にホストの書き込みとして入れ
  (`TrackingMap::for_model(..).apply` → その間だけ `Puppet` の入力を外して進める → 戻す。`crate::tracking::update_with_motion_over_tracking`)、
  仕様の「手で動かしたパラメータとの関係」と同じ扱いでモーションがその上にフェードの重みで乗る。モーションが動かさないパラメータは
  トラッキングのまま。ビューアでだけ対象を絞る案は、トラッキングが `update` の中で書くので外から戻せず採れなかった。
  ビューアは「トラッキング中もモーションを優先」(既定で入、localStorage) が入っていて、**待機以外の優先度** のモーションの再生中だけ
  入れる (ループする待機モーションが頭の向きをずっと上書きしないように。`animation.ts` の `motionOverTrackingActive`)。
  優先する間は待機以外のモーションに「トラッキングで隠れる」の印を出さない。`docs/spec/motion.md` に 3 行足した (英語の `spec/` には
  WebPuppet の記述が無いので変えていない)。
- **待機モーションの自動再生**: 「待機モーションを自動で流す」(既定で入、`ugoe-viewer.auto-idle`) と選択欄。モデルを開いたとき・ほかの
  モーションが終わったとき・「止める」で止めたときに待機の優先度で始める (「繰り返す」が先)。待機モーションそのものを止めたら流さない。
  既定は名前が `idle` のもの、無ければ `idle` で始まるもの (ループを先に)。**モデルごとには覚えない** (T0711 の仕組みに入れない): 名前の約束で
  足り、覚えたものの一覧と「忘れる」を増やすほどではないと判断した。README の「待機モーション (T0721)」に書いた。
- **使っている表情の一覧**: `WebPuppet.active_expressions()`。`ExpressionManager::players` は止めたかを返さないので、ランタイム層を変えず、
  WebPuppet が表情の操作を写して持ち (`crate::animation::ActiveExpressions`)、再生中の表情が無い名前を除いて返す。モデルを読み直すと空。
  ビューアは操作のたびにこれを読む (`animation.ts` の `activeExpressions` は消した)。team-lead に ugoe-motion に getter を足す相談をしたが、
  返事の前にこの形で済ませて取り下げた。
- bindings/web のスタブに `active_expressions`・`is_motion_over_tracking`・`set_motion_over_tracking` を足した。エディタの
  `ugoe-wasm-stub.d.ts` は T0720 の口も入っていない部分的なものなので触っていない。
- テスト: Rust 3 件 (`tracking::tests::motion_over_tracking_*` 2 件、`animation::tests::active_expressions_follow_the_operations`)、Vitest 3 件
  (`defaultIdleMotion`・`motionOverTrackingActive`・`loadToggle`/`saveToggle`)。文書: viewer README・ugoe-wasm README・`docs/manual/motions.md`。
- **確かめたこと**: `just ci`・`just wasm-check`・`pnpm --filter @ugoe/runtime check`・`vite build`・`just viewer-build --bundles deb`。
  ヘッドレス Chrome (SwiftShader の WebGL2、`vite preview`) で見本の アバター・着せ替え (スクリプト `<scratchpad>/T0721/check.mjs`、出力 `run.log`):
  開くと idle が流れる・nod が終わると idle に戻る・shake を止めると idle に戻る・idle を ■ で止めると流れない・待機を tilt に替えると
  (ループしないので) 繰り返す・トラッキング (ヨー 20°、jawOpen 0.4) 中は idle の AngleX がトラッキングの 20・shake で AngleX が 12.5
  (優先を切ると 20)・終わると 20 に戻る・nod + smile で AngleY が nod、MouthOpenY はトラッキングの 0.4、MouthForm は smile の 1・
  表情 smile+surprised → smile を外す (フェード中) → surprised だけ → sad → 空・自動再生を切って読み直しても覚えている。
  スクリーンショットは `<scratchpad>/T0721/shots/` (`01-idle-auto.png`〜`06-expressions.png`)。読み込み時の 404 が 1 件 (前のタスクと同じ)。サーバーは止めた。
- **確かめていないこと**: 本物のカメラと顔での併用 (入力は `setTracking` で直接渡した)、Tauri 版の画面 (WebKitGTK)、本物の GPU。
- **範囲外の変更**: なし。
- **後続の候補**: 待機モーションの選択をモデルごとに覚える (要望があれば `modelMemory.ts` の入れ物を足し、覚えたものの一覧に出す)。
  優先の切り替えをパラメータごと (頭だけ・目は除くなど) にする。モーションの優先を C ABI・npm パッケージ (`<ugoe-avatar>`) にも出す。
