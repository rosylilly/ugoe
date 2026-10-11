---
id: T0715
title: 見本とビューア: 見本の本体をレベル 2 にし、公開していないスロットをビューアで隠す
status: done
milestone: M4
crates: [ugoe-cli, ugoe-wasm, assets, apps/viewer, apps/editor, docs]
depends_on: [T0712]
assignee: claude-worker-T0715
---

# T0715: 見本とビューア: 見本の本体のレベル 2 とスロットの公開

## 目的

T0712 の後続の候補をまとめる。

## 受け入れ条件

- [x] 着せ替えの見本の本体をアバタープロファイルのレベル 2 にする (視線・眉・口の形・BodyAngleY/Z など、足りないパラメータを足す)。コミットした見本を作り直し、モジュールが今までどおり付くことを確かめる
- [x] ビューアが `avatar.slots` を読み、アプリに公開していないスロットを利用者に見せない (付け外しの一覧に出さない)。公開の有無の扱いを README に書く
- [x] エディタから見本のプロジェクトを開く入口 (「Open sample…」など。今は CLI で書き出してから開く)。入れるなら IPC と ipc.ts も揃える
- [x] 見本の本体に物理のリグを入れる (モジュールのプロジェクトで、本体のリグの ▲▼ やテンプレートの無効化を見本で確かめられるように。T0653 の後続の候補)
- [x] `just ci`・ビューアのビルド・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0712-sample-dress-up-project.md` のメモ

## メモ

- **見本の本体をレベル 2 に (`crates/ugoe-cli/src/sample/dress_up.rs`):** パラメータに `BodyAngleY/Z`・`EyeBallX/Y`・`BrowLY`・`BrowRY`・`MouthForm` を足し
  (全 15 個、標準の値域)、`avatar.level` を 2 にした (`ugoe validate --profile avatar` は「claims level 2, satisfies level 2」、警告なし)。
  - 体: `BodyWarp` の軸に `BodyAngleY` (上ほど ±4px 上下) と `BodyAngleZ` (腰の下 `(256, 600)` を軸に、上の辺で ±5 度。下の辺は動かさない) を足し、
    `HeadRotation` の中心も同じ `body_pose` で動かし、頭の角度に `BodyAngleZ` の分を足す。`BodyAngleZ` は首の位置が円を描くので 0 もキーにした
    (2 つのキーの間の直線では既定の姿で首が約 1px ずれ、`meshes_sit_where_they_were_drawn` が壊れる)。
  - 目: 瞳とハイライトを別のメッシュ `EyeLIris`・`EyeRIris` (テクスチャ `eye-l-iris` など) に分け、`EyeBallX/Y` で横 ±4px・縦 ±3px 動かし、開いた目 `EyeL`/`EyeR` を
    マスクにしてくり抜く。まばたきは瞳にも同じつぶし方を掛ける。
  - 眉: 1 枚だった `Brows` を `BrowL`・`BrowR` に分け、`BrowLY`・`BrowRY` で ±5px 上下 (正で上)。
  - 口: 格子を 4x2 にし、`MouthForm` で口角を ±4px (正で笑顔)。
- **見本の本体の物理 (T0653 の後続):** 既定の後ろ髪 `HairBack` を回転デフォーマ `HairSway` (`HeadWarp` の子、頭の中心) に入れ、リグ `HairSway`
  (長さ 120px の振り子、`AngleX` の入力。モジュールの揺れものと同じ `pendulum`) で揺らす。重力などは `physics_edit::default_settings()`。
  `Bundle.physics` に入り、`--project` の本体のプロジェクトにも入る。モジュールのプロジェクトの物理のタブでは本体のリグとして並ぶ (▲▼ は無効)。
  - **衝突体は入れなかった:** 最初は肩の衝突体をスロット `Hair` の `colliders` に公開したが、物理を渡さずにモデルだけで作った Puppet
    (`hair_modules_sway_after_a_head_shake` の形。FFI などでモデルだけを読むホストも同じ) では、`Hair` のスロットにモジュールを付けられなくなった
    (`UnknownReference slot Hair colliders → physics collider Shoulders`)。見本のモジュールは使わないので、公開をやめ、衝突体ごと除いた。
  - 生成の中の検査 (`check`) は本体の物理を `validate_physics` で確かめ、合成の物理も本体の物理で作る。
- **コミットした見本の作り直し:** `assets/samples/dress-up/dress-up.ugoe` だけが変わった (52,030 → 74,494 バイト)。**4 つの `.ugoemod` はバイト列で同じ**
  (モジュールの生成は触っていない)。`committed_sample_is_up_to_date` が通る。`dress-up.jpg` は既定の姿の絵が変わらないのでそのまま。
  - **覚えた着せ替えの組 (T0711) への影響:** 本体の指紋が変わる。ビューアの「サンプル」→「着せ替え」で覚えた組は場所 `sample:dress-up/dress-up.ugoe` の鍵で
    引くので、今までどおり戻る (前の指紋の組を localStorage に入れて確かめた: 尋ねる → 戻すで `TwinTails` が付く)。**前の見本をドロップや「開く」で
    開いて覚えた組 (場所が `path:` か無いもの) は、指紋が変わるので、新しい見本では戻らない** (前の見本のファイルを開けば今までどおり戻る)。
- **ビューア: 公開していないスロットを隠す:** WebPuppet に `avatar_slots_json()` (本体のアバタープロファイルの `slots`、プロファイルが無ければ `null`) を足し
  (ugoe-wasm は team-lead の承認を得て crates: に足した。下の「範囲外の変更」)、ビューアの `dressUp.ts` の `visibleSlots`・`parseAvatarSlots` と
  `groupBySlot(slots, modules, published)` で絞る。公開していないスロットは一覧・「n / m スロット」・パネルを出す判定に入れない。そのスロットのモジュールを
  読み込んだときだけ「公開されていない」の印で後ろに出し、付けられない (付いていれば外せる)。モデルを開いた後にそのモジュールを開いても自動では付けず、
  理由を出す (`Player.loadModule` の `#hiddenSlot`)。プロファイルの無いモデルは今までどおりすべて見せる。付け外しそのもの (wasm) は公開の有無を見ない。
  README (`apps/viewer/README.md` の着せ替えの「スロットの公開」、`crates/ugoe-wasm/README.md`) に書いた。Vitest 2 件 (`dressUp.test.ts`)。
