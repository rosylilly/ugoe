---
id: T0315
title: 物理: 静止の姿で衝突体に入っている髪を知らせる
status: done
milestone: M4
crates: [ugoe-cli, ugoe-editor-core, apps/editor, docs]
depends_on: [T0311]
assignee: claude-worker-T0315
---

# T0315: 物理: 静止の姿で衝突体に入っている髪を知らせる

## 目的

T0311 で足した `PhysicsSystem::rest_pose_collisions` を、利用者が気づける場所につなぐ。

## 受け入れ条件

- [x] `ugoe validate` が、静止の姿で衝突体に入っている鎖を警告として出す (エラーにはしない)
- [x] エディタの Verify タブ (または物理の画面) に同じ警告を出し、該当の鎖と衝突体を選べる
- [x] PSD の取り込みや物理のテンプレートの適用の直後にも知らせるかを決めて書く
- [x] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0311-physics-collision-robustness.md` のメモ
- `docs/spec/physics.md`

## メモ

- **検出** (`crates/ugoe-editor-core/src/physics_check.rs`): `rest_pose_collisions(model, physics)` がモデルをコンパイルし、既定値で
  `PhysicsSystem::rest_pose_collisions` を呼び、リグと衝突体の組ごとに `RestPoseCollision { rig, collider, particles }` にまとめる。
  ugoe-physics は変えていない。editor-core に `ugoe-physics` の依存を足した (ワークスペースの既存のクレート。`just layers` は通る)。
- **CLI:** `ugoe validate` がバンドルの物理 (または新しい `--physics <PATH>`) を `validate_physics` で検査し (エラー)、エラーがなければ
  静止姿勢の当たりを `warning:` として標準エラーに出す (終了コードと `ok` は変えない)。T0511 と衝突しないよう validate の枝と新しい
  `physics.rs` だけを触った。
- **エディタ:** Tauri コマンド `check_physics` (`src-tauri/src/physics_check.rs`、`ipc.ts` の `checkPhysics`)。一覧は
  `panels/physics/RestPoseWarnings.svelte` (表記は `restPose.ts`、Vitest) で、ドキュメントの版が変わるたびに調べ直す。「Verify」の
  「Physics」と物理のパネル (「Preview」の下、当たりがあるときだけ) に出す。行を押すとリグと衝突体を選ぶ (Verify からは Physics のタブへ移る)。
- **取り込み・テンプレートの直後:** 別の通知は足さないことにした (docs/editor.md#物理の警告 に理由)。テンプレートの適用・衝突体の編集は
  物理のパネルで行い、その場の一覧がすぐ更新される。PSD の取り込みは物理を作らない。`ApplyReport` に足すとすべてのコマンドの後に評価が要る。
- **確かめたこと:** vite build + `just wasm` + 本物の `Document` への HTTP ブリッジ (スクラッチ領域、リポジトリ外。`fixtures::dynamic_hair`
  に肩を髪の上へ動かした物理) + ヘッドレス Chrome で、Verify に `髪 (Hair) hits 肩 (Shoulder) / segment 2`、行を押すと Physics タブで
  髪のリグと肩が選ばれること、「Delete collider」で両方の一覧から消え「No rig hits a collider in the rest pose.」になることを確認。
- **確かめていないこと:** 実際の Tauri のウィンドウでの表示。マニュアルの画像は撮っていない。
- **後続の候補:** 既定値以外の姿 (表情や体の向きの初期値を持つモデル) での検査。`ugoe validate --json` があれば警告も構造化して出す。
