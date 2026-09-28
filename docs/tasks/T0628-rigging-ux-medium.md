---
id: T0628
title: リギング操作の使い勝手 (優先度 中・低)
status: done
milestone: M3
crates: [ugoe-editor-core, apps/editor, docs/manual]
depends_on: [T0624]
assignee: claude-worker-T0628
---

# T0628: リギング操作の使い勝手 (優先度 中・低)

## 目的

操作マニュアル (T0624) の作成中に見つかった問題のうち、回避できるが手間のかかるものを直す。詳細は `docs/tasks/T0624-user-manual.md` のメモ。

## 受け入れ条件

- [x] キーの値を、最小・最大以外にも数値で入力して追加できる
- [x] キーを作った後に回転デフォーマの原点を動かすと、今のセルだけが動く。すべてのセルの原点を一緒に動かす操作を用意する
- [x] デフォーマを外す (子を親に戻して、見た目を保ったままデフォーマを削除する) 操作
- [x] 形状編集で、メッシュの頂点を複数選んで動かせる (今は 1 点かすべて)
- [x] プロジェクトを開いたとき、パラメータのスライダーの値を既定値に戻す
- [x] ヘルプのリンクをブラウザで開く (`tauri-plugin-opener` など)
- [x] 操作マニュアルの画面写真のヘッダーに出ている、確認用のバックエンドの名前 (`vbridge`) を正しい表示にする
- [x] 操作マニュアルを更新し、`just ci` が通る

## 参考

- `docs/tasks/T0624-user-manual.md` のメモ

## メモ

- **キーの値の入力** (`ParametersPanel.svelte`): 名前の右の値の表示を入力欄にした。Enter (かフォーカスを外す) でスライダーを
  その値にし、◆ でその値にキーを足す。数でない・値域の外は理由を赤い字で出して値を変えない。◆ で足す前に、キーの上と
  みなす許容差の中に別のキーがないか確かめる (重複の拒否。バックエンドの `DuplicateKey` の前に止める)。
  純粋な関数は `panels/parameterValue.ts` (Vitest)。専用の「+ 値」ボタンは作らず、スライダーを合わせてから ◆ の流れに寄せた。
- **回転の中心をすべてのセルで動かす:** 新しいコマンド `move_rotation_origin { id, delta }`。すべてのセルの原点に同じ
  `delta` (親の座標系) を足し、直接の子を基準の姿 (既定値のパラメータ) で見た目が変わらないように写し直す
  (`F_new⁻¹ ∘ F_old`、どちらもアフィンなので厳密)。**判断:** リガーが期待するのは「絵を動かさずに中心だけを動かす」なので
  子を補正する。子の格子は回転デフォーマと軸が別なので、補正できるのは基準の姿だけで、他のセルでは子が新しい中心の
  まわりに回る (テストで `後 = 前 + (I - R)·delta` を確認)。画面はギズモのバーの「Origin: all cells」か Shift + ドラッグ。
  今のセルだけを動かす従来の動作 (子ごと動く) はそのまま。
- **デフォーマを外す:** 新しいコマンド `unwrap_deformer { id }` (子を `reparent_preserving_look` と同じ規則で親へ →
  `remove_deformer`、1 つの Undo)。インスペクタの「Deformer」の節に「Unwrap」を足し (T0627 の持ち分の
  `DeformerProperties.svelte` への小さな追加)、元の子を選ぶ。
- **フォーム編集の複数選択** (`viewport/tools/form/`): クリック・Shift + クリック・要素の内側 (メッシュの三角形・ワープの格子)
  からの矩形選択・「All」/ Ctrl + A・Esc。選んだ点のドラッグで 1 つの `set_keyform_cell`。要素の外側のクリックは下の
  選択レイヤーに通す (ワープのギズモと同じ作り)。従来の「Shift + ドラッグですべて」は Shift = 追加・解除に変えた
  (すべては「All」)。選択はフォーム編集の中だけで要素ごとに持つ。補助は `form.ts` (Vitest)。
