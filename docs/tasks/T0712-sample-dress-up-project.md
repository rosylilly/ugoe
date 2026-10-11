---
id: T0712
title: 見本: 着せ替えの見本をエディタで開けるプロジェクトとしても出力し、本体にアバタープロファイルを持たせる
status: done
milestone: M4
crates: [ugoe-cli, assets, docs]
depends_on: [T0710, T0707]
assignee: claude-worker-T0712
---

# T0712: 見本: 着せ替えの見本のプロジェクトとアバタープロファイル

## 目的

T0710 の後続の候補。着せ替えの見本 (`ugoe sample dress-up`) は `.ugoe` と `.ugoemod` だけで、エディタでモジュールのプロジェクトの見本として開けない。
また見本の本体はアバタープロファイルを名乗っておらず、スロットの「アプリに見せる」の見本にならない。

## 受け入れ条件

- [x] `ugoe sample dress-up` に、本体の `.ugoeproj` と、それを参照するモジュールのプロジェクトを出力する形を足す (`--project` など)。出力したプロジェクトをエディタの中核 (`ugoe-editor-core`) で開けることをテストで確かめる
- [x] 見本の本体にアバタープロファイルを持たせる (準拠レベルの検査を通す)。既存の見本の出力を変えるなら、コミットしたファイルを作り直して理由をメモに書く
- [x] マニュアル (`docs/manual/dress-up.md`) から見本のプロジェクトに触れる
- [x] `just ci` が通る

## 参考

- `docs/tasks/T0710-dress-up-sample.md` のメモ (sample.rs は T0707 も触るので、T0707 の後に着手する)

## メモ

- **やったこと**
  - `ugoe sample dress-up --project <DIR>` (`DressUpSample::save_projects`): 本体 `dress-up.ugoeproj` と、それを参照するモジュールのプロジェクト
    `long-hair`・`twin-tails`・`cat-ears`・`sailor` (`.ugoeproj`) を同じディレクトリに並べて書く (`base` は `../dress-up.ugoeproj`)。
    形式は `ugoe_editor_core::Project` が書く: 本体は `Project::new` で保存し、モジュールは保存した本体から `Project::new_module`
    (エディタの「New Module…」と同じ) で作り、`ModuleContext::reconfigured`/`workspace` で見本のモジュールを入れて保存する。
    プロジェクトはリポジトリにコミットしていない (`.ugoe`・`.ugoemod` と同じ中身で、いつでも作り直せるため)。
  - **見本の本体のアバタープロファイル:** レベル 1 (パラメータはもとからレベル 1 の分だけ。レベル 2 には視線・眉・口の形・体の Y/Z が要る)。
    メタデータと許諾は見本のアバターと同じ形 (MIT、誰でも使える)、`slots` に `Hair`・`Headwear`・`Outfit` をすべて書いてアプリに公開する。
    あわせて当たり判定 `Head` (`Face`)・`Body` (`Skin`) と `avatar.hit_areas` を足した。生成の中の検査 (`check`) で `check_profile` の
    問題 (警告も) が無く、主張したレベルを満たすことを確かめる。
  - **コミットしたファイルの作り直し:** 本体にプロファイルと当たり判定を足したので `assets/samples/dress-up/dress-up.ugoe` を作り直した
    (モジュールの `.ugoemod` は変わらない。絵も変わらないので `dress-up.jpg` はそのまま)。`ugoe validate --profile avatar` は
    「claims level 1, satisfies level 1」。
  - テスト (`sample::dress_up::tests`): `base_conforms_to_the_avatar_profile`、`projects_open_in_the_editor_core` (書いたプロジェクトを
    `Project::load` で開け、本体は見本の本体と同じモデル・テクスチャ、モジュールのプロジェクトは `kind: module`・本体の相対パス・スロットを持ち、
    `export_module` の結果が見本の `.ugoemod` と **バイト列で** 同じ)、`module_projects_convert_coordinates` (モジュールのプロジェクトの
    作業用のモデルで、モジュールのメッシュと回転デフォーマに `space::local_to_canvas`/`canvas_to_local` が使え、評価の結果と一致し往復で戻る)。
  - 文書: `assets/samples/README.md` (dress-up の節に当たり判定・アバタープロファイルの行、「エディタで開くプロジェクト」の節)、
    `docs/manual/dress-up.md` (「見本のプロジェクトを開く」の節と冒頭・「見本で試す」からの案内)、`crates/ugoe-cli/README.md` (`--project`)。
- **T0517 の修正の確認:** `module_projects_convert_coordinates` は、ugoe-rigging の `ancestors_only` の修正 (`slots.clear()`・`avatar = None`) を
  一時的に戻すと `Evaluate("model failed validation: [UnknownReference … slot Hair … Skin, … slot Outfit … BodyWarp]")` で失敗し、戻すと通る
  (戻したのは確認の間だけで、コミットには含めていない)。見本のモジュールのプロジェクトで座標変換が通ることを確かめた。
- **確かめたこと:** `just ci` (main (T0517) に rebase し `just wasm` 2 つの後。その後の main は tasks の変更だけ)。
  ヘッドレスの Chrome (SwiftShader) と本物の `Document`/`Project` への使い捨ての HTTP ブリッジ (scratchpad/T0712/bridge。T0517 のものを
  ポートとパスだけ替えた) で、`--project` で書いたプロジェクトを「Open…」で開いた: 本体 (インスペクタにアバタープロファイル「1 — Basic」、
  Slots タブに 3 つのスロットと `app` の印、`check_avatar_profile` はエラー・警告なし) → `long-hair.ugoeproj` (`Module: Hair`、本体が下敷き、
  `AngleX` = 30・`AngleZ` = 20 で髪が頭と一緒に動く) → `sailor.ugoeproj` (`Module: Outfit`、`BodyAngleX` = 10) → `cat-ears.ugoeproj`
  (`Module: Headwear`) を「Export Module」相当で書き出すと、コミットした `cat-ears.ugoemod` と `cmp` で同じ。横のはみ出し 0。
  スクリーンショット: scratchpad/T0712/shots/t0712-0{1..6}-*.png。
  ビューア (ビルド済みの `dist/` を `vite preview`) で「サンプル」→「着せ替え」が今までどおり開け (スロット 3 つ、`LongHair*`・`CatEars*`・`Sailor*`、
  警告なし)、「差し替える」で `TwinTails` に替わる: scratchpad/T0712/viewer/shots/0{1..4}-*.png。読み込み時の 404 が 1 件出るのは T0710 と同じ
  (見本のファイルではない)。使ったブリッジと vite preview は止めた。
  (scratchpad は作業したエージェントの一時ディレクトリ。もう残っていない)
- **確かめていないこと:** 実際の Tauri のウィンドウ。`just test-gpu` (描画・構造体の変更なし) は流していない。エディタでの編集・保存の往復
  (開いて書き出すまで)。ビューアでアバタープロファイルの `slots` を使う画面は無い (ビューアは公開の有無を見ていない)。
- **範囲外の変更:** なし (ugoe-cli・assets・docs だけ)。
- **後続の候補:**
  - 見本の本体をレベル 2 にする (視線・眉・口の形・`BodyAngleY/Z` のパラメータと絵の動きを足す)。
  - ビューアが `avatar.slots` を読み、公開していないスロットを利用者に見せない (spec の推奨。今の見本はすべて公開なので変わらない)。
  - 見本のプロジェクトをビューアの「サンプル」のようにエディタから開く入口 (今は CLI で書き出して「Open…」)。
