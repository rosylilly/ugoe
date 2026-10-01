---
id: T0655
title: 見本のプロジェクトの変換を editor-core にまとめる・プレビューで公開していないスロットに印・物理の無いときのスロットの衝突体
status: done
milestone: M4
crates: [ugoe-editor-core, ugoe-cli, apps/editor, ugoe, ugoe-runtime, docs]
depends_on: [T0715]
assignee: claude-worker-T0655
---

# T0655: 見本のプロジェクトとスロットの公開の残り

## 目的

T0715 の後続の候補をまとめる。

## 受け入れ条件

- [x] CLI の `save_projects` と、エディタの `sample.rs` にある見本からプロジェクトへの変換 (約 40 行の写し) を editor-core にまとめる
- [x] エディタのプレビューの着せ替えで、アバタープロファイルが公開していないスロットに印を付ける (ビューアは T0715 で隠している)
- [x] スロットが `colliders` で本体の衝突体を公開しているとき、物理の設定を渡さずにモデルだけで作った Puppet では、そのスロットにモジュールを付けられなくなる。扱いを見直す (物理が無ければ衝突体の参照を無視するなど)。仕様 (spec/model.md の Slots と Composition) と合わせ、変えるなら spec/ と docs/spec/ を同じ変更で直す
- [x] 見本をアバタープロファイルのレベル 3 にするかを決める
- [x] エディタの File > Open Sample… が見本の本体の物理しか写さず、見本のモーションと表情 (T0719) がプロジェクトに入らない。変換を editor-core にまとめるときに、モーションと表情も写す (T0719 の後続の候補)
- [x] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0715-sample-and-viewer-slots.md` のメモ

## メモ

- **見本の変換を editor-core に (`crates/ugoe-editor-core/src/bundle_project.rs`):** `project_from_bundle` (バンドルの中身 = モデル・テクスチャと
  法線マップ・物理・モーション・表情。名前はモデルの名前)、`module_project` (保存した本体に `Project::new_module` で作り中身を入れる)、
  `save_bundle_projects(dir, base_name, bundle, modules)`、`module_project_name` (kebab-case + `.ugoeproj`)。CLI の `save_projects` と
  エディタの `sample.rs` の写し (約 40 行ずつ) をこれに置き換えた。テクスチャの取り出しは `load_base` (`.ugoe` の本体) と共有し
  (`bundle_project::images`)、`ProjectError::UnsupportedTexture` を足した (`.ugoe`/`.ugoemod` の PNG でないテクスチャ。本体は従来どおり
  `UnsupportedBaseTexture`)。数が足りなければ `MissingTexture` (従来の `load_base` は黙って短い方に合わせていた)。
  - **見本のモーションと表情もエディタのプロジェクトに入る** (T0719 の後続)。`sample_projects_open_and_export_the_same_modules` で確かめる。
  - **出力が変わらないこと:** `--project` の出力を変更の前後で `diff -r` して同じ。コミットした見本 (`assets/samples`) は変わらない
    (`committed_sample_is_up_to_date`)。新しいテスト `projects_from_committed_files_match_the_generator` (ugoe-cli) は、生成した見本の
    `save_projects` と、コミットしたファイル (エディタが埋め込むもの) からの `save_bundle_projects` が、ファイルの中身まで同じことを確かめる。
    editor-core のテスト 3 件 (`bundle_project_tests.rs`: 書いて開いて書き出すとバイト列で同じ・法線マップ・PNG でない/足りない画像)。
- **物理の無いときのスロットの衝突体 (仕様を変えた):** 本体に物理の設定が無ければ、スロットの `colliders` は何も指さないことにした。
  `ugoe_runtime::compose_physics_without_base(base, &mut composed, settings, modules)` を足し、リグの参照のうちスロットが公開する本体の衝突体を
  外し、合成したモデルのスロットの `colliders` を空にする (合成した物理の `validate_physics` が通る)。スロットにもモジュールにも無い衝突体は
  これまでどおり `UnknownCollider`。`compose_physics` (本体の物理があるとき) は変えていない。Puppet (`set_modules` の本体に物理が無い場合) と
  `ugoe render --module` (本体に物理が無い場合) がこれを使う。テスト: runtime `exposed_colliders_are_ignored_without_base_physics`、
  ugoe (Puppet) `exposed_colliders_are_ignored_without_base_physics` (モデルだけの Puppet で `Chest` を公開するスロットに付けられる。本体として
  覚えたモデルのスロットは変えない)。
  - 仕様: `spec/model.md` の IDs and references inside a module と Composition の手順 2 (物理の箇条)、`docs/spec/modules.md` (合成・検証・
    Puppet での着せ替え)、`docs/spec/model.md` の物理の検証。`just spec` は通る。適合テスト `just conformance --only c` は 53/53 (例は変えていない)。
  - **エディタ (editor-core) は変えていない:** モジュールのプロジェクトの作業用のモデルと書き出しの検査は、本体に物理が無いと既定の設定で
    `compose_physics` するので、物理の無い本体が衝突体を公開していて、モジュールのリグがそれを参照すると今までどおりエラーになる。
    ゆるくすると、保存のときに参照が黙って消える (作業用の物理から分けるため) ので、作者のツールではエラーのままにした (後続の候補)。
  - 見本に肩の衝突体を入れ直すことはしていない (見本の作り直しになるため。後続の候補)。
- **プレビューの着せ替えの公開していないスロット:** `slotRows(slots, modules, published)` の行に `published` を足し、アバタープロファイルの
  `slots` に無いスロットの行に `authors only` の印 (説明つき) を付ける。隠さず付け外しもできる (作者のツールなので)。プロファイルが無ければ
  印は付けない。Vitest 1 件 (`dressUp.test.ts`)。狭いときは印をスロットの名前の下に回す。`docs/editor.md`・`docs/manual/dress-up.md` に書いた。
- **見本をレベル 3 にするか: しない (レベル 2 のまま)。** レベル 3 は拡張パラメータ 8 つ (頬・舌・耳など) に絵の動きが要り、見本の絵と
  モデルの作り直しになる。着せ替えの見本の目的はスロットとモジュール (モジュールの `requires` も 2 で足りる) で、レベル 3 の見本は
  `avatar.ugoe` が既にある。作り直すと見本の指紋も変わり、ビューアが覚えた着せ替えの組が場所の鍵でしか戻らなくなる (T0715・T0719 と同じ)。
- **IPC と生成物:** IPC の型は変えていない (`just gen` の差分なし)。Command は足していない (OP_COUNT は 101 のまま)。
- **確かめたこと:** main (T0658) に rebase した後、`just wasm`・`just wasm apps/viewer/src/lib/wasm/generated`・`just gen` (差分なし)・`just ci`・
  `cargo check -p ugoe-editor`・`just test-gpu`・`just wasm-check`・`just spec`。`just conformance --only c` は rebase の前。
  ヘッドレスの Chrome (SwiftShader) と使い捨ての HTTP ブリッジ (scratchpad/T0655/bridge。`open_sample_project` はコミットした見本を
  `save_bundle_projects` で書く) で: 「Open Sample…」の本体に Motions 4 (`idle`・`nod`・`shake`・`tilt`)・Expressions 3 が入り、プレビューにも出る。
  Slots タブで `Headwear` の「Show to apps」を外してプレビューに入り、`cat-ears.ugoemod`・`long-hair.ugoemod` を読み込むと、`Headwear` の行に
  `authors only` が付き、`CatEars` は付けられる。横のはみ出し 0。使ったブリッジと vite preview は止めた。
  スクリーンショット: /tmp/claude-1000/-home-rosylilly-src-github-com-rosylilly-ugoe/32f91561-94f7-4861-9c03-8fe8c9ff7192/scratchpad/T0655/editor/shots/t0655-0{1,2,3,4}-*.png。
- **確かめていないこと:** 実際の Tauri のウィンドウ (ダイアログ・`include_bytes!` を含むビルド)。ブラウザ以外のホスト (FFI・Unity・Godot) で、
  衝突体を公開するスロットにモデルだけで付けること (どれも Puppet を通るので同じ規則)。
- **範囲外の変更:** なし。
- **後続の候補:**
  - 見本の本体に肩の衝突体を入れ直し、スロット `Hair` の `colliders` で公開する (見本の作り直し。モデルだけのホストでも付けられるようになった)。
  - エディタのモジュールのプロジェクトで、物理の無い本体が公開する衝突体の扱い (今はエラー。本体に物理を足すよう案内するなど)。