- **開いたときのスライダー:** `ViewportController.resetParametersOnNextModel()`。ファイルメニューの `onchange(replaced)` が
  New / New from PSD / Open のときに呼び、次に読み込むモデルで値を引き継がずに `reset_parameters` する。Import PSD は保つ。
  `App.svelte` (T0627 の持ち分) の `onchange` に 1 行足した。
- **Help:** `tauri-plugin-opener` 2.6.0 / `@tauri-apps/plugin-opener` 2.6.0 (どちらも MIT OR Apache-2.0。`just licenses` 可)。
  権限は `opener:allow-open-url` を `https://github.com/rosylilly/ugoe/*` だけに限った。開けなければ理由と URL を出す
  (`HelpButton.svelte`)。`App.svelte` の `<details class="help">` をこの部品に置き換えた。テスト用に `window.__UGOE_OPEN_URL__`。
- **画面写真のヘッダー:** `vbridge` はヘッドレスの確認用のブリッジが `app_info` に `"bridge"` を返していたため
  (エディタの不具合ではない)。ブリッジ (スクラッチ領域、リポジトリ外) をワークスペースの版 (`0.0.0`、実物と同じ) を返すように
  直し、マニュアルの 5 枚をすべて撮り直した (ヘッダーは `v0.0.0`・「Help」ボタン、パラメータの値の欄も新しい画面)。
- **テスト:** Rust 8 件 (`deformer_edit_tests.rs`: 中心の移動で全セルがずれて基準の姿が同じ・新しい中心のまわりに回る・
  ワープの下・拒否、外すで子が親へ・根のデフォーマ・ワープの中のワープ・ミラーの対と拒否。いずれも Undo / Redo と検証を通す)、
  JSON の往復の見本と `sample_commands`。Vitest 11 件 (`parameterValue.test.ts`、`form.test.ts`)。
- **確かめたこと:** ビルドした画面 (`vite build`、`just wasm`) をヘッドレス Chrome (SwiftShader) で開き、IPC を本物の
  `Document` に中継するブリッジにつないで操作した。値の欄の `0.5` + Enter → ◆ でキー `[0.5]`、`2` と `abc` の拒否、`0` の追加で
  `[0, 0.5]`。「Unwrap」で左目が顔のワープの子になりデフォーマが消え、左目が選ばれ、Undo 1 回で戻る。AngleZ の 3 キーの
  回転デフォーマで「Origin: all cells」のドラッグ → 3 セルとも同じ量 (0, 46.3) ずれ、子のワープの制御点が逆にずれ、
  既定値の描画はチェックボックス以外の画素が一致。Undo で戻り、チェックなしなら今のセルだけ、Shift + ドラッグなら全セル。
  AngleZ = 30 で頭が新しい中心のまわりに回る。フォーム編集で顔 (39 頂点) の内側からの矩形で 11 点、Shift + クリックで 1 点を外して
  1 点を足し、ドラッグで選んだ 11 点だけが同じ量動き (コマンド 1 つ)、Undo 1 回で元どおり。内側のクリックで解除、「All」・
  Esc・Ctrl + A。保存 → スライダーを動かす → Open で既定値に戻り、Import PSD では保ち、もう一度 Open で戻る。Help は
  差し替えありで URL を開き、なし (ブリッジが opener を知らない) では理由と URL を出す。チュートリアルの手順 (T0624 の
  スクリプト) も通し直した。
- **確かめていないこと:** 実際の Tauri のウィンドウでの操作と、opener で OS のブラウザが開くこと (権限の書式はビルド時の
  検査 (`cargo check -p ugoe-editor`) を通ることまで)。
- **後続の候補:** フォーム編集の点の選択を `Selection` に持ち、メッシュ編集の頂点の選択と共有する。回転の中心の移動を
  基準の姿以外 (今の姿) で補正する選択肢。キーの値の入力で直接キーを足す (「+ 値」) 操作。
