---
id: T0522
title: AI: 法線マップの提案の仕上げ (縁の幅の単位・丸みとならし・提案の対象の解析・英語の題名)
status: done
milestone: M6
crates: [ugoe-ai, ugoe-editor-core, apps/editor, docs, ugoe-mcp]
depends_on: [T0520]
assignee: claude-worker-T0522
---

# T0522: 法線マップの提案の仕上げ

## 目的

T0520 の後続の候補をまとめる。

## 受け入れ条件

- [x] 「Edge width (px)」の単位が、まとまりの格子の画素で、キャンバスの px と紛らわしい (見本の顔で 148 と出るが、キャンバスでは約 67 px)。キャンバスの px で見せるか、単位を明示する
- [x] AI タブで、縁の丸みとならしも選べるようにする
- [x] editor-core の proposal::analyze が set_normal_map の対象のテクスチャを拾わない (T0520 では区切りの subjects で補った)。analyze を直す
- [x] 提案の題名と理由が、英語の画面に日本語のまま出る (T0637 からの課題)。英語にする
- [x] 眼鏡のほかに向きだけを受けたいラベルがあれば orientation_only に足す
- [x] `just ci`・`just test-gpu`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0520-ai-normal-map-panel.md` のメモ

## メモ

### やったこと

- **縁の幅の単位 (キャンバスの px):** 内部の値 (`BulgeSettings` の `edge`・`smoothing`、`BulgeOptions` の `radius`・`smoothing`) は今までどおりまとまりの格子の画素のまま。
  `ugoe_ai::normal::NormalMapGroup::canvas_per_pixel` (格子の 1 画素のキャンバスの px = 形に加わるテクスチャ (members と shape_only) のうち最も細かいものの
  `canvas_per_texel`。`ugoe_rigging::group_bulge_normal_maps_oriented` の格子と同じ規則。単独 (`alone`・ラベルなし) は `None`) を足し、IPC の
  `NormalMapGroupView::canvas_per_pixel` で画面に渡す。**変換: 画面の値 = 格子の画素 × canvas_per_pixel、送る値 = 画面の値 ÷ canvas_per_pixel**
  (`apps/editor/src/lib/panels/lighting/lighting.ts` の `bulgeForm`・`bulgeSettings`。単独のものは 1 倍で「(texture px)」と出す)。測るのは画像をデコードせずに
  (`png_size` と三角形の面積。`TexelCanvas::rasterize` と同じ `canvas_per_texel` を共有の関数にした)。
  見本のアバターの顔は格子の 144 画素 = **キャンバスの 48 px** (T0520 のメモの「約 67 px」は見積もりの誤り。設計の「形の幅 約 230 px の 0.21 = 48」と一致)。
  ならしも同じくキャンバスの px で見せる (顔 6 画素 = 2.0 px)。提案の理由も縁とならしをキャンバスの px で書く。
- **AI タブ:** まとまりの行の下に開閉 (「edges 48 px · smoothing 2.0 px」) を置き、「Round edges only」・「Edge width (px)」・「Smoothing (px)」・「Defaults」。
  選び方はキャンバスの px で覚えるので、分類し直して格子が変わっても見た目の幅は変わらない。Lighting タブにも「Smoothing (px)」を足した (状態はあったが欄がなかった)。
- **proposal::analyze:** `subject` が `texture` (文字列) を拾う (`set_normal_map` だけが該当)。エディタの `CommandSection::subjects` は要らなくなったので除き
  (IPC の型の変更。`just gen` で types.ts)、一覧は区切りの中のコマンドを `ProposalCommand::subject` 付きで出す。
- **英語の題名と理由:** ugoe-ai の自動リギング・モジュールのリグ (ポーズの行を含む)・振り向き・法線マップ (形・深度) の題名と理由、エディタの
  `proposals.rs` が足す行 (AI に問い直した回数・確からしさが低いレイヤー) を英語にした。AI の答えの `reason` はそのまま (録画は変えていない)。
  分類 (classify) は提案を作らない (ラベルの規則の日本語のレイヤー名は入力の照合なので残す)。テストの `contains` を英語に合わせた。
- **orientation_only:** 眼鏡のほかは足さなかった。見本のアバターでまつ毛・眉はメッシュの範囲でも顔の輪郭の内側 (eyelash-r 385〜 / face 383〜) で、
  形に加えても輪郭を広げない。顔の上に描くが外にはみ出すものの例がないため、決める根拠がない。
- docs: editor.md (ライティングの画面の「長さの単位」・法線マップの提案の「選ぶ」と「区切り」・analyze の対象)、ai-integration.md、manual/ai.md・lighting.md、
  マニュアルの画像 `ai-normal-maps.png`・`lighting.png` を撮り直した。
- テスト: ugoe-ai `groups_tell_the_canvas_size_of_their_pixels`、editor-core `normal_map_commands_name_their_texture`、エディタの lighting の
  `canvas_per_pixel`・区切りの対象を `subject` で、Vitest (lighting の変換 2 件・AI タブの選び方の変換)。
- Command の enum は足していない (OP_COUNT は 101 のまま)。

### 確かめたこと

- main (f898f57) に rebase 後、`just wasm` 2 つ・`just gen` (差分なし)・`just ci`・`just test-gpu`・`cargo check -p ugoe-editor`。
- ヘッドレスの Chrome と使い捨ての HTTP ブリッジ (scratchpad `T0522/bridge`。T0520 のものの写しに `canvas_per_pixel` を足し、区切りから `subjects` を除いた):
  眼鏡を足した見本のアバター → AI タブで顔の行を開く (edges 48 px · smoothing 2.0 px) → 顔を縁 40 px・ならし 4 px、前髪を縁だけ 12 px に →
  提案の理由が英語で「edge 40 px, smoothing 4.0 px」「edge 12 px」→ 区切りの中が「set_normal_map face」など → Preview → Adopt (25/25) →
  Lighting タブで顔に「Edge width (px) 48」「Smoothing (px) 2.0」。横のはみ出し 0。ブリッジと vite preview は止めた。
  画像 (scratchpad = /tmp/claude-1000/-home-rosylilly-src-github-com-rosylilly-ugoe/32f91561-94f7-4861-9c03-8fe8c9ff7192/scratchpad):
  `T0522/shots/t0522-01-face-shape.png`・`-02-edited.png`・`-03-proposal.png`・`-04-viewport.png`・`-05-lighting-face.png`、マニュアル用の元 `manual-ai-normal-maps.png`・`manual-lighting.png`。

### 確かめていないこと・後続の候補

- 実際の Tauri のウィンドウ。単独のもの (「texture px」) の画面は単体テストだけ (見本はすべてラベルが付く)。
- **ugoe-mcp** (範囲外。team-lead に相談したが返事の前に review にした): `autorig.rs` の `propose_rig` が理由の頭に足す 3 行 (「エージェントが決めた値」「既定の値」「分類: …」)
  が日本語のまま。題名と本文は英語になったので混ざる。エディタにつないだエージェントの提案として同じ一覧に出るので、英語にする小さな後続が要る (MCP のテストは影響なし)。
- 深度推定からの法線マップ (`propose_depth_normal_maps`) は画面がない。

- team-lead が review の後に ugoe-mcp の autorig.rs の理由の頭 3 行を英語にした (承認済み。エディタの提案の一覧で英語の題名・本文と混ざらないように)。
