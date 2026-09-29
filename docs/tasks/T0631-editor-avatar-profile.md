---
id: T0631
title: エディタ: アバタープロファイルの編集
status: done
milestone: M5
crates: [ugoe-editor-core, apps/editor, docs/manual, "ugoe-cli (sample.rs)", "ugoe-model (avatar.rs の ts 属性)", docs/editor.md]
depends_on: [T0110]
assignee: claude-worker-T0631
---

# T0631: エディタ: アバタープロファイルの編集

## 目的

エディタで `Model.avatar` (準拠レベル・メタデータ・利用許諾・トラッキングの上書き・視線・標準の当たり判定) を設定し、準拠の検査の結果を見られるようにする。

## 受け入れ条件

- [x] `Model.avatar` を設定・変更する `Command` (例: `set_avatar_profile`、部分の更新) と、検証・Undo のテスト
- [x] インスペクタかダイアログで、メタデータと利用許諾を編集できる。既定値は最も制限の強い値
- [x] `check_profile` の結果 (名乗るレベル・満たすレベル・エラーと警告) を表示する
- [x] T0702 の見本のジェネレータが、`Model.avatar` を直接書く代わりにこのコマンドを使う
- [x] 操作マニュアルを更新し、`just ci`・`just gen` (差分なし)・エディタのビルドが通る

## 参考

- `docs/spec/avatar-profile.md`

## メモ

- **コマンド** (`crates/ugoe-editor-core/src/avatar.rs`、テストは `avatar_tests.rs`): `set_avatar_profile` (丸ごと・`null` で外す)、
  `update_avatar_profile` (与えたフィールドだけ。プロファイルがなければ最も制限の強い既定値から作る。`look_at: null` と省略を区別)、
  `set_hit_areas`。拒否するのはモデルによらず不正な値 (`EditError::InvalidAvatarProfile`) だけで、参照の欠けや必要なパラメータの不足は
  準拠の検査が示す。`remove_mesh` と `set_hit_areas` は、なくなった当たり判定を指す標準の当たり判定を外す。OP_COUNT は 90 → 93。
- **準拠の検査:** `avatar::check` と Tauri の `check_avatar_profile` (`ipc.checkAvatarProfile`)。
- **画面:** インスペクタで何も選んでいないときに「Avatar profile」の節 (`panels/inspector/avatar/`)。レベル・メタデータ・利用許諾・
  標準の当たり判定 (Head / Body) を編集し、検査の結果を出す。`InspectorPanel.svelte` は 2 行だけ。
- **見本のジェネレータ** (`crates/ugoe-cli/src/sample.rs`): 当たり判定とプロファイルを `set_hit_areas` と `set_avatar_profile` で書く
  (承認済みの範囲外。受け入れ条件のため)。出力は `assets/samples/avatar.ugoe` とバイト単位で同じ。
- **範囲外の変更 (承認済み):** `ugoe-model` の `AvatarProfile::look_at` に `ts(optional = nullable)` を足した。serde は `None` を
  省略するのに、生成した型が必須 (`look_at: LookAt | null`) だったため (codegen の models に `fixtures::avatar_profile()` を足したら
  `svelte-check` が見つけた)。JSON・スキーマは変わらない。`docs/editor.md` にも節を足した (コードのコメントが参照するため)。
- **確かめたこと:** `just ci`・`just gen` (差分なし)・`just wasm-check`・`vite build`。ビルドした画面をヘッドレスの Chrome で開き、
  本物の `Document` への使い捨ての HTTP ブリッジ (scratchpad、リポジトリ外) につないで、見本のモデル (プロファイルを外したものと
  そのままのもの) で、作成 (許諾が最も制限の強い値)・作者と商用利用と再配布の編集・Ctrl+Z・レベル 3 の準拠・Head の当たり判定の対応・
  名前を空にしたときのエラー表示を確認。インスペクタの横のはみ出しは 0。
- **確かめていないこと:** 実際の Tauri のウィンドウ。
- **後続の候補:** 画面でのトラッキングの上書き・視線 (`look_at`)・公開するスロットの編集、当たり判定そのもの (`set_hit_areas`) の
  作成・削除の画面、ビューア側での利用許諾の表示と警告。
