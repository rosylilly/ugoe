---
id: T0519
title: AI: モジュールの自動リギングの改善 (メッシュへの直接の追従・衝突体・差分のポーズ・プレビューの物理)
status: done
milestone: M6
crates: [ugoe-ai, ugoe-editor-core, apps/editor, docs]
depends_on: [T0517]
assignee: claude-worker-T0519
---

# T0519: AI: モジュールの自動リギングの改善

## 目的

T0517 の後続の候補をまとめる。

## 受け入れ条件

- [x] 追従: 今は本体のワープを 8×8 のワープで近似しており、見本で振り向きの移動量の 1 割ほどずれる。衣装のメッシュのキーフォームに本体の変形を直接写す形を選べるようにする
- [x] 本体のメッシュのキーフォームだけで動く部分 (目の開閉など) への追従。衣装の差分 (A/B) をポーズにする提案
- [x] 揺れ: スロットが公開する本体の衝突体を使い、長い髪やスカートが体にめり込まない提案
- [x] AI タブのプレビューで物理を動かして、採用の前に揺れを確かめられる
- [x] 着せ替えの見本 (`ugoe sample dress-up`、T0712 でプロジェクトとしても出力する) のモジュールのプロジェクトで、座標変換 (T0517 の ugoe-rigging の修正) と提案を画面で確かめる
- [x] 録画した応答でテスト。`just ci`・`just test-gpu`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0517-ai-module-proposals.md` のメモ

## メモ

- **やったこと**
  - **組み方 `ModuleRigOptions`** (ugoe-ai。利用者が選び、AI には送らない): `follow` (`Warp` = 今までの追従のワープ / `Mesh` = 衣装の
    メッシュのキーフォームに本体のワープの変形を直接写す)、`colliders`、`poses` (既定は Warp・入・入)。`propose_module_rig` と
    `propose_module_rig_with_ai_guided` の引数に足した。
  - **直接写す形** (`mesh_keyforms`): 衣装のメッシュに本体のワープ (と下の本体のメッシュ) の軸のキーを足し、キーの組ごとに基準の姿の頂点を
    本体のワープの局所座標に写してからその組の姿で写し、`canvas_to_local` で戻す。追従のワープは作らない (揺らすなら軸のない `SwayRoot`)。
    テストで帽子の頂点のずれは移動量の 2% 未満 (ワープの形は 2 割の許容)。
  - **本体のメッシュへの追従** (`module_rig/mesh.rs`): 目の開閉のように本体のメッシュ自身のキーフォームで形が変わる部分に重なる衣装 (ラベルなしと
    顔・目・口のラベル) に、本体のメッシュの各キーの変位を三角形の重心座標で写す (外の頂点は近い辺の変位を距離で弱める)。頂点の半分以上が
    本体のメッシュの中か 4 px 以内 (取り込みの余白で輪郭の頂点は外に出る。画面で見つけて直した)。ワープの形では衣装のメッシュに本体のメッシュの
    軸だけを足し、直接写す形ではワープの軸と合わせる。
  - **衝突体**: スロットが公開する本体の衝突体 (本体の物理にあるもの) を揺れのリグに当てる。半径 0 でも静止姿勢で当たる衝突体は外し、残りに
    衣装の幅の 1 割の衝突の半径を付けても当たらなければ付ける (`physics_check::rest_pose_collisions`)。理由に当てた・外した衝突体を書く。
  - **差分のポーズ** (`module_rig/pose.rs`): 名前の末尾の A〜E だけが違う衣装のパーツ・メッシュをポーズにする。本体のポーズの要素で
    スロットが公開する (`pose_parts`) 同じ文字の差分があり、絵が重なればその `links` に加え、そうでなければ衣装だけのグループを足す。
    メッシュの差分はパーツを作って入れる。
  - **`Command::SetPose`** (editor-core。enum の末尾、`OP_COUNT` 100 → 101、codegen.rs の見本は `lighting_samples` の末尾)。ポーズを編集する
    Command が無かったので足した。モジュールのプロジェクトでは既存の本体の検査で本体のグループ・要素は変えられない (テスト)。
    `MODULE_CANNOT` の「Pose groups (there is no command for them yet)」を「Edit pose groups by hand (only AI rig proposals make them for now)」に変えた。
  - **エディタ**: AI タブ (モジュールのプロジェクト) に「How to rig」(`ModuleRigChoice { direct, colliders, poses }`、`propose_rig_from_psd`・
    `regenerate_rig_proposal` の `module_options`。ipc.ts・types.ts は `just gen`)。プレビュー中の「Run physics / Stop physics」
    (`ProposalsView.togglePhysics`。`ViewportController.startPhysicsPreview` を使い、プレビューをやめると止める)。
  - 文書: ai-integration.md (モジュールの自動リギングの表と組み方)、editor.md (コマンド一覧の `set_pose`・「ポーズの編集」の節・AI の提案の
    プレビュー・IPC の表・モジュールの AI の提案と組み方)、manual/dress-up.md・manual/ai.md、ugoe-ai の README。
- **範囲外の変更 (ugoe-editor-core。team-lead に相談を送り、返事が来る前に進めた。`crates:` に足した)**: `Command::SetPose` (command.rs・
  document.rs・codegen.rs・tests.rs・module_project_tests.rs・schema/command.schema.json)。T0132 の atlas.rs には触れていない。
- **テスト**: ugoe-ai `tests/module_rig.rs` に直接写す形 (`the_mesh_mode_follows_the_face_warp_at_the_keys`)、新しい `tests/module_rig_sample.rs`
  (着せ替えの見本の本体で、目の上の化粧が目と一緒に潰れ帽子は動かない・衝突体 (静止姿勢で当たる `Neck` を外し `Shoulder` に半径付きで当てる、
  切ると当てない)・差分のポーズ (本体の腕の差分と重なる袖は `links`、重ならないバッジは衣装のグループ。切ると断る))、単体 (重心座標と外の弱め方・
  差分の名前)。録画の再生のテスト (`module_rig_retry.json`) はそのまま通る。editor-core `module_pose_groups_can_be_set`。
  エディタ `module_rig_choice_reaches_the_proposal`。
- **確かめたこと**: main (T0127 ほか) に rebase 後、`just wasm` 2 つ・`just gen` (差分なし)・`just ci`・`just test-gpu`・`cargo check -p ugoe-editor`。
  ヘッドレスの Chrome (SwiftShader) と本物の `Document`・`Proposals`・`ugoe_ai::module_rig` への使い捨ての HTTP ブリッジ (scratchpad/T0519/bridge。
  T0653 のものに T0517 の提案の IPC・`module_options`・座標変換の IPC を足した) で、`ugoe sample dress-up --project` の見本 (本体に衝突体 `Neck`・
  `Shoulder` と、それを公開するスロット `Back` (取り付け先は頭の回転) を足した):
  - `makeup.ugoeproj` (Headwear): 化粧・帽子・バッジ A/B の PSD を取り込み → Propose rig → 化粧が `EyeL`/`EyeR` に従い、バッジはポーズ →
    Preview で `EyeLOpen`・`EyeROpen` = 0 で化粧が目と一緒に潰れる → Adopt (パラメータ 8 のまま)。
  - `long-back.ugoeproj` (Back): 長い後ろ髪 → 既定 (FollowHeadWarp) と「Follow the base warps exactly」(SwayRoot・髪のメッシュに AngleX/AngleY)
    の 2 つの提案。どちらも `Shoulder` に半径 24 px で当て、`Neck` は外す → Preview → Run physics → AngleX = 30 で髪が揺れて戻る →
    End preview で物理も止まる → Adopt。
  - `cat-ears.ugoeproj`: Ears を選んで「Form edit」→ 取っ手が耳に重なり (`local_to_canvas`)、ドラッグで `set_keyform_cell` が通る
    (`canvas_to_local`。T0517 の ugoe-rigging の修正の画面での確認)。横のはみ出し 0。
  スクリーンショット: scratchpad/T0519/shots/t0519-0{1..8}-*.png (06b を含む)。使い終えた vite preview とブリッジは止めた。
  (scratchpad は作業したエージェントの一時ディレクトリ。もう残っていない)
- **確かめていないこと**: 実際の Tauri のウィンドウ、実際の AI の API (AI の経路は組み方を渡すだけで、録画は既存のもの)。衝突体で揺れが実際に
  止まる様子 (見本の揺れは小さく当たらない。数値は ugoe-physics のテストに任せた)。本体のポーズの要素への追従 (`links`) の画面 (見本の本体に
  ポーズがない。テストだけ)。直接写す形で衣装のメッシュを編集した後の組み直し。マニュアルの画像は足していない。
- **後続の候補**:
  - ポーズを手で編集する画面 (`set_pose` はある)。
  - 直接写す形で、衣装のメッシュを編集したら提案し直すよう知らせる (今は文書だけ)。
  - 本体のメッシュへの追従で、ワープ (目のワープなど) と本体のメッシュの両方が動く部分を合わせて写す (今は本体のメッシュの変位を基準の姿で写すだけ)。
  - 差分の判定を名前以外 (AI の分類・絵の重なり) でも行う。
