---
id: T0656
title: エディタ: モジュールの違いをメッシュの中身まで出す・違いの行からツリーで選ぶ
status: done
milestone: M4
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: [T0654]
assignee: claude-worker-T0656
---

# T0656: モジュールの違いの詳細

## 目的

T0654 の後続の候補をまとめる。

## 受け入れ条件

- [x] 今の編集と書き出したモジュールの違いを、メッシュの中身 (頂点・キーフォーム・描画順) まで出す。アトラス化したファイルとは UV を元に戻して比べる
- [x] 違いの行から、その要素をツリーで選べる
- [x] 見本のモジュールをアトラス化して配るかを決める (決めたら assets/samples と見本の生成を直す)
- [x] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0654-editor-module-compare-details.md` のメモ

## メモ

- **中身の違い:** `compare_with_module_file` の戻り値を `Vec<ModuleDifference>` (`text` と `items`) にした。行の文言は T0654 のまま。
  足した・変えたパーツ・デフォーマ・メッシュを `DifferenceItem` (`kind`・モジュールの中の `id`・`text`・作業用のモデルの `element`) で
  添える。メッシュは名前・パーツ・親・テクスチャ (名前)・頂点 (数か、UV の変わった頂点の数)・三角形・キーフォーム (軸・数・頂点の位置の
  変わったキーフォームの数)・描画順・不透明度と色・合成とマスク・その他、デフォーマは名前・パーツ・親・種類・格子・キーフォーム、パーツは
  名前・親・不透明度。描画順は `ModuleContext::map_draw_order` でスロットの範囲に写し (ツリー・インスペクタと同じ値)、小数 2 桁で丸めて出す
  (`module_differences` に写し方を渡す)。除いた要素は今の編集に無いので `items` に入れない。
- **アトラス化したファイル:** `build_module_atlas` の対応表はファイルに残らないので、比べる側で戻す (`FileView`・`Finder`)。`atlas-` で始まり
  法線マップが無く、今の編集に同じ名前が無いテクスチャをアトラスとみなす。同じ ID のメッシュの UV の組から置いた左上の画素を推し量って
  多数決、その場所の画素が今の編集の画像と同じかを確かめる。確かめられず過半数にも届かなければ、メッシュの UV の範囲を覆う位置を画素で
  探す (不透明な画素 32 個を先に見る。試す位置は 65536 まで)。除いたメッシュも、見つけた場所の中か、まだ見つけていない今の編集の画像を
  画素で探して戻す。UV は今の編集の UV を `Placement::map` (書き出しと同じ計算。atlas.rs で `pub(crate)` にし、逆の `unmap` を足した) で
  写した値がファイルと同じならその UV にするので、丸めのずれは出ない。使うメッシュをすべて戻せたアトラスはテクスチャとして比べず、
  戻せなければ注記 (「Some meshes in the file's texture atlases could not be matched …」)。T0654 の「アトラス化してある」の注記は除いた。
- **ツリーで選ぶ:** 違いの要素の ID をボタンにし、押すとプレビューを出てツリーで選ぶ (`differenceRef`。`App` → `PreviewPanel` →
  `DressUpSection` の `onselect`)。ツリーはプレビューの間は隠れる (`.shell.previewing .left`) ので、出てから `selection.set`。
  入り直すと付けていたモジュールは付き直る。
- **見本のモジュールはアトラス化して配らない** (assets/samples と `ugoe sample` は変えない)。理由: 見本は開いて読む・直す教材で、
  `ugoe info` やエディタでテクスチャが層ごとの名前で見える方が分かりやすい。大きさはほとんど変わらない (T0132: 21039→21162、15240→15033、
  38104→34707、9691→9644 バイト)。描画のまとまりは付ける側 (本体と合わせて 13 → 10 回) の話で、小さな見本では効かない。アトラス化は
  `ugoe build <m>.ugoemod --atlas` とエディタの書き出しの設定で誰でも試せ、見本のプロジェクト (`--project`) から書き出したものと比べても
  今回の比較で「同じ」と出る。
- **IPC:** `compare_preview_module` の戻り値を `ModuleDifference[]` に (ipc.ts・`just gen` の types.ts に `ModuleDifference`・
  `DifferenceItem`・`DifferenceKind`)。Command の enum は足していない (`OP_COUNT` は 101 のまま)。
- **テスト:** editor-core `module_compare_tests.rs` (変えたメッシュの詳細と `element`・描画順はスロットの範囲・頂点の数・足した要素は選べて
  除いた要素は添えない・アトラス化したファイルと同じ/UV 1 つ/画像/メッシュを除く・UV をすべて変えても画素で探す・画像も UV も違えば注記)、
  Vitest (`differenceRef`・`DressUpEntry.differences` の形)、Tauri の `preview_module_is_compared_with_the_exported_file` を新しい型に。
- **確かめたこと:** main (T0405・T0520・T0408) に rebase 後、`just wasm` 2 つ・`just gen` (差分なし)・`just ci`・`cargo check -p ugoe-editor`・
  `just test-gpu`。ヘッドレスの Chrome (SwiftShader) と本物の `Document` への使い捨ての HTTP ブリッジ (scratchpad/T0656/bridge。T0654 のもの
  をそのまま、ポートだけ変えた) で `ugoe sample dress-up --project` の cat-ears.ugoeproj: アトラス化して書き出す (1 枚 228×184) →
  プレビューで見本の cat-ears.ugoemod は「Same」→ エージェントの編集で Band の描画順を 990・頂点を 1 つ動かし Ears を消すと
  「Meshes: 2 → 1 (removed: Ears; changed: Band)」と「Band: vertex positions in 1 keyform; draw order 1000 → 990」→ 「Band」を押すと
  プレビューを出てツリーの CatEars/Band が選ばれ、インスペクタに Band → アトラス化したファイルを読み込んでも同じ 2 行 (テクスチャの行も
  注記も出ない) → 開き直すとアトラス化したファイルとも「Same」。横のはみ出し 0。
  スクリーンショット: scratchpad/T0656/shots/t0656-*.png (マニュアルに module-compare-details.png を足した)。vite preview とブリッジは止めた。
- **確かめていないこと:** 実際の Tauri のウィンドウ。大きなアトラス (2048 四方) での画素の探索の重さ (UV の組で決まれば探さない)。
  ほとんど透明・一様な画像が多いアトラスで、画素だけで探して別の場所に当たること (UV の組の多数決が先)。今の編集でスロットを変えたときの
  描画順の表示 (ファイルの値も今のスロットで写す)。法線マップを持つモジュール (アトラスにまとめないので影響なし)。
- **範囲外の変更:** なし (editor-core・apps/editor・docs だけ)。
- **後続の候補:**
  - 物理のリグ・衝突体・グルーの違いも中身まで出す (今は「changed」だけで、ツリーで選べない。Physics タブで選ぶ形が要る)。
  - 違いの要素を選ぶとき、プレビューを出ずにビューポートで強調する (今はツリーが隠れるので出る)。
  - `.ugoemod` にアトラスの対応表 (元の名前と置いた場所) を残せば、画素で探さずに戻せる (ファイル形式の変更になるので、要るなら ADR)。
