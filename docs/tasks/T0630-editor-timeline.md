---
id: T0630
title: エディタ: タイムラインとモーションの編集
status: done
milestone: M3
crates: [ugoe-editor-core, ugoe-wasm, apps/editor, docs/manual]
depends_on: [T0303, T0309]
assignee: claude-worker-T0630
---

# T0630: エディタ: タイムラインとモーションの編集

## 目的

エディタでモーション (と表情) を作り、タイムラインで再生・編集できるようにする。

## 受け入れ条件

- [x] モーション・表情をプロジェクトに持ち、保存・読み込み・`.ugoe` への書き出しができる (プロジェクトの形式を変えるなら `docs/spec/file-formats.md` も)
- [x] モーション・表情を編集する `Command` (作成・削除・名前・長さ・ループ・フェード、カーブの追加・削除、点の追加・移動・削除、区間の種類 (直線・ベジェ・ステップ) とベジェの取っ手、イベント) と、検証・Undo のテスト
- [x] タイムラインのパネル: 再生・停止・シーク・ループ、パラメータごとのトラックとキー、今のスライダーの値でキーを打つ操作、カーブの表示と取っ手の編集
- [x] 表情の作成 (今のパラメータの値から、op の選択) とプレビュー
- [x] 操作マニュアルにモーションの章を足す
- [x] `just ci`・`just gen` (差分なし)・エディタのビルドが通る

## 参考

- `docs/spec/motion.md`
- `docs/editor.md`

## メモ

- **データの置き場所:** モーションと表情を `Document` のスナップショットに移した (`Animations { motions, expressions }`、
  `Document::with_animations`・`motions()`・`expressions()`)。編集は Undo の対象で、`is_modified` にも効く。物理は
  `ProjectSession` のまま (T0629)。`Project::from_document(meta, &doc, physics)` からモーション・表情の引数を外した
  (呼び出し側の `ugoe-cli` の 2 か所とテスト 2 か所は引数を消しただけの範囲外の変更)。プロジェクトの形式は変えていない
  (`motions/`・`expressions/` は元からある) ので `file-formats.md` は変更なし。
- **コマンド (17 個、`motion_edit.rs`):** `add/remove/rename_motion`・`set_motion_properties`・`add/remove_motion_curve`・
  `set_motion_curve_fade`・`set_motion_key`・`move_motion_key`・`remove_motion_key`・`set_motion_segment` (直線・ベジェと
  取っ手・ステップ・逆ステップ)・`set_motion_events`・`add/remove/rename_expression`・`set_expression_properties`・
  `set_expression_parameters`。モーション・表情は名前、カーブは対象、キーは時刻で指定。名前はテクスチャ名と同じ規則。
  **検証はモデルと同じ考え方** (編集したモーション・表情の問題が増えたら拒否。読み込んだ不正なデータも直す編集はできる)。
  **判断:** キーを区間の中に打つと同じ種類で分け、ベジェは de Casteljau で分けて打った値の分だけ取っ手をずらす
  (曲線上に打つと形が変わらない。テストで 41 点を比較)。キーの移動は隣を越えられず、隣のベジェの取っ手を一緒に動かして
  区間に収める。消すと入る区間の種類でつなぎ、最後のキーならカーブを消す。規則は `docs/editor.md#モーションと表情の編集`。
- **`ugoe-wasm` の追加 (team-lead の承認を得て `crates:` に加えた):** タイムラインのシークを決定的にするため、`ugoe-wasm` に `sample_motion_json(json, t)` (各カーブの時刻 t の値の JSON、
  `ugoe_motion::curve::evaluate` そのもの。ネイティブのテスト付き。変更は `src/animation.rs` と `src/web/animation.rs` だけ)。
  README とエディタの wasm のスタブの型も更新。パーツの不透明度・エフェクトの強さのカーブの値も返すが、タイムラインは
  パラメータのカーブだけを書き込み、ほかは無視する (`docs/editor.md#タイムライン` と manual の「制限」に書いた)。
  **判断:** タイムラインの再生・シークは WebPuppet のモーションの再生を使わず、この値をスライダーと同じ `setParameter` で
  書き込む。T0309 の既知の問題 (編集のたびの読み直しで再生状態が初めからになる) を避け、同じ時刻は常に同じ姿になる。
