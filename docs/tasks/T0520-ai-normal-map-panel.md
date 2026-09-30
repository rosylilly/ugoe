---
id: T0520
title: AI: すべてのテクスチャの法線マップを AI タブから提案する・眼鏡などの扱い
status: done
milestone: M6
crates: [ugoe-ai, ugoe-rigging, apps/editor, docs]
depends_on: [T0518]
assignee: claude-worker-T0520
---

# T0520: AI: 法線マップの提案の画面

## 目的

T0518 の後続の候補をまとめる。

## 受け入れ条件

- [x] AI タブから、すべてのテクスチャの法線マップをまとまりごとに提案し、プレビューして採用できる
- [x] 眼鏡など、ふくらませず向きだけを受けるまとまりを足す (意味ラベルの扱い)
- [x] マニュアル (lighting.md・ai.md) の画像を撮り直す
- [x] 録画した応答でテスト。`just ci`・`just test-gpu`・`cargo check -p ugoe-editor` が通る (法線マップの提案は AI を使わないので、録画の再生は要らない)

## 参考

- `docs/tasks/T0518-ai-normal-map-groups.md` のメモ

## メモ

### 範囲の追加

- `ugoe-rigging` (team-lead に相談を送り、返事が来る前に進めた): 眼鏡の「向きだけを受ける」には、まとまりのふくらみの格子と形の mask が要り、
  それは `normal/group.rs` の中でしか素直に書けない。**追加だけ** (`group_bulge_normal_maps_oriented`・`GroupNormalMaps`)。
  既存の `group_bulge_normal_maps` の署名と結果は変えていない (中身を `GroupField` に分けて共有しただけ)。`lit.rs` (T0716) と見本は変わらない
  (`just ci` の見本の最新のテストが通る)。

### やったこと

- **ugoe-rigging** (`normal/group.rs`): `group_bulge_normal_maps_oriented(members, shape_only, oriented, options)`。`oriented` は形に加えず、
  画素の位置がまとまりの形の中ならその高さを引き、形の外の画素は **近い形の中の画素の向き** を受ける (`extend_edges` を 64 画素)。
  最初は形の外に高さを広げていた (`fill_missing`) が、輪郭の高さの行ごとの違いが持ち出されて、つるが上下に 34° 傾いたので向きを広げる形に変えた。
  輪郭をまたぐ隣の画素の角度は 12° 以内 (テスト)。
- **ugoe-ai** (`normal.rs`): `SurfaceGroup::of(Eyewear)` を `Alone` から `Face` に変え、`SurfaceGroup::orientation_only(label)` (眼鏡だけ) を足した。
  `NormalMapGroup` に `oriented` (と、`textures` に向きだけを受けるものも含める)。`normal_map_groups` (すべての対象のまとまりの一覧。単独のもの
  (`alone`・ラベルなし) はそれぞれ 1 行にまとめて最後)。提案の理由に「形に加えず向きだけを受けたもの」。
- **エディタ**:
  - IPC `get_normal_map_groups` (`Vec<NormalMapGroupView>`。`oriented`・`with_normal_map` を足した) と `propose_normal_maps(choices, replace_existing)`
    (`NormalMapGroupChoice { group, settings }`。AI なし・何も送らない。モジュールのプロジェクトでは断る)。提案のコマンドをまとまりごとの区切り
    (`CommandSection { label, indices, subjects }`) にして `ProposalState::sections` で渡す。ipc.ts・types.ts (`just gen`)。
  - AI タブ (`ProposalsPanel.svelte`) に「Normal maps from the shapes」(モデルのプロジェクトだけ): まとまりごとに含めるか・強さ・テクスチャの数・
    法線マップを持つ数・向きだけの数、「Replace existing normal maps」、「Propose normal maps (n)」。提案のコマンドの一覧は区切りの単位
    (1 つだけの区切りも名前を出し、中はテクスチャの名前付き)。分類し直すとまとまりを読み直す。
  - Lighting タブ (`NormalMapSection.svelte`): 眼鏡を選ぶと「Takes the direction of group “face” (…) without bulging」、顔を選ぶと「…; eyewear only take its direction」。
    AI タブへの案内。「Lit meshes」の一覧は触っていない。
