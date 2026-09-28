---
id: T0617
title: 推定の使い勝手 (深度の編集と、組み合わせのセルの自動推定)
status: done
milestone: M2
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: [T0616]
assignee: claude-worker-T0617
---

# T0617: 推定の使い勝手 (深度の編集と、組み合わせのセルの自動推定)

## 目的

M2 の受け入れテスト (T0616) で見つかった、キーフォームの推定 (ADR 0010) を使ううえでの 2 つの穴を埋める。

1. 既存のメッシュ・ワープに頂点ごとの深度 (`depth`) を設定するコマンドも UI もなく、エディタから深度からの回転を使えない。
2. 新しい格子にキーを足すと、組み合わせのセルも `authored` になるため、推定の前に手で `derived` にする手間がある。ADR 0010 の「端の形だけ作れば、組み合わせは推定される」を満たすには、この手間をなくす必要がある。

## 受け入れ条件

- [x] 深度を設定するコマンド (例: `SetDepth { target: mesh | warp, depth: Option<Vec<f32>> }`) があり、長さを検査し、深度からの回転のセルを作り直す (`touched`)
- [x] 深度の簡単な作り方をコマンドまたはエディタの機能として用意する: 一様な値、要素の外形から作るふくらみ (ドーム状) のプリセット、選択した点の値の設定
- [x] インスペクタに深度の編集 UI がある (プリセット・選択点の値)。ビューポートで深度を色などで確かめられると望ましい
- [x] `AddKeyformKey` で新しくできたセルのうち、中立でない軸が 2 本以上あるセル (組み合わせのセル) は `derived` (加算合成) とし、その場で推定する。1 軸のセル (端のセル) は従来どおり補間した値の `authored` とする。全軸に中立のキーがない格子では従来どおり。キーを足しても見た目が変わらないこと (T0608 の条件) は保つ
- [x] `docs/spec/keyform-inference.md` と `docs/editor.md` を更新し、T0616 の受け入れテストから「手で derived にする」手順を取り除く (テストが新しい挙動を確かめる)
- [x] `just ci`・`just gen` (差分なし)・エディタのビルドが通る

## 参考

- `docs/tasks/T0616-m2-acceptance.md` のメモ
- `docs/adr/0010-keyform-inference.md`

## メモ

- **深度のコマンド** (`crates/ugoe-editor-core/src/depth.rs`): `set_depth { target, depth }` (`target` は
  `ChildRef` = メッシュかワープ。長さ `DepthLength`・有限 `NonFiniteDepth` を検査し、`touched` に積むので
  深度からの回転のセルが新しい深度で作り直される) と `apply_depth_preset { target, preset }`。プリセットは
  `uniform { value }`・`dome { peak }` (基準の姿の外接矩形に内接する楕円で正規化した距離 ρ から
  `peak · sqrt(1 − ρ²)`、楕円の外は 0)・`points { indices, value }` (他の点は今の深度、なければ 0。範囲外は
  `DepthPointOutOfRange`)。計算は純粋な関数 `uniform_depth`・`dome_depth`・`points_depth`。基準の姿は軸ごとに
  既定値に最も近いキーのセル (`grid::rest_cell_index`)。深度からの回転のセルが残る格子の深度を外すと、作り直しの
  失敗 (`Inference`) で拒否される (仕様に明記)。
- **キーの追加の新しい規則** (`keyform::add_key` → `derive_combinations`、`grid::add_key`):
  1. 組み合わせのセル (追加後の格子で中立でない軸が 2 本以上) は、格子の方法が加算合成なら同じ apply で推定する。
     推定は新しい組み合わせのセルだけを `derived` にした複製のモデルで `infer_additive_target` (他のセルはすべて
     `authored` 扱いで元にする) を走らせ、推定値が補間値と一致する (数値は大きさに対して 1e-4、他のフィールドは等しい)
     セルだけを推定値の `additive` にする。一致しないセル (交互作用の項を持つ `authored` の斜め・古い値の `derived` を
     元にした補間) は見た目を保つために 2 へ回す。
  2. それ以外の新しいセルは、埋めた元のセル (補間の 2 セル、範囲外なら端のセル) のどれかが推定のセルなら `derived`、
     なければ `authored`。**T0608 の「格子に推定のセルがあれば derived」から変えた点**: そのままだと、`AngleY` に
     -30 を加えて斜めが `additive` になった直後に 30 を加えると、端 `(0, 30)` が `derived` になってしまうため。
  3. 追加後の格子に中立のキーがない軸があれば T0608 の規則のまま。深度からの回転の格子は 1 を行わない (回転は補間と
     一致しないので見た目が変わる)。
  - 既存のセルは作り直さない (T0608 の判断を保つ)。`add_keyform_key` は `touched` に積まない。