- **画面 (`apps/editor/src/lib/panels/timeline/`):** ビューポートの下のパネル (畳める)。「Motions」: モーションの作成・削除・
  名前・長さ・ループ・フェード・イベント、再生・停止・Repeat・フレーム送り・時刻の欄・目盛りでのシーク、パラメータごとの行と
  キーのひし形 (ドラッグで時刻)、行の ◆ と「◆ Key changed」(今のスライダーの値でキーを打つ)、選んだ行のカーブのグラフ
  (キーと取っ手のドラッグ・ダブルクリックでキー・区間の種類・時刻と値の欄・カーブのフェード・カーブの削除)、Space / ← → /
  Delete。「Expressions」: スライダーから作成 (op を選ぶ。既定値に重ねると今の値になる値)・op と値の表・パラメータの追加・
  「Set values from sliders」・フェード・名前・削除・プレビュー (重ねて見せ、戻す)。IPC は `get_animations` を足した。
  モーションを選ぶ・開くだけではスライダーを動かさない (シーク・再生・キーの操作の後だけ書き込む)。
- **テスト:** Rust 16 件 (`motion_edit_tests.rs`: Undo / 変更の印、名前、長さ・フェードの検証、対象の存在、イベントの並び、
  カーブの追加・削除・フェード、キーの挿入の順と種類、ベジェの形を保つ分割、移動と取っ手、削除とつなぎ、区間の種類と取っ手の
  範囲、読み込んだ不正なモーション、batch の Undo、表情、保存・読み込み・書き出しの往復) と JSON の往復の見本、wasm 1 件。
  Vitest 17 件 (`timeline.test.ts`・`expressions.test.ts`)。
- **確かめたこと:** ビルドした画面 (`just wasm`・`vite build`) をヘッドレス Chrome で開き、IPC を本物の `Document` に
  中継するブリッジ (リポジトリ外) で `ugoe sample avatar --project` のプロジェクトを操作した。AngleX に 0 / 30 / -30 を
  0 / 1 / 2 秒でキー、「Key changed」で AngleY、区間をベジェにして取っ手をドラッグ (0.5 秒の値が 15 → 29.3)、ひし形の
  ドラッグ (2 → 1.8 秒) と Ctrl + Z、長さ・ループ、再生 (時刻が進み、ループで折り返す)・停止、Space / → / Delete、イベント、
  Save As・Export。書き出した `.ugoe` を `ugoe::Puppet` で再生し、AngleX がエディタの表示と同じ値 (0.5 秒で 29.287) になり、
  イベントが発火することを確かめた。表情は「New from sliders」(EyeLOpen 0.2・MouthForm 0.8 → add -0.8 / 0.8)、op の変更、
  パラメータの追加、名前の変更、プレビューと戻し、保存 (`expressions/Smile.expression.json`)。マニュアルの 2 枚はこの画面。
- **確かめていないこと:** 実際の Tauri のウィンドウでの操作 (`cargo check -p ugoe-editor` と `vite build` まで)。
- **やらなかったこと・後続の候補:**
  - プレビューでフェード・目パチ・呼吸・物理を掛ける (WebPuppet の再生を読み直しに耐えるようにするか、時刻を指定して
    `Puppet` を評価する API)。パーツの不透明度・エフェクトの強さのカーブの作成とプレビュー。
  - タイムラインの拡大・スクロール、複数のキーの選択とまとめての移動・コピー、値の軸の拡大。
  - `remove_parameter` でモーション・表情の参照も直す (今は `(missing)` の行として残る)。
  - `docs/manual/images/screen.png` (画面の全体) はタイムラインのない頃の写真のまま。
