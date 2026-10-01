---
id: T0660
title: エディタ: プレビューでのポーズの切り替え・名前で見つけた差分の追従・物理の無い本体の衝突体の案内・見本の肩の衝突体
status: done
milestone: M4
crates: [ugoe-ai, ugoe-editor-core, ugoe-mcp, ugoe-cli, assets, apps/editor, docs]
depends_on: [T0523, T0655]
assignee: claude-worker-T0660
---

# T0660: ポーズと見本の残り

## 目的

T0523・T0655 の後続の候補をまとめる。

## 受け入れ条件

- [x] プレビューモードのパネルで、ポーズの表示する要素を切り替えられる
- [x] 名前で見つけた衣装の差分も、見えている / 隠してあるに応じて本体の差分に追従させる
- [x] ugoe-mcp の set_layer_labels で variant_sets を受ける
- [x] エディタで、物理の無い本体が公開する衝突体をモジュールのリグが参照しているときの案内 (今はエラーになる)
- [x] 見本の本体に肩の衝突体を入れ直し、スロット Hair で公開する (見本の作り直し)。T0655 の規則で、物理を渡さずに作った Puppet でもモジュールを付けられることを確かめる
- [x] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0523-ai-pose-and-rig-notices.md`・`docs/tasks/T0655-sample-projects-and-slot-exposure.md` のメモ

## メモ

- **やったこと**
  - **プレビューのパネルのポーズ** (`preview/PreviewPanel.svelte`・`preview.svelte.ts`): 編集中のモデルにポーズのグループがあれば「Runtime」の下に
    「Pose」の節を出し、グループごとの選択で表示する要素を `show_pose` で切り替える (`PreviewMode.showPose`・`poseShown`。フェードはポーズの時間で、
    毎フレームの `update` が進める)。入るときは編集中のビューポートで見ていた要素 (`ViewportController.posePreview`) から始め、モデルを読み直すたびに
    渡し直す。ポーズの段を止めている間は選べない (理由を出す)。モジュールのプロジェクトで今の編集を本体に付けて比べている間は、今の編集が
    `<名前> (editing)` で付くので、モジュールのパーツの接頭辞を付け替える (`preview.ts` の `previewPart`)。T0659 と重ならないよう、着せ替えの節には触れていない。
  - **名前で見つけた差分の追従** (`module_rig/pose.rs`): 名前で見つけた組が本体の要素の文字と合わなければ (`linked_group` が無ければ)、PSD で
    見えている差分がちょうど 1 つのとき (`shown_first`・`part_hidden`)、見えているものを先頭に並べ替えて `linked_by_drawing` で対応付ける。理由の文は
    「the base names have no matching letters, so the variant shown in the PSD follows the base entry shown first …」。
  - **MCP の `set_layer_labels` の `variant_sets`** (`ugoe-mcp/src/autorig.rs`): `layers` と `variant_sets` のどちらかを省ける (どちらも無ければエラー)。
    `variant_sets` は `apply_answer` にそのまま渡し、PSD 全体の組を置き換える (`[]` で消す。`reset` でも消える)。結果に `variant_sets` (レイヤーの番号の組)、
    `get_psd_layers` の各レイヤーに `variant`。ツールの説明 (エージェントに見せる文) と入力の形 (`output_schema` の `variant_sets` に説明を付けたもの) を足した。
    MCP にはモジュールの自動リギングのツールが無いので、組は MCP のセッションの分類に残るだけ (エディタの AI タブの分類には入らない。文書に書いた)。
  - **物理の無い本体が公開する衝突体の案内** (editor-core・エディタ): `ModuleContext::compose_physics` (作業用のモデルと書き出しの検査で共用) が、本体に物理が
    無く、スロットが公開する衝突体への参照で `UnknownCollider` になったとき `ModuleError::ColliderNeedsBasePhysics { rig, collider, slot }` (本体に物理を足すよう
    促す文) にする。ゆるめてはいない (T0655 の判断どおり、保存で参照が黙って消えないように)。`ModuleContext::unavailable_colliders` (スロットが公開するが本体に無い衝突体)。
    エディタ: `ModuleProblemKind::BaseNeedsPhysics` (`base_needs_physics`。説明と「Open base project」)、`ModuleStatus.unavailable_colliders` と物理の画面の
    「Colliders」の案内 (`unavailableCollidersNotice`)。IPC は型だけ (`just gen` の types.ts。コマンドは変えていないので ipc.ts はそのまま)。Command は足していない (`OP_COUNT` は 101 のまま)。
  - **見本の肩の衝突体** (`ugoe-cli/src/sample/dress_up.rs`): 本体の物理に `Shoulders` (体のワープ `BodyWarp` に付けた、胴の上の辺に沿う半径 34px のカプセル) を足し、
    スロット `Hair` の `colliders` で公開する。見本の本体の後ろ髪とモジュールのリグは使わない (モジュールの `.ugoemod` を変えないため)。
  - **見つけて直したもの (範囲内):** モジュールの問題の枠の長い説明が枠からはみ出していた (T0658 の `header > :not(.file-menu)` の `white-space: nowrap` が枠にも効く)。
    `ModuleProblemBar.svelte` で `:global(header) > div.problem` に `white-space: normal`。
  - 文書: editor.md (プレビューのポーズ・物理の画面の案内・`module_status`・`base_needs_physics` と物理の無い本体の衝突体)、ai-integration.md (分類の手がかりの MCP・
    名前の組の追従・MCP の表)、spec/modules.md (作者のツールの扱い・見本)、manual/preview.md・dress-up.md・mcp.md、assets/samples/README.md、ugoe-ai と ugoe-mcp の README。
    英語の公開仕様は変えていない (ランタイムの規則は変わらない)。
- **見本の作り直し:** `assets/samples/dress-up/dress-up.ugoe` だけが変わった (74,494 → 77,054 バイト)。**4 つの `.ugoemod` はバイト列で同じ。** 絵は変わらないので
  `dress-up.jpg` はそのまま。`committed_sample_is_up_to_date`・`projects_from_committed_files_match_the_generator` が通る。
  - **本体の指紋が変わる:** ビューアの「サンプル」→「着せ替え」で覚えた組は場所 `sample:dress-up/dress-up.ugoe` の鍵で今までどおり戻る。前の見本をドロップや「開く」で
    開いて覚えた組 (場所が `path:` か無いもの) は、新しい見本では戻らない (T0715 と同じ)。
  - T0655 の規則の確認: `hair_slot_exposes_the_shoulders` (ugoe-cli) が、物理を渡さずにモデルだけで作った Puppet に、肩に当たるよう書き換えた `LongHair` と元の
    `LongHair` を付けられ (リグの衝突体の参照は外れる)、物理を渡した Puppet では合成した物理に `Shoulders` があることを確かめる。`hair_modules_sway_after_a_head_shake`
    (モデルだけの Puppet) も通る。
- **テスト:** ugoe-ai `named_variants_follow_the_base_shown_and_hidden_entries` (本体の腕の名前を「Arm Up」「Arm Down」にし、隠した Sleeve A・見えている Sleeve B →
  Sleeve B が ArmA・Sleeve A が ArmB。どちらも見えていれば衣装だけのグループ)。ugoe-mcp `agent_driven_auto_rig_from_psd` に `variant_sets` (形が ugoe-ai と同じ・
  どちらも無いとエラー・2 つの組に入る番号のエラー・組だけを渡すと分類は変わらない・1 枚の組を捨てる・一覧の `variant`・`[]` で消える)。editor-core
  `exposed_colliders_need_the_base_physics`。エディタ `exposed_colliders_without_base_physics_are_explained` (見本を書き出し、本体の物理を外すと
  `base_needs_physics` と `unavailable_colliders`)。Vitest (`unavailableCollidersNotice`・`base_needs_physics` の説明と直し方・`previewPart`)。
- **確かめたこと:** main (T0721 の 0a602c7・4565060) に rebase 後、`just wasm` 2 つ・`just gen` (差分なし)・`just ci`・`cargo check -p ugoe-editor`・`just test-gpu`。
  `just conformance --only c,wasm` は rebase の前 (c・wasm とも 53/53)。
  ヘッドレスの Chrome (SwiftShader、WebGL2) と使い捨ての HTTP ブリッジ (scratchpad/T0660/bridge。T0655 のものに `unavailable_colliders`・`base_needs_physics`・
  見本の準備 `t0660-prepare` を足した。準備は見本を `save_bundle_projects` で書き、本体に髪と服のポーズのグループを足し、物理あり / なしの 2 組を作る) で:
  本体のプレビューで「Pose」→ Group 1 を Clothes にすると髪が消えて服が出る → Pose の段を切ると選択が無効になり理由 → 出て入り直すと Hair から。
  `twin-tails.ugoeproj` (物理あり) は物理の画面に案内なし、プレビュー (今の編集を付けて比べる) で本体のグループを切り替えられる。物理なしの `twin-tails.ugoeproj` は
  「Colliders」に案内。肩に当たる `long-hair.ugoeproj` を物理なしの本体で開くと `base_needs_physics` の枠 (「Open base project」)。横のはみ出し 0。
  スクリーンショット: /tmp/claude-1000/-home-rosylilly-src-github-com-rosylilly-ugoe/32f91561-94f7-4861-9c03-8fe8c9ff7192/scratchpad/T0660/shots/t0660-0{1,2,3,4,5,5b,6}-*.png。
  使ったブリッジと vite preview は止めた。
- **確かめていないこと:** 実際の Tauri のウィンドウ。モジュールのプロジェクトで、モジュール自身のポーズのグループをプレビューで切り替えること (接頭辞の付け替えは
  Vitest だけ。見本のモジュールにはパーツが 1 つしかなく、グループを作れなかった)。実際の AI・MCP クライアントが `variant_sets` を渡すこと (stdio のテストだけ)。
  プレビューの画面で見えていた選択のギズモ (回転デフォーマの円と線) は前からのもので、この作業では触っていない。
- **範囲外の変更:** なし (ugoe-ai・ugoe-editor-core・ugoe-mcp・ugoe-cli・assets・apps/editor・docs)。
- **後続の候補:**
  - プレビューのポーズに、着せ替えで付けたモジュールのグループも出す (WebPuppet にポーズのグループを返す口 (`pose_json` など) が要る。ugoe-wasm)。
  - MCP にモジュールの自動リギングのツールを足し、`variant_sets` をポーズに使う (今は分類に残るだけ)。
  - 見本のモジュール (`LongHair`) の揺れを肩の衝突体に当てる (モジュールの `.ugoemod` と指紋が変わる)。
  - 物理の無い本体で開けないモジュールを、参照を外して開く選択肢 (今は本体に物理を戻すか本体を選び直す)。
