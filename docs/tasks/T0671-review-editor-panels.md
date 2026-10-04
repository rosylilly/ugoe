---
id: T0671
title: コードレビュー: エディタのフロントエンド (パネル)
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T0671
---

# T0671: コードレビュー: エディタのフロントエンド (パネル)

## 目的

エディタのフロントエンド (パネル) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `apps/editor/src/lib/panels/` のすべて (ツリー・インスペクタ・パラメータ・物理・ライティング・スロット・グルー・ポーズ・タイムライン・検証)

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - パネルの間で重複した部品・処理、`@ugoe/ui` で置き換えられる自前の部品
  - 数値の入力の onchange (docs/ui.md) と Undo の粒度 (ドラッグ中に何個のコマンドが積まれるか)
  - 大きいパネル (inspector・timeline・physics) の分け方
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0693〜T0696 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

`apps/editor/src/lib/panels/` のすべて (81 ファイル。テストを含む) を読んだ。場所は `panels/` からの相対パス。

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 中 | `inspector/inference/InferenceSection.svelte` | 推定の設定のフォーム・ミラー元の選択・左右の対の欄を `$effect` で作り直していたが、格子は読み直しのたびに別のオブジェクトになるので、関係のない編集 (セルの由来の変更・キーの追加など) のたびに編集中のフォームが消え、ミラー元が先頭 (対の相手) に戻っていた。「Mirror from itself」で実行した後にもう一度押すと、相手からミラーしてしまう | 直した (読み直しで変わらない鍵 `inferenceFormKey`・`targetKey` の変化でだけ作り直す。鍵のテストを追加) |
| 中 | `inspector/depth/depth.ts` `depthRange`、`InferenceSection.svelte` の既定の中心 | `Math.min(...depth)` で点を引数に広げていた。JavaScriptCore (WebKitGTK・WKWebView) は引数が 65536 個までなので、頂点の多いメッシュでパネルが例外で止まる | 直した (1 つずつ見る。`defaultPivot` を inference.ts に移した。30 万点のテストを追加) |
| 中 | `physics/*`・`glues/GluesPanel.svelte`・`poses/PosesPanel.svelte`・`inspector/depth/DepthSection.svelte`・`lighting/NormalMapSection.svelte` | 適用中 (`busy`) の編集を何も言わずに捨てる。欄は入れた値を出したままになる。待たせるだけでは、古い写し (`rig`・`world`) から組み立てたコマンドが前の編集を戻す。成否を `doc.error` で見るパネルと `apply` の戻り値で見るパネルが混在 | T0693 を起票 |
| 中 | パネル全体 (`JSON.parse(doc.model.json)` が 10 か所以上) | コマンドを 1 つ適用するたびに、開いているパネルの数だけ同じモデルの JSON を解析する (大きなモデルで編集のたびの待ちが伸びる)。try / catch の同じ処理も重複 | T0694 を起票 |
| 中 | 数値の欄 (物理・グルーの強さ・描画順・タイムラインなど) | `NumberField` は矢印キー 1 回ごとに確定し、パネルはそのたびにコマンドを送るので、押し続けると Undo の履歴が 1 回ごとに積まれる (スライダーは `oncommit` なので起きない。キー・取っ手のドラッグは離したときに 1 つ) | T0695 を起票 (まとめる場所は設計の判断) |
| 中 | `poses/PosesPanel.svelte` | 要素の並べ替えに HTML5 のドラッグ & ドロップを使う。Tauri のファイルのドロップを有効にしているので、Windows (WebView2) では届かないはず (ツリーはこのためにポインタで実装している) | T0696 を起票 |
| 低 | `verify/VerifyPanel.svelte` | 今のセルの印をパラメータの値とキーの完全一致で決めていた。値は WebPuppet の f32 を経由するので、0.1 のようなキーのセルは印が付かない | 直した (`isAtCell`。キーフォームの格子と同じ許容差。テストを追加) |
| 低 | `timeline/CurveGraph.svelte` | ベジェの取っ手を押して動かさずに離すと、何も変えない `set_motion_segment` を送り、Undo の履歴が 1 つ増える | 直した (`segmentHandle` で元の取っ手と比べる。テストを追加) |
| 低 | `InspectorPanel.svelte`、locales | 何も選んでいないときの文に、作りかけのときの `(rev N)` (ドキュメントの版) が残っていた | 直した (マニュアルの「Nothing selected」に合わせた) |
| 低 | `ParametersPanel.svelte` `saveEditor` | `set_parameter_range` に `name` も入れて送っていた (バックエンドが無視するだけ) | 直した |
| 低 | `TreePanel.svelte` の見出し、`tree/edit.ts`、`inspector/inference/OriginLegend.svelte` | 古いコメント (付け替えは見た目を保たない・T0607 待ち)、型から外れた doc コメント、無い名前 `ORIGIN_STYLES` | 直した |
| 低 | `timeline/MotionEditor.svelte`・`CurveGraph.svelte` | `.visually-hidden` を自前で定義 (`@ugoe/ui` の `ugoe-visually-hidden` と重複) | 直した |
| 低 | `glues/glues.ts` | 描画用の複製のグルーの接頭辞を `drawClone.ts` と別に定義 | 直した (`GLUE_PREFIX` を使う) |
| 低 | `TreePanel.svelte` の選択を見せる `$effect` | `roots` (読み直しのたびに作り直す) に依存するので、どの編集の後でも、選んでいる要素の祖先を開き直してスクロールする (利用者が畳んだ祖先が開く) | 見送り (起票の候補。画面の操作の不具合で、今の検査の道具では先に失敗するテストを書けない) |
| 低 | `lighting/NormalMapSection.svelte` | テクスチャと法線マップの PNG を、どの編集の後でも (`doc.revision`) 取り直す | 見送り (起票の候補。同じ名前のテクスチャの差し替えも映すために版を見ている。テクスチャの版を持てば減らせる) |
| 低 | `inspector/drawClone/drawClone.ts` `isInSync` | `ugoe_editor_core::draw_clone::is_in_sync` と同じ規則を TS で書き直している (今は一致) | 見送り (バックエンドの IPC を足せば 1 つにできる。範囲の外) |
| 低 | `inspector/inference/inference.ts` `describeWarning` | 作り直しの失敗の理由 (`detail`) はバックエンドの英語の文のまま差し込む | 見送り (`InferError` にコードが無い。横断に記録) |
| 低 | `glues/glues.ts` `keyValue` | 軸に同じキーがあるかを完全一致で見る (パラメータのパネル・キーの格子は許容差つき) | 見送り (グルーの強さのキーは欄に打った値なので実害は小さい。T0693 で送り方をそろえるときに合わせる候補) |
| 低 | `lighting/PreviewLightsSection.svelte` | 「Reset lights」と光源の削除は project.json に残す設定で Undo が無いのに、確かめずに消す (docs/ui.md の「取り消せない操作」) | 見送り (M10 の T0674 で見る) |
| 低 | `ParametersPanel.svelte` (値の欄・ネイティブのスライダー)、`AvatarSection`・`DeformerProperties`・`RigEditor`・`ColliderSection`・`SlotsPanel`・タイムラインの名前の欄、`InferenceSection`・`GluesPanel`・`CurveGraph` の空にできる数値の欄、`LightingPanel` のチェック | `@ugoe/ui` に無い部品 (文字の欄・空にできる数値の欄) を自前の `<input>` と同じ CSS で何か所にも書いている | 見送り (M10 の T0674。`@ugoe/ui` に `TextField` と空を許す `NumberField` を足す候補) |
| 低 | `tree/NodeIcon.svelte`、`inspector/inference/inference.ts` `ORIGIN_MARKS`、`InferenceSection.svelte` の `.source` | 色を直に書いている (トークンの外) | 見送り (M10 の T0674) |
| 低 | `physics/ColliderSection.svelte` の一覧、`poses/PosesPanel.svelte` の ▼ | 取り付け先を表示名でなく ID で出す。▼ にだけツールチップが無い | 見送り (M10 の T0674) |
| 低 | `ParametersPanel`・`PosesPanel`・`KeyformInspector` (`withClick`) | `Tooltip` のトリガーの `onclick` を手で重ねている (`@ugoe/ui` の `mergeAttributes` で書ける) | 見送り (動きは同じ。部品を移すときに) |