- **docs**: ai-integration.md (表の `eyewear` と「向きだけを受ける」、`normal_map_groups`)、editor.md (「法線マップの提案」の節、IPC の表、`get_proposals` の `sections`)、
  manual/ai.md (「法線マップを提案する」と画像 `ai-normal-maps.png`)、manual/lighting.md (眼鏡と AI タブへの案内、`lighting.png` を撮り直し)、ugoe-ai・ugoe-rigging の README。
- **テスト**: rigging `tests/normal.rs` 2 件 (眼鏡の帯: 顔の形は変わらず、顔の上は顔の向き、つるは外向きに折れずに続く・形がなければ平ら)。
  ugoe-ai `tests/normal.rs` 2 件 (眼鏡のラベルは顔のまとまりで向きだけ・一覧はすべての対象を 1 回ずつ、設定は 1 枚ずつ尋ねたときと同じ)。
  エディタ `proposals_tests` 1 件 (まとまりごとの設定・区切り・顔のまとまりを外して採用・Undo・選ばない/知らないまとまりを断る)、`lighting::tests` 1 件 (分類で眼鏡)、
  Vitest 3 件 (選び方・送るもの・区切りの名前)。
- Command の enum は足していない (OP_COUNT は 101 のまま)。

### 確かめたこと

- main に rebase 後、`just wasm` 2 つ・`just gen` (差分なし)・`just ci`・`just test-gpu`・`cargo check -p ugoe-editor`。
- ヘッドレスの Chrome (SwiftShader、`vite preview`) と本物の `Document`・`Proposals`・`ugoe_ai::normal` への使い捨ての HTTP ブリッジ (scratchpad `T0520/bridge`。
  T0518 のものに提案と法線マップの提案を足した。区切りの作り方はエディタと同じ手順を写した): 見本のアバターのプロジェクトに眼鏡のレイヤー (名前 `eyewear`、つるが顔の外に出る) を
  取り込んだもの → 光源を左から → AI タブにまとまり 6 行 (顔は「15 textures, 1 direction only」) → 顔を 0.3・耳飾りを外して「Propose normal maps (23)」→
  区切り 6 つ (顔 15・体 4・前髪 1・後ろ髪 1・耳 2・ライティング 1) → Preview で陰影が付く → 前髪の区切りを外すとプレビューが作り直される (23 of 24) → 戻して Adopt
  (23/25 のテクスチャに法線マップ・ライティング有効) → Lighting タブで眼鏡の表示 → Undo で 0/25・提案が再び出る。横のはみ出し 0。ブリッジと vite preview は止めた。
  画像 (scratchpad = /tmp/claude-1000/-home-rosylilly-src-github-com-rosylilly-ugoe/32f91561-94f7-4861-9c03-8fe8c9ff7192/scratchpad): `T0520/shots/t0520-01-groups.png`・`-02-preview.png`・
  `-03-viewport-left.png`・`-04-viewport-no-front-hair.png`・`-05-lighting-eyewear.png`・`-06-lighting-face.png`、マニュアル用の元 `manual-ai-normal-maps.png`・`manual-lighting.png`。

### 確かめていないこと・後続の候補

- 実際の Tauri のウィンドウ。PSD の分類 (AI) のラベルでの一覧は単体テストだけ (画面は名前の規則)。本物の眼鏡の絵での見た目 (確かめたのは描いた輪と帯だけ)。
- AI タブで選べるのは強さだけ (縁の丸み・ならしはまとまりの既定)。まとまりごとに縁も選べるようにするか。
- `Edge width (px)` はまとまりの格子 (最も細かいテクスチャの画素) の単位で、見本のアバターの顔は 148 (キャンバスでは約 67 px。虹彩のテクスチャが細かいため)。
  画面の「px」がキャンバスの px に見えて紛らわしい (T0518 からの挙動。変えていない)。
- 提案の題名・理由が日本語で英語の画面に混ざる (T0637 からの課題)。
- `proposal::analyze` の `subject` が `set_normal_map` の `texture` を拾わない (editor-core。範囲外なので区切りの `subjects` で補った)。拾えば MCP のエージェントの提案でも名前が出る。
- 眼鏡のほかに向きだけを受けたいもの (顔の化粧・頬の線・耳の上の髪飾りなど) があれば `orientation_only` に足す。
