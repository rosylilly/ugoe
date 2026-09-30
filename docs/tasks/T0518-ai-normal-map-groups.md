---
id: T0518
title: AI: 法線マップの生成で、部位のまとまりごとにふくらませる
status: done
milestone: M6
crates: [ugoe-ai, ugoe-rigging, apps/editor, docs]
depends_on: [T0707]
assignee: claude-worker-T0518
---

# T0518: AI: 法線マップの生成で、部位のまとまりごとにふくらませる

## 目的

T0707 の後続の候補。T0707 の見本 (`ugoe sample avatar --lit`) は、部位をまとまり (顔と目口鼻眉・首と体と腕・髪・耳) に分け、まとまりごとに 1 つの形としてふくらませることで、
目が顔の上で平らに浮いたり、首と肩に段差が出たりするのを防いだ (`crates/ugoe-cli/src/sample/lit.rs`)。
エディタの Generate と `ugoe_ai::normal` の提案は、今もパーツごとにふくらませている。

## 受け入れ条件

- [x] 意味ラベル (T0503) からまとまりを決め、まとまりごとにふくらませる生成を `ugoe-ai` (または `ugoe-rigging`) に置く。T0707 の見本の生成もそれを使うかを決める (使うなら見本のファイルが変わらないことを確かめる)
- [x] 意味ラベルごとの既定の強さと合わせる。形の外に法線を広げて輪郭に線が出ないようにする
- [x] エディタの法線マップの生成 (Lighting タブ) と AI の提案がこの方法を使う
- [x] 描いた絵を目で確かめる。`just ci`・`just test-gpu`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0707-lit-sample-avatar.md`・`docs/tasks/T0506-normal-map-gen.md` のメモ

## メモ

### やったこと

- **ugoe-rigging** (`normal/group.rs`): `group_bulge_normal_maps(members, shape_only, options)`。T0707 の `lit.rs` の方法を一般にしたもの。
  テクスチャごとに画素の中心の基準の姿のキャンバス座標 (`PlacedTexture::positions`) と `canvas_per_texel` を受け取るので、UV の反転・拡大縮小・
  基準の姿での変形も扱える。格子はまとまりの中で最も細かいテクスチャの 1 画素 (`radius`・`smoothing` はその単位)。画素の正方形が覆う升目を形にし、
  ふくらませて、画素の位置で双線形に引いてテクスチャの画素の単位に直す。メッシュが覆わない画素は `fill_missing` (ugoe-ai の深度の `fill` をここへ移した)。
  形の外に 4 画素法線を広げる `NormalMap::extend_edges` と、縮小の `NormalMap::downscale` も `lit.rs` から移した (`lit.rs` はまだ自分の複製を使う)。
  既存の関数 (`bulge_height`・`bulge_normal_map`・inflate) の署名と中身は変えていない。
- **ugoe-ai** (`normal.rs`): `SurfaceGroup` (face・body・hair_front/side/back・ear・earwear・headwear・tail・alone) と、ラベルからの対応・既定の設定
  (T0707 の見本の値。顔は縁だけを丸め、幅は形の幅の 0.21 = 見本の顔で 48 画素)。`propose_bulge_normal_maps` の設定を `BulgeNormalMapOptions`
  (`default`・`per_group`・`per_texture`・`textures`・`replace_existing`) に変えた (**公開 API の変更**。使う側はテストとエディタだけ)。
  対象がまとまりの一部でも形はまとまり全体で決める。ラベルのない・`alone` のテクスチャは単独 (`bulge_normal_map` + 縁の広げ)。
  `normal_map_group` で 1 枚のまとまりと既定の設定を返す。`classify::label_meshes_by_name` (メッシュの名前の規則だけでラベル)。
- **エディタ**: `generate_texture_normal_map(texture, settings: Option<BulgeSettings>, whole_group)` を `ugoe_ai::normal` 経由に変え、
  `get_normal_map_group(texture)` (`NormalMapGroupView`) を足した。ラベルはメッシュの名前の規則に、AI のパネルの PSD の分類 (`mesh_parts`) を重ねる。
  Lighting タブの法線マップの節 (`NormalMapSection.svelte`) だけを変えた: まとまりの表示、設定をまとまりの既定から始める・「Defaults」・「Generate for the group」。
  「Lit meshes」の一覧 (T0651 が触る所) は変えていない。`ugoe_editor_core::lighting::generate_normal_map` は画面から使わなくなった (editor-core は範囲外なので残した)。
- **docs**: `ai-integration.md#まとまりでふくらませる` (表と手順)、`editor.md#ライティングの画面`、`manual/lighting.md` (「形から作る」)、ugoe-ai・ugoe-rigging の README。
- **テスト**: rigging `tests/normal.rs` 4 件 (顔の上の目が顔の向きを受ける・形にだけ加える・解像度と左右反転の置き方・輪郭の外への広げ)、単体 2 件。
  ugoe-ai `tests/normal.rs` 2 件 (見本と同じ PSD で目・鼻・口と顔、首・腕と体の向きが 3° 未満・1 枚だけ作っても全体と同じ画像・まとまりと既定の設定)。
  エディタ `lighting::tests` 2 件。GPU の `normal_render.rs` は名前のラベルでまとまりを使うようにし、左・右・上から照らした画像も書き出す。