確かめたこと (受け入れ条件の追加の観点):

- **重複と `@ugoe/ui`:** 上の表のとおり。パネルの間の重複は、モデルの解析 (T0694)・`send` と `busy` (T0693)・文字の欄の CSS (M10)。
  `PickList` はスロットとポーズで共有済み、`RestPoseWarnings` は物理と検証で共有済み。
- **数値の入力と Undo の粒度:** スライダー (グルーの重み・光源の強さ・法線マップ) は `oncommit` でだけ送り、ドラッグ中は表示だけ。
  パラメータのスライダーは表示の状態でコマンドを送らない。キー・取っ手・ツリーのドラッグは離したときに 1 つ (動かさない取っ手は直した)。
  数値の欄は確定ごとに 1 つで、矢印キーを押し続けると積み上がる (T0695)。
- **大きいパネルの分け方:** インスペクタは節ごとの部品 (`inspector/*`)、物理は `RigEditor`・`ColliderSection`・`TemplateSection`、
  タイムラインは `MotionEditor`・`CurveGraph`・`ExpressionEditor` に分かれていて、純粋な処理は `*.ts` にありテストされている。
  いちばん大きい `MotionEditor.svelte` (745 行) もドープシートと棒の 2 つの塊で、今は分けなくてよいと判断した。

