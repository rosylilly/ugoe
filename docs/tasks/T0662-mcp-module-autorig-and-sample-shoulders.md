---
id: T0662
title: MCP にモジュールの自動リギングのツール・見本のツインテールを肩の衝突体に当てる
status: done
milestone: M4
crates: [ugoe-ai, ugoe-mcp, ugoe-cli, assets, apps/editor, docs]
depends_on: [T0660]
assignee: claude-worker-T0662
---

# T0662: モジュールの自動リギング (MCP) と見本の肩

## 目的

T0660 の後続の候補のうち、ツール層と見本に閉じるものをまとめる。

## 受け入れ条件

- [x] ugoe-mcp にモジュールの自動リギングのツールを足し、`set_layer_labels` で受けた `variant_sets` をポーズに使う (今は分類に残るだけ)
- [x] 見本のモジュール `TwinTails` の房の揺れ (`TailL`・`TailR` のリグ) を、本体がスロット Hair で公開する肩の衝突体 `Shoulders` に当てる (`.ugoemod` と指紋が変わる。ビューアの覚えた組への影響をメモに書く)。`LongHair` は変えない (後ろ髪の振り子が静止姿勢で肩を横切るため。team-lead の承認で `LongHair` から読み替えた)
- [x] ツールの説明・ugoe-mcp の README・`docs/manual/mcp.md` を揃える
- [x] `just ci`・`just conformance --only c,wasm` が通る

## 参考

- `docs/tasks/T0660-editor-pose-and-sample-leftovers.md` のメモ

## メモ

- **やったこと**
  - **MCP のモジュールの自動リギング** (`ugoe-mcp/src/autorig.rs`・`tools.rs`): `get_module_rig_guide` (値の説明と範囲・衣装のメッシュの位置と従う本体の
    ワープ (`module_rig_request_guided` の `system` と `prompt`)・`module_rig_output_schema`・既定値・組み方の既定・スロット・差分の組・衣装の原画) と
    `propose_module_rig` (`params` は `module_rig_output_schema` の形、組み方 `follow`・`colliders`・`poses` は省くと既定。`module_mesh_parts` を通すので
    `set_layer_labels` の `variant_sets` が `MeshPart::variant` に入り、ポーズの AI の手がかりになる)。提案は既存の `add_proposal` に置き、適用は
    `apply_proposal` (確認) のまま。`import_psd` に `base` と `slot` (一緒に渡す) を足し、`Project::new_module` に `map_new_meshes` で取り込む
    (エディタの PSD の取り込みと同じ)。`base` も `--root` で解決する。結果に `module`、`meshes` は衣装のメッシュだけを数える。
    モジュールのプロジェクトで `get_rig_guide`・`propose_rig` を呼ぶとモジュールのツールへ案内するエラー (パラメータを足すため)、逆も同じ。
    `set_layer_labels` の `rig.ready` はモジュールではモジュールのリグを組めるか、`rig.tool` に呼ぶツール。サーバの instructions・`apply_proposal` の説明も直した。
  - **見本の肩 (読み替え。team-lead が承認):** `LongHair` の後ろ髪の振り子 (頭の中心から真下に 250px) は静止姿勢の区間が `Shoulders` を横切る
    (`rest_pose_collisions` が `LongHair/Sway` の粒子 1 を返す)。当てると physics.md の「始めの向きで当たっているとき」で後ろ髪が横へ押しのけられ、
    ugoe-ai の module_rig も静止姿勢で当たる衝突体は外す規則なので、`LongHair` は変えず、肩のすぐ外に垂れる `TwinTails` の `TailL`・`TailR` のリグを
    `Shoulders` に当てた (`ugoe-cli/src/sample/dress_up.rs`)。理由は dress_up.rs のコメント・assets/samples/README.md・docs/manual/dress-up.md・spec/modules.md に書いた。
  - 文書: ugoe-mcp の README (手順の節「着せ替えのモジュールの自動リギング」・ツールの表・許可の一覧)、docs/manual/mcp.md (エディタにつなぐ・ファイルを直接扱う)、
    docs/ai-integration.md (分類の手がかりの MCP・MCP の表・モジュールの注意)、ugoe-ai の README。英語の公開仕様は変えていない (ランタイムの規則は変わらない)。