- **受け入れテスト** (`tests/m2/mod.rs`): 段階 3 から `mark_keyform_derived` と `infer_keyforms` を除いた。キーを
  加えた時点で斜めが `additive` (値は `B + Δ_X + Δ_Y`) で、端を 1 つ編集するたびに斜めが式どおりに作り直されることを
  確かめる (アサーションは以前のものをすべて残し、端の編集ごとの検査を足した)。段階 5 は `add_deformer` +
  `reparent_preserving_look` の回避策をやめ、`create_warp_deformer` で `FaceTurn` を作り、`apply_depth_preset`
  (ドーム 12 → 辺の中点を 4) で以前と同じ深度にした。振り向きでは端も回転で作るので、端 4 セルの
  `mark_keyform_derived` は残り (手で作る形ではないことの宣言)、`additive` になった斜めは
  `infer_keyforms { overwrite_derived: true }` で回転に置き換える。手で斜めを `derived` にする手順はなくなった。
- **テスト**: Rust — `depth_tests.rs` 8 件 (純粋な関数 3、作り直し、メッシュと Undo、拒否、プリセット、ドームでの
  作り直し)、`keyform_tests.rs` 6 件 (2 本目の軸、端の編集で斜めが作り直される、3 軸 (2・3 軸のセル)、交互作用が
  あると補間値の `authored`、既存の `derived` を作り直さない、中立のキーのない格子) と T0608 の由来のテストの更新。
  すべてランタイムの評価で見た目が変わらないことを確かめる。JSON の往復と `sample_commands` に 2 コマンドを追加。
  Vitest 8 件 (`inspector/depth/depth.test.ts`)。
- **UI**: `inspector/depth/DepthSection.svelte` (点の数と範囲、Uniform「Set all」、Dome peak「Dome」、Selected
  「Set selected」、「Clear depth」、「Show in viewport」)。選んだ点はメッシュ編集モードの頂点、ワープはギズモの制御点
  (ギズモの選択をローカル状態から `Selection.controlPoints` に移した)。ビューポートは `tools/depth/DepthOverlay.svelte`
  (点の周りに深度の色の輪。ツールの点を隠さないようツールの上に輪で描く。`ViewportController.showDepth`)。
- **検証したこと**: ヘッドレス Chrome (SwiftShader の WebGL2) で `vite build` した画面に、Tauri の IPC を本物の
  `Document::apply_with_report` に中継する使い捨ての HTTP ブリッジ (スクラッチ領域、T0613 のもの) をつないだ。
  モデルは `keyform_inference` の `Nose` の中心に頂点を足したもの。`Nose`: 「5 vertices · none」→ Set all 3 で
  `[3,3,3,3,3]`、Dome 10 で `[0,0,0,0,10]`、輪の色 (灰 4・赤 1)、メッシュ編集で頂点を 1 つ選ぶと「Selected (1)」、
  -5 で `[0,-5,0,0,10]` (その輪が青)、Clear で `null`。`Face` (ワープ): 「4 control points · 0 – 16 px」、ギズモで
  制御点を選び 40 にすると深度 `[0,0,16,40]` と `(30, 0)` のセルの点 3 が `192 → 203.43` (回転の式どおり) で由来
  `depth_rotation`、Clear はエラー表示 (`has no depth`) で拒否、Undo で戻る、チェックを外すと輪が消える。
  `EyeL` に AngleX・AngleY の 3 キーを加えると格子の表示が `+ A + / A A A / + A +`。
- **検証できなかったこと**: 実際の Tauri の WebView での操作。
- **後続の候補**: 深度をブラシで塗る道具 (今は値の設定だけ)。メッシュの外形 (境界の辺) からの距離によるドーム
  (今は外接矩形の楕円なので、細長くない形でないと外形に沿わない)。深度からの回転の格子で、端のセルを最初から
  回転で作る設定 (今は端を `derived` にする操作が要る)。