### 起票の候補 (枠の T0693〜T0696 を使い切ったので、オーケストレーターが起票する)

- ツリーの選択を見せる `$effect` が読み直しのたびに走り、畳んだ祖先を開き直す (`TreePanel.svelte`)。主な選択が変わったときだけ開く。
- 法線マップの節が、どの編集の後でもテクスチャの PNG を取り直す (`NormalMapSection.svelte`)。
- M10 (T0674) へ: 「Reset lights」・光源の削除の確かめ、自前の文字の欄と空にできる数値の欄 (`@ugoe/ui` に足す候補)、直に書いた色、
  衝突体の取り付け先の表記、ポーズの ▼ のツールチップ。

### 横断 (T0008 向け)

- **読み直しのたびに作り直すオブジェクトに `$effect` で反応する:** モデルは JSON から毎回作り直すので、`$derived` のオブジェクトに依存する
  `$effect` は関係のない編集のたびに走る。ローカルの入力の状態を作り直す `$effect` は、文字列などの変わらない鍵に依存させる (T0671 で直した
  InferenceSection、候補のツリーの選択の表示)。ほかのアプリ・ビューポートの道具にもありそう。
- **`Math.min(...配列)` / `Math.max(...配列)`:** JavaScriptCore は引数が 65536 個までなので、頂点などの長い配列で例外になる。
  残っている所: `apps/editor/src/lib/preview/differenceHighlight.ts:93`、`packages/face-tracking/src/pose.ts:71` (こちらは短い配列)。
- **f32 を経由した値の完全一致の比較:** WebPuppet から読み直したパラメータの値をキーと `===` で比べると外れる。`keyforms.ts` の `keyTolerance` を使う。
- **`busy` で編集を捨てる `send` と `doc.error` での成否の判定:** パネルごとに書き方が違う (T0693)。
- **`.visually-hidden` の自前の定義:** `apps/editor/src/lib/ui/TabStrip.svelte`・`apps/viewer/src/lib/MemoryDialog.svelte` にも残っている (`ugoe-visually-hidden` がある)。