- **見本の作り直し:** 変わったのは `assets/samples/dress-up/twin-tails.ugoemod` だけ (38,104 → 38,120 バイト)。`dress-up.ugoe`・`long-hair.ugoemod`・
  `cat-ears.ugoemod`・`sailor.ugoemod` はバイト列で同じ (`git diff main --stat -- assets/` で確かめた)。静止姿勢では房は肩に当たらないので `dress-up.jpg` はそのまま。
  - **指紋の影響:** 本体の指紋は変わらないので、ビューアがモデルごとに覚えた組 (鍵は本体) はそのまま見つかる。`TwinTails` は名前 → このセッションの中身 (指紋) →
    場所の順で戻すので、「サンプル」→「着せ替え」で覚えた組は場所 `sample:dress-up/twin-tails.ugoemod` から新しい中身で戻る。ドロップや「開く」で読んだ前の
    `twin-tails.ugoemod` (場所が無いもの) は、前と同じく次のセッションでは「場所が分からない」になる (覚えた指紋が古いだけで、扱いは変わらない)。
  - エディタの「Open Sample…」の `twin-tails.ugoeproj` は、物理を外した本体で開くと `base_needs_physics` の枠が出るようになる (前は「Colliders」の案内だけ)。
  - 互換性テストの資料 (`assets/compat/`) は見本を使わないので作り直していない (`cargo test -p ugoe-format --test compat` は just ci で通る)。
- **テスト:** ugoe-mcp `agent_driven_module_rig_from_psd` (`character.psd` を見本の本体のスロット `Headwear` に取り込む・`base` だけはエラー・
  本体向けのツールの案内・左右の目を `variant_sets` の組にすると `poses: false` では使わず既定ではポーズになる (理由の文と保存した `module.json` の
  `pose.groups`)・範囲の外の値・知らない `follow`・パラメータが増えない・モジュールでないプロジェクトの案内)。ugoe-cli `twin_tails_hit_the_shoulders`
  (房は静止姿勢で当たらず、`LongHair` を当てると当たる。頭 ±30・体 ±10 を 2 秒振る間、`TailR` の振り子の区間と肩の芯の距離の最小が、当てると 34.0
  (半径 34。縁で止まる)、当てないと 25.8 (入り込む))。`hair_slot_exposes_the_shoulders` は `TwinTails` がそのまま当たる形に直した。
- **範囲外の変更 (team-lead が承認):** `apps/editor/src-tauri/src/sample.rs` の `exposed_colliders_without_base_physics_are_explained` だけ。`TwinTails`
  を「当たらないモジュール」に使っていたので、当たるモジュールを `TwinTails` (そのまま)、当たらないモジュールを `LongHair` に入れ替えた (テストの意図は同じ:
  当たるものは `base_needs_physics`、当たらないものは案内なしで `unavailable_colliders`)。apps/viewer に `TwinTails` を当たらない前提にしたテストは無かった。
  crates: に apps/editor を足した。
- **確かめたこと:** `just ci` (main は ae681e5 のまま。`just wasm` 2 つを先に実行)、`just conformance --only c,wasm` (c・wasm とも 53/53。見本を作り直した後)、
  `cargo test -p ugoe-editor sample::`。
- **確かめていないこと:** 実際の MCP クライアント (Claude Code など) がモジュールのツールを使うこと (stdio のテストだけ)。エディタに組み込んだ MCP
  (`--editor`) でモジュールのプロジェクトを開いて `propose_module_rig` の提案が「AI」タブに出ること (同じツールの経路だが画面では見ていない)。
  `just test-gpu` (描画は触っていない)。ビューアの画面で房が肩に当たる見た目。
- **後続の候補:**
  - `--root` のとき、`open_project` で開くモジュールのプロジェクトの `project.json` の本体の参照が root の外を指しても読んでしまう (前からのこと。
    `import_psd` の `base` は root で解決する)。本体のパスも root で確かめる。
  - MCP の `propose_module_rig` に利用者の注文と作り直し (`ModuleRigGuidance`) を渡す口 (今はエージェントとの会話で代える)。
  - `LongHair` を肩に当てるには、後ろ髪を左右に分けて肩の外に根元を置くなど、絵とリグの作り直しが要る。

