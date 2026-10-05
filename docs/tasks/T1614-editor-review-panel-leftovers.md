---
id: T1614
title: "エディタ: ツリーが畳んだ祖先を開き直す・法線マップの節の取り直し・着せ替えの比べる途中の結果を直す"
status: done
milestone: M9
crates: [apps/editor]
depends_on: []
assignee: claude-worker-T1614
---

# T1614: エディタ: ツリーが畳んだ祖先を開き直す・法線マップの節の取り直し・着せ替えの比べる途中の結果を直す

## 目的

M9 のレビュー (T0671・T0672) で見送った画面の不具合。

- `panels/TreePanel.svelte` の選択を見せる `$effect` が `roots` (読み直しのたびに作り直す) に依存するので、どの編集の後でも選んでいる要素の祖先を
  開き直してスクロールする (利用者が畳んだ祖先が開く)。主な選択が変わったときだけ開く (T0671)。
- `lighting/NormalMapSection.svelte` が、どの編集の後でも (`doc.revision`) テクスチャと法線マップの PNG を取り直す。テクスチャの版を持つなどで減らす (T0671)。
- `preview/dressUp.svelte.ts` の `load` (プロジェクトを開き直す) が比べている途中の `#compareFiles` を捨てない (古い結果が `highlight` を書き換えうる)。`#compareToken` を進める (T0672)。

## 受け入れ条件

- [x] 3 つそれぞれに、先に失敗するテストを書いてから直した (画面の操作で書けないものは理由をメモに)
- [x] `just ci` が通る

## 参考

- T0671・T0672 のメモ (所見の表と起票の候補)

## メモ

- T0026 で起票した (元の候補: T0671 のツリーと法線マップの節、T0672 の `DressUp.load`)。オーケストレーターの集計には無かったが、M9 の done のタスクのメモの起票の候補にあった。

### やったこと (T1614)

どれも先に失敗するテストを書いてから直した (失敗を確かめた)。

- **ツリー (`panels/TreePanel.svelte`):** 祖先を開いた主な選択 (`表示の種類/key`) を覚え、同じものでは開き直さない。
  ツリーにまだ無い要素 (足した直後に選んだもの) は、読み直して現れたときに開く。選択が無くなったら覚えを消す。
  部品のテスト `panels/TreePanel.test.ts` (happy-dom。畳んだ祖先が編集の後の読み直しで開かないこと・主な選択が変われば開くこと・
  後から現れた要素を開くこと)。
- **法線マップの節 (`lighting/NormalMapSection.svelte`):** バックエンドに `get_texture_versions(names)` (画像の中身のハッシュ。
  無い画像は null) を足し、編集のたびには版だけを問い合わせ、名前か版が変わった画像だけを `get_texture_png` で読み直す
  (同じ名前の画像の差し替え・Undo も映る)。使わなくなった blob の URL は捨て、閉じるときにも捨てる。
  Rust のテスト `lighting::tests::texture_versions_change_with_the_image` と部品のテスト `NormalMapSection.test.ts`。
  IPC の追加に合わせて `acl.rs`・`capabilities/default.json`・`lib.rs`・`ipc.ts` と `docs/editor.md` (ライティングの画面と IPC の表) を直した
  (`docs/editor.md` は `crates:` の外だが、IPC と同じ PR で直す規約の小さな変更)。
- **着せ替え (`preview/dressUp.svelte.ts`):** `load` で `#compareToken` を進め、`highlight` を消す。テストは `dressUp.test.ts`
  (比べ直している途中に開き直し、遅れて届いた古い結果が強調を書き換えないこと)。
- テストの道具: `FakeDocument` (`viewport/tools/test/harness.svelte.ts`) に `revision` を足した。

### やらなかったこと

- 実機 (Tauri の WebView) では動かしていない (happy-dom の部品のテストと Rust のテストで確かめた)。
- 版はハッシュなので、編集のたびに画像 2 枚分のハッシュをバックエンドで計算する (PNG の転送と画像の読み込みは無くなった)。
  `Arc` の同一性で版を決めればハッシュも省けるが、解放後の番地の使い回しで取り違えうるので採らなかった。

### 起票の候補

- なし。