- **エディタ: File > Open Sample…:** 「Open Sample…」はフォルダーを選ばせ、その中の新しいフォルダー `ugoe-dress-up-sample` (あれば `-2`・`-3`…。既にあるフォルダーには
  書かない) に本体とモジュールのプロジェクト 4 つを書いて、本体を開く (`src-tauri/src/sample.rs` の `open_sample_project(parent)`、ipc.ts の `openSampleProject`)。
  見本の `.ugoe`・`.ugoemod` は `include_bytes!` でバイナリに埋め込む (約 160KB)。変換は `ugoe sample dress-up --project` の `save_projects` と同じ手順
  (エディタは ugoe-cli に依存できないので、約 40 行を写した)。開いた後の最初の場所は見本のフォルダーにし、続けて「Open…」でモジュールのプロジェクトを開ける。
  型は増やしていない (戻り値は `ProjectStatus`) ので `just gen` の差分はない。テスト: `sample_projects_open_and_export_the_same_modules`
  (書いたプロジェクトを開け、本体のモデルと物理が同じ、各モジュールを書き出すと埋め込んだ `.ugoemod` とバイト列で同じ)、`sample_folders_do_not_overwrite_earlier_ones`。
  文書: `docs/editor.md` (IPC の表・ファイルメニュー・ダイアログの表)、`docs/manual/dress-up.md` の「見本のプロジェクトを開く」(CLI は後ろに残した)。
