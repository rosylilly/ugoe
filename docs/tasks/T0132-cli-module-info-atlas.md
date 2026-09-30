---
id: T0132
title: CLI: `ugoe info` でモジュールの概要を出す・モジュールのアトラス化・見本のまとまりの設定を共有する
status: done
milestone: M4
crates: [ugoe-cli, ugoe-format, ugoe-ai, ugoe-editor-core, docs]
depends_on: [T0131, T0716]
assignee: claude-worker-T0132
---

# T0132: CLI: モジュールの概要とアトラス化

## 目的

T0131・T0716 の後続の候補をまとめる。

## 受け入れ条件

- [x] `ugoe info <m>.ugoemod` でモジュールの概要 (名前・スロット・条件・要素の数・テクスチャ) を出す
- [x] モジュールのテクスチャのアトラス化 (`build --atlas` をモジュールでも使えるようにする。モジュールの単位でまとめる。docs/spec/file-formats.md のアトラス化の節に書いた後続の課題)
- [x] 法線マップ付きの見本 (sample/lit.rs) のまとまりの設定 (GROUPS) と、`ugoe_ai` の既定のまとまりが同じ値を別々に持っている。共有するか、別のままにする理由を書く (共有すると ugoe-ai の既定を変えたときに見本も変わる)
- [x] `just ci` が通る

## 参考

- `docs/tasks/T0131-cli-build-normal-maps.md`・`docs/tasks/T0716-sample-lit-shared-inflate.md` のメモ

## メモ

- crates: に ugoe-editor-core を足した (team-lead の承認済み)。アトラス化の本体 (`build_atlas`) は editor-core にあり、
  モジュールは本体なしではコンパイルできず描画順が決まらないので、CLI だけで作るとモジュールの形の仮の Model を渡して
  「コンパイルに失敗して宣言順に落ちる」のに頼ることになるため。触ったのは atlas.rs・atlas_tests.rs と lib.rs の re-export 1 行だけ。

### やったこと

- `ugoe info <m>.ugoemod` (`crates/ugoe-cli/src/main.rs`): 名前・スロット・`requires` (アバタープロファイルのレベル・スロット・パラメータ)・
  使うパラメータ (`Module::referenced_parameters`)・パーツ/デフォーマ/メッシュ/グルーの数・物理のリグと衝突体・ポーズのグループと追従・
  テクスチャ (名前・大きさ・形式・法線マップ) を出す。`--json` は `"kind": "module"`。モデルの JSON にも `"kind": "model"` を足した (キーを足しただけ)。
  `render` にモジュールを渡したときのエラーは前のまま。
- モジュールのアトラス化: `ugoe_editor_core::build_module_atlas` を足した (`build_atlas` の中身を `pack_textures` に切り出して共有。
  `build_atlas` の動作は変えていない)。本体のアトラスには入れず、モジュールの単位でまとめる。モジュールは本体なしでは描画順が決まらないので、
  画像はメッシュの宣言順で最初に使われる順に詰める。`ugoe build <m>.ugoemod --atlas` で使う (前はエラー)。画像は名前で引くので、
  テクスチャ・法線マップの名前が重なるモジュールと PNG でないものはエラー。ファイル形式は変えていない (spec/・互換性の資料はそのまま)。
  docs/spec/file-formats.md のアトラス化の節、docs/editor.md (エディタのモジュールの書き出しではまだ選べない)、ugoe-cli の README を直した。
- 見本の `GROUPS` と `ugoe_ai::normal::SurfaceGroup` の既定は **共有しない** ことにし、理由を両方のコメントに書いた:
  見本はコミットしたファイルなので ugoe-ai の既定の調整で変わらないようにする。また、まとめ方が同じではない
  (顔の縁の幅は既定では形の幅に対する比 `FACE_EDGE`、見本は 48 画素で固定。見本はラベルでなくメッシュの名前で選び、左右の耳・耳飾りを別のまとまりにする)。
  値 (強さ・ならし) は今は同じ。見本は変わっていない。

### 確かめたこと

- `just ci`・`just test-gpu` (`CARGO_BUILD_JOBS=8`)。
- テスト: `info_describes_modules`・`build_packs_module_textures_into_an_atlas` (cli.rs)、`packs_module_textures_in_declaration_order`
  (atlas_tests.rs)、`module_atlas_does_not_change_the_rendered_image` (atlas_render_gpu.rs: 見本の long-hair・sailor・cat-ears をまとめて
  dress-up.ugoe に付けた絵が 2 つの姿で変わらない。描画は 13 回 → 10 回)。
- 見本の 4 モジュールはどれも 1 枚のアトラスになる (ファイルの大きさはほぼ同じ: 21039→21162、15240→15033、38104→34707、9691→9644 バイト)。

### 確かめていないこと・後続の候補

- ビューア・エディタの画面では見ていない (ファイルの形は同じ)。
- 法線マップを持つモジュールのテクスチャはまとめずに残る (本体と同じ規則)。法線マップ付きのモジュールの見本はない。
- 後続の候補: エディタのモジュールの書き出し (`export_module`) でもアトラス化を選べるようにする。見本の着せ替えのモジュールをアトラス化して配るか。

