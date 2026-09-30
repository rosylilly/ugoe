---
id: T0647
title: エディタ: 本体にスロットを定義する
status: done
milestone: M4
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: [T0121]
assignee: claude-worker-T0647
---

# T0647: エディタ: 本体にスロットを定義する

## 目的

ADR 0024。本体のプロジェクトで、着せ替えのスロットを定義・編集できるようにする。

## 受け入れ条件

- [x] スロットを足す・変える・消す `Command` (Undo と検証のテスト)。アバタープロファイルの `avatar.slots` との関係を扱う
- [x] スロットの画面: 取り付け先 (anchor・part)、描画順の範囲、置き換える要素、公開するパラメータ・衝突体・ポーズの要素。描画順の範囲をビューポートやツリーで確かめられる
- [x] ヘッドレスのブラウザで確かめる。操作マニュアルに着せ替えの章 (`docs/manual/dress-up.md`) を作り、スロットの節を書く
- [x] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/adr/0024-module-authoring.md`、`docs/spec/modules.md`

## メモ

- **コマンド** (`crates/ugoe-editor-core/src/slot_edit.rs`、テストは `slot_tests.rs`): `add_slot { slot }`・`set_slot { id, slot }`
  (丸ごと置き換え。`slot.id` が違えば名前の変更で、`avatar.slots` の参照も付け替える)・`remove_slot { id }` (`avatar.slots` からも外す)。
  OP_COUNT は 96 → 99 (T0646 はコマンドを足さない)。`EditError::InvalidSlot` を足した。
  - 拒否: ID が `[A-Za-z0-9_]+` でない・重複、公開する衝突体が物理の設定にない (物理がないと `validate_physics` が走らないので
    コマンドで調べる)、**描画順の範囲がほかのスロットと重なる** (端が接するだけはよい。幅 0 の範囲は 1 点の共有でも重なる)。
    読み込んだ時点の重なり (置き換える前の範囲とも重なるもの) は見逃すので、古いファイルでもほかの項目は編集できる。
    参照先と範囲の値 (`min <= max`) は適用後の `validate` が拒否する。
  - 削除の連鎖: `remove_parameter`→`parameters`、`remove_mesh`→`mask_sources`、`remove_part`→`replaces`・`pose_parts`、
    物理の衝突体の削除→`colliders` から外す (最後のものは `Document::apply` の後段で、消えた衝突体をまとめて外す)。
    取り付け先のデフォーマ (`anchor`) とモジュールの親のパーツ (`part`) は `HasChildren` で拒否 (外すと座標や追従が変わるため)。
    `unwrap_deformer` も取り付け先なら拒否される。
- **画面:** 右の列の上に「Slots」タブ (`panels/slots/SlotsPanel.svelte`・`PickList.svelte`・`slots.ts` (Vitest)・`slotsView.svelte.ts`)。
  一覧 (範囲・取り付け先・`app`・`overlap` の印)・「+ Slot」(空いている範囲の手前から幅 50)・各項目の編集と説明・
  「Show to apps」(`update_avatar_profile { slots }`。プロファイルがなければ無効)・「Draw order」(本体のメッシュを手前から並べ、
  スロットの範囲の端の行を差し込み、範囲の中のメッシュに色を付ける。行でメッシュを選ぶ)・「Select meshes in range」
  (範囲のメッシュを選択してビューポートで枠を出す)。`App.svelte` のタブの切り替えを `openTab` にまとめた。
- **マニュアル:** `docs/manual/dress-up.md` (全体の流れの表・スロットの節・準備中の節)、README の目次・screen.md・avatar.md から
  リンク。`docs/editor.md#スロットの編集` と削除の連鎖の表、`docs/spec/modules.md` の冒頭の注記を更新。
- **見本のアバター:** スロットの例は入れなかった (出力は変えていない)。見本のパーツは髪を分けておらず (`face` / `body` など)、
  `replaces` の良い例を作れないため。T0648 で見本のモジュールを作るときに、本体のスロットと一緒に入れるのがよい。
- **確かめたこと:** `just ci`・`just gen` (差分なし)・`cargo check -p ugoe-editor`・Vitest。ビルドした画面をヘッドレスの Chrome で開き、
  本物の `Document` への使い捨ての HTTP ブリッジ (scratchpad/T0647、リポジトリ外) で見本のアバター (プロジェクト) を読み、
  「+ Slot」・名前の変更 (`Hair`)・取り付け先 `HeadRotation`・親のパーツ `face`・範囲 940〜980・マスク `face`・Show to apps・
  2 つ目の `Outfit` (`BodyWarp`・140〜180・Replaces `body`)・範囲を 950 まで広げたときの重なりのエラー (値は 180 に戻る)・
  「Select meshes in range」で `topwear` が選ばれビューポートに枠・Ctrl+Z で Replaces が戻ることを確認。横のはみ出しは 0。
  スクリーンショット: scratchpad/T0647/shots/slots-0{1..5}-*.png (マニュアルの画像はそのうち 2 枚)。
- **確かめていないこと:** 実際の Tauri のウィンドウ。モジュールを実際に付けたときの見た目 (T0649 で)。
- **後続の候補:**
  - 標準パラメータの一覧をフロントエンドに生成して、「Parameters」から標準パラメータを除く (今は全部出して説明で断っている)。
  - ビューポートにスロットの取り付け先 (デフォーマ) を強調する表示。ツリーの Parts の中にも範囲の帯を出す。
  - 見本のアバターにスロットを入れる (T0648 の見本のモジュールと一緒に)。
