---
id: T0523
title: エディタと AI: 作り直しの知らせをプロジェクトに残す・差分のポーズの追従・ポーズの画面の操作
status: done
milestone: M6
crates: [ugoe-ai, ugoe-editor-core, apps/editor, docs]
depends_on: [T0521]
assignee: claude-worker-T0523
---

# T0523: ポーズと自動リギングの知らせの残り

## 目的

T0521 の後続の候補をまとめる。

## 受け入れ条件

- [x] 「リグを提案し直す」の知らせをプロジェクトに残し (今はセッションの間だけ)、インスペクタにも出す
- [x] 絵の重なりで見つけた衣装の差分を、本体の差分に追従させる
- [x] AI の分類に差分の手がかりを足す
- [x] ポーズの画面に、ドラッグでの並べ替えと、表示中の要素の切り替えを足す
- [x] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0521-ai-module-pose-editing.md` のメモ

## メモ

- **やったこと**
  - **知らせをプロジェクトに残す** (Tauri `proposals.rs`・`project.rs`): 採用したときの衣装のメッシュの形 (親のデフォーマの ID・UV・三角形) を
    FNV-1a 64 ビットの指紋 (16 桁の 16 進) にし、`project.json` の `editor.rigged_meshes` (`{"<メッシュの ID>": "<指紋>"}`) に置く。ID は
    ファイルの中の ID (`ModuleContext::display_id`。取り込んだばかりのメッシュは接頭辞がなく、開き直すと `<名前>/` が付くため)。採用と
    Dismiss で書き換えて未保存の印を付け、`ProjectSession::from_project` で読む。項目のない前からのプロジェクト・形の違う値は何も覚えない。
    `editor` の自由な JSON なので `PROJECT_FORMAT_VERSION` は変えていない (file-formats.md の `project.json` の表に行を足した)。
    インスペクタ: 選んだメッシュが知らせの対象なら上に「Propose the rig again」と「Open AI tab」(`panels/inspector/StaleRigNotice.svelte`)。
  - **絵の重なりで見つけた差分の追従** (`module_rig/pose.rs`): 差分の見つけ方を `Source { Name, Hint, Drawing }` にし、AI の手がかりと
    絵の重なりの組は `linked_by_drawing` で本体の要素に対応付ける (見えている衣装 → 本体のグループの最初の要素、隠してある衣装 → 残りの
    要素で IoU が最大のもの。対応する要素がすべてスロットの公開する要素で、どの組も絵が重なるときだけ `links`)。
  - **AI の分類の差分の手がかり** (`classify.rs`): 出力の形の最上位に `variant_sets` (同じ部品の別の絵のレイヤー番号の組) を足し
    (strict の構造化出力のため必須)、`LayerLabel::variant` (組の最も小さい番号) に入れる。指示は各リクエストの本文に書く (システム
    プロンプト = MCP の `classification_instructions` は変えない)。答えに項目がなければ手がかりを変えない (MCP の `set_layer_labels` は
    `layers` だけ)。作り直しでは前の組をプロンプトに書く。`MeshPart::variant` を経て、`pose.rs` の `hinted_variants` が名前の次・絵の重なりの
    前に組にする (見えているものが先頭)。AI タブの分類の「Labels」に「Alternative drawings of the same part」(IPC: `LayerLabelView.variant`)。
  - **ポーズの画面:** 要素のドラッグでの並べ替え (同じグループの中。`moveEntryTo`)。「Show the pose in the viewport」で編集中のビューポートに
    ポーズを効かせ、要素の「Show」で表示する要素を切り替える (`ViewportController.setPosePreview`: ポーズの段を有効にし、`show_pose` と
    `update(3600)` でフェードを待たずに切り替える。モデルを読み直しても続け、タブを離れるとやめる)。プレビューモードでは右の列が隠れるので、
    編集中のビューポートで確かめる形にした。`ugoe-wasm-stub.d.ts` に `show_pose` を足した (wasm は変えていない)。
  - 文書: editor.md (作り直しの知らせ・ポーズの画面・分類の表示・IPC)、ai-integration.md (分類の出力と手がかり・ポーズの見つけ方と追従)、
    spec/file-formats.md (`editor.rigged_meshes`)、manual/dress-up.md・ai.md、ugoe-ai の README。
- **録画:** team-lead の承認を得て、手で作った OpenAI 互換の録画 `openai_classify_layers_bustup.json` の答えにだけ `"variant_sets": []` を足した
  (答えの形に必須の項目が増えたため。`layers` の判断の中身は変えていない。再生は method と URL だけを照らし合わせる)。Claude の録画は変えず、
  欠けを手がかりなしとして読むことをテストで確かめた (tests/classify.rs の冒頭にも書いた)。
- **テスト:** ugoe-ai `variant_sets_mark_alternative_drawings` (組の番号・1 枚の組を捨てる・範囲の外と重複を断る・作り直しで置き換える・
  項目のない答えは変えない)、録画の再生で手がかりがないこと、`module_rig_sample.rs` の `drawn_variants_follow_the_base_shown_and_hidden_entries`
  (見えている袖 → Arm A、隠したカフス → Arm B。公開しなければ衣装だけのグループ) と `ai_variant_hints_become_poses` (重ならない帽子とベレー帽。
  手がかりがなければ断る)。エディタ `rigged_costume_shapes_are_kept_in_the_project`。Vitest (ドラッグの並べ替え・表示する要素・分類の組)。
- **確かめたこと:** main (98c152b、T0658・T0720 の後) に rebase 後、`just wasm` 2 つ・`just gen` (差分なし)・`just ci`・`just test-gpu`・
  `cargo check -p ugoe-editor`。ヘッドレスの Chrome と使い捨ての HTTP ブリッジ (scratchpad/T0523/bridge。T0521 のものに指紋の保存と読み込み、
  `variant` と、AI の答えの代わりに環境変数で手がかりを付ける経路を足した。ブリッジはエディタの規則を写したもので、本物の保存の規則は単体テスト):
  本体で Arm B を Arm A の上へドラッグ → `set_pose` の順が入れ替わる → Undo。髪と服のグループを足し「Show the pose in the viewport」→ 服が消え、
  服の「Show」→ 髪が消えて服が出る → 並べ替えて読み直しても続く → Inspector タブへ移ると全部見える。`hats.ugoeproj` で分類 (hat と cap を組に) →
  「Labels」に「hat / cap」→ 直接写す形で提案 (理由に「the AI classified “hat”, “cap” as alternative drawings」) → 採用 → 化粧の頂点を消す →
  ツリーで化粧を選ぶとインスペクタに知らせ → 保存 (`project.json` に `editor.rigged_meshes`) → 別のプロジェクトを開いて戻る → タブに「AI •」と
  知らせ → Dismiss。横のはみ出し 0。スクリーンショット: scratchpad/T0523/shots/t0523-01〜10 (04b を含む)。ブリッジと vite preview は止めた。
  (scratchpad = /tmp/claude-1000/-home-rosylilly-src-github-com-rosylilly-ugoe/32f91561-94f7-4861-9c03-8fe8c9ff7192/scratchpad)
- **確かめていないこと:** 実際の Tauri のウィンドウ。実際の AI の API が `variant_sets` を答えること (録画と FakeProvider だけ。OpenAI の strict で
  入れ子の配列の形が受け入れられるかも実機では見ていない)。本物の本体を保存して開き直す経路での指紋の照合 (テストは同じメタ情報から作り直した
  セッション。テストの本体はテクスチャを持たず保存できなかった)。ポーズの見た目の確認で、見本の腕 A / B は服の写しなので見分けられず、髪と服で確かめた。
- **範囲外の変更:** なし (ugoe-ai・apps/editor・docs。editor-core と Command は触っていない。`OP_COUNT` は 101 のまま)。
- **後続の候補:**
  - プレビューモードのパネルにもポーズの要素の切り替えを出す (今は編集中のビューポートだけ。プレビューのパネルは T0655 の範囲だったので避けた)。
  - 名前で見つけた差分が本体の文字と合わないとき、見えている / 隠してあるでの追従に回す。
  - モジュールの名前を変えたら `editor.rigged_meshes` の ID が合わなくなる (今は名前は module.json から変わらない)。
  - ugoe-mcp の `set_layer_labels` に `variant_sets` を受ける (今は分類の手がかりはエディタの AI の経路だけ)。
