---
id: T0607
title: エディタ: デフォーマの作成とギズモ
status: done
milestone: M2
crates: [ugoe-editor-core, ugoe-wasm, apps/editor]
depends_on: [T0606]
assignee: claude-worker-T0607
---

# T0607: エディタ: デフォーマの作成とギズモ

## 目的

選択したメッシュやデフォーマを包むワープデフォーマ・回転デフォーマを作り、ビューポートのギズモで制御点・原点・角度を編集できるようにする。

## 受け入れ条件

- [x] 選択した要素の外接矩形からワープデフォーマ (行数・列数を指定) を作り、子の座標を新しい局所座標系に変換して付け替える `Command` がある。見た目 (評価結果) が変わらないことをテストする
- [x] 回転デフォーマを作り、子を付け替える `Command` がある (同じく見た目が変わらない)
- [x] ビューポートで、ワープの制御点 (複数選択と一括移動)、回転デフォーマの原点・角度・拡大率をギズモで編集できる。編集は現在のパラメータ値のキーフォームに書き込む (キーの上にいないときの扱いは T0608 と揃える)
- [x] `just ci` が通る

## 参考

- `docs/spec/evaluation.md#3-デフォーマのワールド変換`

## メモ

- **コマンド** (`crates/ugoe-editor-core/src/deformer_edit.rs`、テスト `deformer_edit_tests.rs`、式と制限は
  `docs/editor.md#デフォーマの編集`): `create_warp_deformer { id?, name, children, rows, cols, parent? }`・
  `create_rotation_deformer { id?, name, children, origin?, parent? }`・`reparent_preserving_look { element, new_parent }`。
  子の指定は新しい型 `ChildRef` (`{"kind": "mesh" | "deformer", "id"}`)。エラーは `EditError::DeformerEdit` を足した。
  - 基準の姿 (既定値) での各デフォーマのワールド変換を、評価用のメッシュを足したモデルの複製から 1 回の評価で作り
    (ワープは制御点の格子、回転は単位ベクトル)、キーフォームの点を `F_new⁻¹ ∘ F_old` で写す。ワープの逆はニュートン法。
    回転デフォーマの子は線形部分の比 `Q` を相似変換として分解して角度・拡大率・反転を直す (反転を含む場合も)。
  - ギズモの書き込みは、依頼の `SetWarpControlPoints` / `SetRotationKeyform` を一度作ったが、T0608 の
    `set_keyform_cell` が main に入ったのでリベース時に取り除き、それを使うようにした (コマンドを増やさないため)。
- **wasm**: `WebPuppet.deformer_ids()`・`warp_points(id)` (キャンバス座標の制御点)・`rotation_frame(id)`
  (`[ox, oy, ax, ay, bx, by]`)。ランタイムがワールド変換を公開しないので、`src/gizmo.rs` が評価用のメッシュを足した
  モデルの複製を別のインスタンスで持ち、問い合わせのときにパラメータ値を写して評価する (ネイティブでテスト)。
- **フロントエンド**: `viewport/tools/deformer/` (`DeformerGizmo.svelte`、`gizmo.ts` + Vitest)、
  `panels/inspector/DeformerProperties.svelte` (「Wrap in deformer」: Warp の行数・列数 / Rotation)。
  `ToolOverlay.svelte` は主な選択がデフォーマのとき FormTool の代わりにギズモを出す。ツリーの `moveInto` は
  デフォーマ・メッシュの親の付け替えを `reparent_preserving_look` にした。制御点・原点の書き込みは T0608 の
  `canvas_to_local` で親の座標系に直す。キーの上にないときは T0608 と同じ文言で拒否し「Move to nearest key」を出す。
- **テスト**: Rust 23 件 (作成: 最上位のメッシュ・ワープの中 (親がアフィンな姿では厳密、台形では近似になることも確認)・
  回転デフォーマを包む (ワープの子の回転になる)・親の違う子 (共通の祖先)・ワープの親の下・`parent` の指定・パーツの選択・
  回転の作成 3 種・入れ子の回転 / 付け替え: 回転間 (全ポーズで厳密)・ワープから出す / 入れる (キーの間の既定値の
  台形ワープ)・ワープの子のワープ・ワープの子の回転を回転の下へ・回った親 (角度 +20 度)・反転した親 (反転と角度の向きが
  変わる)・同じ親は何もしない / 拒否: 空・重複・行数・未知の要素・ID の重複・循環・NaN の原点・潰れたワープ)。
  見た目は前後のモデルをランタイムで評価して、いくつかのパラメータ値で比べる。Vitest 13 件 (`gizmo.test.ts`)、
  ツリーの `edit.test.ts` を新しいコマンドに更新。`cargo test -p ugoe-wasm` に `gizmo.rs` の 3 件。
- **検証したこと**: `just ci`・`just gen` (差分なし)・`just wasm-check`・`just wasm`・`pnpm --filter @ugoe/editor check` /
  `build`。ヘッドレス Chrome (WebGL2、SwiftShader) で、`vite build` した画面の IPC を本物の `Document` へ中継する
  使い捨てのブリッジ (T0608 のものを複製、スクラッチ領域) につなぎ、`rotation_chain` に最上位のメッシュを足したモデルで:
  ツリーで Palm と Free を選び「Warp」(2×3) → 最上位のワープができ両方が子になり、基準の姿の差は 2e-5px;
  格子の内側からの矩形選択で上の行 4 点を選び、ドラッグで 4 点だけが (30,-20)css px = (10.1,-6.7) キャンバス px 動いた;
  Arm (キーの間) で警告と「Move to nearest key」→ Angle = -30 のセルに、回転の取っ手のドラッグで角度 -30 → 45
  (取っ手の向きの差の 75 度)、拡大率の取っ手で 1 → 1.5、原点のドラッグで (+6.74, +3.37) が書き込まれた;
  ツリーで Free を Hand へドラッグ → `reparent_preserving_look` で基準の姿の差 1e-3px; Undo で戻る。
- **検証していないこと**: 実際の Tauri の WebView での操作、WebGPU での描画、Shift での 15 度刻み・Ctrl + A・
  Shift クリックでの点の追加 (ロジックは MeshEditTool と同じ補助)、ギズモが出ている間の格子の外のクリックでのメッシュ選択。
- **制限・後続の候補**:
  - 写し直しは基準の姿で厳密。キーの間の既定値のままワープへ出入りすると補間の分ずれる。ワープの中のワープは、親の
    ワープがアフィンでない姿で近似になる。ワープの下の回転デフォーマは、基準の姿以外でワープの伸縮を伝えない (仕様どおり)。
    親の違う子をまとめて包むと、共通の祖先より下の親の動きに追従しなくなる。
  - `mirror_axis` の写し直しは近似 (最初のキーフォームの重心の高さで写した x)。
  - 評価用のメッシュを足す方法が `mesh_edit.rs`・`space.rs` (T0608)・`deformer_edit.rs`・`ugoe-wasm/src/gizmo.rs`
    に分かれている。ランタイムにデフォーマのワールド変換を読む API を足せば 1 つにまとめられる (ランタイムの公開 API の
    変更なので別タスク)。
  - ドラッグ中は子の描画が変わらない (T0608 と同じ)。回転の取っ手は拡大率によらず同じ長さ。
