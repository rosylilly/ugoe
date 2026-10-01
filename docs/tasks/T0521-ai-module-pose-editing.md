---
id: T0521
title: エディタと AI: ポーズを手で編集する画面・直接写す形の作り直しの知らせ・差分の判定
status: done
milestone: M6
crates: [ugoe-ai, ugoe-editor-core, apps/editor, docs]
depends_on: [T0519]
assignee: claude-worker-T0521
---

# T0521: ポーズの手での編集と、モジュールの自動リギングの残り

## 目的

T0519 の後続の候補をまとめる。

## 受け入れ条件

- [x] ポーズのグループを手で編集する画面 (`Command::SetPose` を使う。モジュールのプロジェクトでは本体のグループは読み取り専用)
- [x] 本体の変形を衣装のメッシュに直接写す形で組んだ後に衣装のメッシュを編集したら、提案し直すよう知らせる
- [x] 本体のワープと本体のメッシュの動きを合わせて写す (今はどちらか一方)
- [x] 衣装の差分 (A/B) を、名前の末尾以外 (絵の重なり・意味ラベル) でも判定する
- [x] 録画した応答でテスト。`just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0519-ai-module-rig-followups.md` のメモ

## メモ

- **やったこと**
  - **ポーズの画面** (エディタ。右の列の新しい「Poses」タブ、`lib/panels/poses/PosesPanel.svelte`・`poses.ts`): グループの一覧、要素の
    ▲▼・✕、「first」の印、要素ごとの「Links」(スロットの `PickList` を流用)、「+ Entry」「+ Group」「+ Group from selection」(ツリーで選んだ
    パーツ)、「Fade time (s)」。操作はすべて `set_pose` (Command は足していない。`OP_COUNT` は 101 のまま)。候補はポーズでまだ使っていない
    パーツ。モジュールのプロジェクトでは、本体のグループ (先頭の `ModuleStatus.base_pose_groups` 個) を点線と「base」の印で示し、並べ替え・
    外す・足す・消すとフェードの時間を無効にし、上に理由を出す。本体の要素のうちスロットが公開するもの (`SlotSummary.pose_parts`) にだけ
    「Links」を出してモジュールのパーツを選べる (本体のリンクは「Base links」として見せ、送るときに残す)。公開していない要素には理由を出す。
    `MODULE_CAN`/`MODULE_CANNOT` を直した。
  - **作り直しの知らせ** (Tauri `proposals.rs`): モジュールのリグの提案を置くときに、メッシュのキーフォームを書くコマンド (`add_keyform_key`・
    `set_keyform_cell`) の番号と対象を覚え、採用 (`adopt_in` に切り出した) で外したコマンドを除いたメッシュの形 (親・UV・三角形) を覚える。
    形が変わり、今キーフォームの軸を持つものを `ProposalState.stale_rigs` に出す。AI タブの上に「Propose the rig again」とメッシュの名前・
    「Dismiss」(`dismiss_stale_rigs`: 今の形を覚え直す)、タブに「•」と説明。直接写す形だけでなく、本体のメッシュへの追従 (ワープの形でも
    頂点ごとに写す) も対象にした。覚えるのはセッションの間だけ。
  - **ワープとメッシュの動きを合わせて写す** (`module_rig/mesh.rs`): 本体のメッシュの変位を、衣装が従う本体のワープ (なければ取り付け先) の
    局所座標 (`Frame`) で測る。キーの組ごとに、その組の姿のフレームで本体のメッシュの頂点を局所座標に戻して基準の姿と比べる (フレームの動きを
    二重に数えない)。軸は本体のメッシュ自身と、フレームより下の祖先のデフォーマ (目の回転など) の軸を合わせたもの (前はメッシュ自身の軸だけで、
    下のデフォーマの動きは写らなかった)。衣装の頂点は局所座標で変位を足してから同じ姿のフレームで写す (`mesh_keyforms`。直接写す形と
    ワープの形で同じ)。
  - **差分の判定** (`module_rig/pose.rs`): 名前で見つからなかった衣装のうち、同じ親のパーツの中の同じ種類で、意味ラベルと左右が同じで、外接矩形の
    IoU が 0.6 以上、PSD で 1 つを見せて残りを隠してある組を差分にする (見えているものが先頭)。どれも見えている重なった絵 (影・線画) は
    差分にしない。この組は本体の要素に追従させない。隠してあったメッシュは、メッシュの不透明度を 1 にし、作るパーツの不透明度を 0 にする
    (基準の姿の見た目は変わらず、ポーズで出せる。名前の差分のメッシュにも同じ扱いを入れた)。
  - IPC: `dismiss_stale_rigs`、`ProposalState.stale_rigs`・`StaleRig`、`SlotSummary.pose_parts`、`ModuleStatus.base_pose_groups` (ipc.ts・
    `just gen` の types.ts)。
  - 文書: editor.md (モジュールの AI の提案の組み方・作り直しの知らせ・「ポーズの画面」の節・使えるもの・IPC の表)、ai-integration.md
    (本体のメッシュへの追従・ポーズ・`follow` の表)、manual/dress-up.md (「ポーズを手で編集する」・知らせ・差分)、manual/ai.md、ugoe-ai の README。