### 見本 (T0707) の生成について

- `lit.rs` は **変えていない** (ugoe-cli と assets は範囲外。team-lead に相談中のまま review に出した)。
- 同じまとまり・設定で共通の関数から作ると、コミットした `avatar-lit.ugoe` との差は形の中の平均で 0.02〜4° (耳飾りは 11°)、輪郭の画素で最大 110°
  (使い捨ての比較: scratchpad `T0518/litcmp`)。原因は `lit.rs` の半画素のずれ (アルファを `round(origin + t)` の升目に置き、高さは `origin + t` で引くので、
  画素の中心が .5 のとき隣の升目と半々に混ざる)。共通の関数はずれがない。寄せるなら見本の作り直し (と `avatar-lit.jpg`) が要る → 後続の候補。

### 確かめたこと

- `just ci` (main (T0713) に rebase し、`just wasm` 2 つの後にもう一度)・`just test-gpu`。`cargo check -p ugoe-editor` は ci の clippy に含まれる。
- 描いた絵 (GPU テストの書き出し、胸から上の PSD、名前のラベル、環境光 0.55 + 平行光源 0.6、高さ 45°): `/tmp/claude-1000/-home-rosylilly-src-github-com-rosylilly-ugoe/32f91561-94f7-4861-9c03-8fe8c9ff7192/scratchpad/T0518/bustup-bulge-left.png`・`-right.png`・`-top.png`
  (比べる前の絵は `bustup-unlit.png`)。光の側が明るく、首と肩・肩の上の腕の境目に段差や線は見えない。
- ヘッドレスの Chrome (SwiftShader の WebGL2、`vite preview`) と本物の `Document`・`ugoe_ai::normal` への使い捨ての HTTP ブリッジ (scratchpad `T0518/bridge`、
  `T0518/shot.mjs`) で、見本のアバターのプロジェクトを開き → Lighting タブで eyewhite-l を選ぶ (「Group “face” with face, mouth_open, …」・強さ 0.35) →
  「Generate for the group」→ neck (body、顔を形に加える・0.3)・front hair・back hair・ears-l (ears_r と)・earwear-l も同じく → 24/24 のテクスチャに法線マップ →
  光源の Direction を左・右・上にして陰影が変わる。横のはみ出し 0。画像: `/tmp/claude-1000/-home-rosylilly-src-github-com-rosylilly-ugoe/32f91561-94f7-4861-9c03-8fe8c9ff7192/scratchpad/T0518/shots/t0518-01-eye-group.png`・`t0518-03-body.png`・
  `t0518-04-{left,right,top}.png`・`t0518-05-viewport-{left,right,top}.png`。ブリッジと vite preview は止めた。

### 確かめていないこと・後続の候補

- 実際の Tauri のウィンドウ。PSD の分類 (AI のパネル) を使ったラベルの経路は単体テストだけ (ブリッジは名前の規則だけ)。
- `lit.rs` を `group_bulge_normal_maps` に寄せ、見本を作り直す (上の「見本について」)。そのとき `lit.rs` の `extend_edges`・`downscale` の複製も消せる。
- 1 枚のテクスチャをラベルの違う複数のメッシュが使うときは、モデルで先のメッシュのラベルにした (分けたほうがよい例はまだない)。
- AI の提案のパネルから法線マップの提案 (全テクスチャ) を出す画面はまだない (Lighting タブの「Generate for the group」はまとまりごと)。
- マニュアルの画像 (`docs/manual/images/lighting.png`) は古い節の見た目のまま。
- 眼鏡 (`eyewear`) は顔の形に加えず単独にした (つるが顔の外に出るため)。顔の面の向きを受けさせるには「向きは受けるが形には加えない」種類が要る。