- **確かめたこと:** main (T0410・T0718・T0127 など) に rebase した後、`just wasm-check`・`just wasm` 2 つ・`just gen` (差分なし)・`just ci`
  (`cargo check -p ugoe-editor` を含む)・`just test-gpu`。bindings/web の手書きの型 (`stub/ugoe_wasm.d.ts`) には `avatar_slots_json` を足していない
  (`pnpm --filter @ugoe/runtime check` は通る。使っていないため。生成物から作り直すときに入る)。
  - `ugoe render` (llvmpipe) で視線・眉・口の形・体の Y/Z、モジュールを付けた体の傾きを描いた: <scratchpad>/T0715/render/row1.png・row2.png。
  - ビューア (ヘッドレスの Chrome、SwiftShader、ビルドした `dist/` を `vite preview`): 「サンプル」→「着せ替え」は今までどおり 3 スロット (`LongHair*`・`CatEars*`・`Sailor*`)。
    `Headwear` を公開しない版 (`--project` の本体の `avatar.slots` を書き換えて `ugoe build`) では「2 / 2 スロット」で `Headwear` が出ず、`cat-ears.ugoemod` を開くと
    付けずに「公開されていない」の行と理由が出る (「付ける」は無効)。横のはみ出し 0。前の指紋で覚えた組が場所の鍵で戻る。
    スクリーンショット: <scratchpad>/T0715/viewer/shots/01-sample.png・02-hidden.png・03-hidden-module.png・04-restored-old-memory.png。読み込み時の 404 が 1 件出るのは T0710 と同じ。
  - エディタ (ヘッドレスの Chrome と使い捨ての HTTP ブリッジ <scratchpad>/T0715/bridge。T0653 のものに `open_sample_project` を足した): 「Open Sample…」→ ダイアログ
    (「Choose a folder to put the dress-up sample projects in」) → `ugoe-dress-up-sample/dress-up.ugoeproj` が開く (レベル 2、スロット 3 つ、パラメータ 15、
    リグ `HairSway`)、知らせにフォルダー。続けて「Open…」はそのフォルダーから始まり、`long-hair.ugoeproj` で `Module: Hair`、物理のタブに `HairSway` (本体の行)と
    `Sway (LongHair/Sway)`、▲▼ はどちらも無効 (説明つき)。もう一度「Open Sample…」で `ugoe-dress-up-sample-2`。横のはみ出し 0。
    **ブリッジの `open_sample_project` は CLI の `--project` で書く** (エディタの `sample.rs` そのものは単体テストで確かめた)。
    スクリーンショット: <scratchpad>/T0715/editor/shots/t0715-0{1,2,3}-*.png。使った vite preview とブリッジは止めた。
- **確かめていないこと:** 実際の Tauri のウィンドウ (ダイアログ・`include_bytes!` を含むビルド)。`.deb` のビルド。見本の新しいパラメータをフェイストラッキングで
  動かすこと (標準の対応なので式は同じ)。エディタのプレビューの着せ替え (エディタの Preview モードは公開の有無を見ていない。T0715 はビューアだけ)。
- **範囲外の変更:** `crates/ugoe-wasm` (`dress_up::published_slots`・`WebPuppet.avatar_slots_json`・テスト 1 件・README 1 行)。ビューアが avatar.slots を読む口が
  wasm に無かったため (.ugoe はバイナリで TS からは読めない)。team-lead の承認を得て crates: に ugoe-wasm を足した。
- **後続の候補:**
  - `save_projects` (ugoe-cli) と `sample.rs` (エディタ) の、バンドルとモジュールからプロジェクトを作る手順を editor-core にまとめる (今は写し)。
  - エディタのプレビューの着せ替えでも、公開していないスロットを区別する (作者のツールなので、隠すより印を付ける方がよいかもしれない)。
  - 見本に衝突体を入れるなら、モデルだけで読むホストでもスロットの `colliders` で付けられなくならないよう、`validate` の扱い (本体の物理が無いときの
    スロットの衝突体の参照) を見直すか、ホストに物理を必ず渡す約束にする。
  - 見本の本体をレベル 3 にする (拡張パラメータ 8 つ)。