- **テスト:** ugoe-ai `tests/module_rig_sample.rs` に、目を視線で傾く回転デフォーマに入れた見本で、取り付け先を頭の回転にしたとき化粧が
  顔のワープ・目の回転・目の開閉を重ねた目と同じ所へ動く (直接写す形とワープの形。ずれは移動量の 8% 未満。前の実装では `EyeBallX` を
  写さない) と、隠したキャップと見えている帽子がポーズになり、見えている眼鏡とレンズ・ラベルの違う隠したネクタイはならない。
  pose.rs の IoU の単体。録画の再生のテスト (`module_rig_retry.json`) はそのまま通る (AI の経路は変えていない)。Tauri
  `editing_a_rigged_costume_mesh_asks_to_propose_again` (カリングでは知らせず、頂点を消すと知らせ、Dismiss で消え、採用を Undo したメッシュは
  知らせない)、module.rs の `base_pose_groups`。Vitest `poses.test.ts` (使っているパーツ・候補・足す/動かす/外す・本体のリンクを残す・
  本体のグループと公開した要素・フェードの時間)。
- **確かめたこと:** main は着手時から動いていない (rebase なし)。`just gen` (差分なし)・`just ci`・`just test-gpu`・`cargo check -p ugoe-editor`。
  ヘッドレスの Chrome と本物の `Document`・`Proposals`・`ugoe_ai::module_rig` への使い捨ての HTTP ブリッジ (scratchpad/T0521/bridge。T0519 の
  ものに `stale_rigs`・`dismiss_stale_rigs`・`pose_parts`・`base_pose_groups` と見本の準備 `t0521-prepare` を足した) で、`ugoe sample dress-up
  --project` の見本の本体に腕の差分 A / B とそのポーズ、腕 A だけを公開するスロット `Sleeves` を足して:
  - 本体 (`dress-up.ugoeproj`): Poses タブ → 「+ Group」(Hair)・「+ Entry」(Clothes)・▲ で Clothes を先頭・腕 A の Links に Head・
    フェード 0.3 → `set_pose` のポーズが画面どおり。
  - `sleeves.ugoeproj`: 本体のグループが点線・「base」、ボタン 7 つとフェードが無効、腕 A だけ Links、腕 B は理由 → 腕 A に衣装のパーツを
    リンク (`ArmA.links = [sleeves]`)。
  - `hats.ugoeproj` (Headwear): 見えている帽子・隠したキャップ・目の上の化粧の PSD → 「Follow the base warps exactly」で Propose rig →
    理由に帽子とキャップのポーズと化粧の目への追従 → Adopt (キャップのパーツの不透明度 0) → 化粧の頂点を 1 つ消す (ブリッジ経由の編集) →
    AI タブに「Propose the rig again · makeup」、タブに「AI •」→ Dismiss で消える → Poses タブに本体のグループと衣装のグループ。
  横のはみ出し 0。スクリーンショット: scratchpad/T0521/shots/t0521-0{1..7}-*.png。使い終えた vite preview とブリッジは止めた。
  (scratchpad = /tmp/claude-1000/-home-rosylilly-src-github-com-rosylilly-ugoe/32f91561-94f7-4861-9c03-8fe8c9ff7192/scratchpad)
- **確かめていないこと:** 実際の Tauri のウィンドウ。ビューポートの描画 (この worktree で wasm を作っていないので、画面の確認はパネルだけ。
  合わせて写した化粧の動きは数値のテスト)。メッシュの編集の画面 (Edit mesh) からの知らせ (頂点の削除はブリッジで送った。ドキュメントの
  変化で `get_proposals` を読み直す経路は同じ)。実際の AI の API (AI の経路は変えていない)。本体に見えている差分と隠した差分がある PSD での、
  本体の要素への追従 (絵の重なりで見つけた組は追従させない)。
- **範囲外の変更:** なし (ugoe-ai・apps/editor・docs。editor-core は触っていない)。
- **後続の候補:**
  - 作り直しの知らせをプロジェクトに残す (今はセッションの間だけ。開き直すと消える)。インスペクタで選んだメッシュにも知らせを出す。
  - 絵の重なりで見つけた差分を、本体の差分 (見えている / 隠してある) と対応付けて本体の要素に追従させる。
  - AI の分類に「差分」の手がかり (同じ部品の別の絵) を足し、名前・重なり以外でも判定する。
  - ポーズの画面で、要素をドラッグで並べ替える・プレビューで表示中の要素を切り替えて確かめる。
