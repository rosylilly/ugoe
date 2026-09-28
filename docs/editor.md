# エディタ設計

コードは `apps/editor/` (Tauri アプリ) と `crates/ugoe-editor-core/` (ヘッドレスな中核)。

## 構成

```
┌────────────── Tauri WebView (Svelte 5 + TS) ───────────────┐
│  UI パネル (パーツツリー / パラメータ / インスペクタ / タイムライン)  │
│  ビューポート <canvas> ── ugoe-wasm (ランタイム + wgpu レンダラ)   │
│         ▲ .ugoe のバイト列 / 変更の差分    │ invoke(Command)      │
└─────────┼──────────────────────────────┼─────────────────────┘
          │                              ▼
┌─────────┴──────── Tauri バックエンド (src-tauri) ───────────────┐
│  薄い IPC 層 (commands.rs)                                       │
│  ugoe-editor-core::Document  ← 正となる状態 (source of truth)    │
│  ファイル I/O / PSD 取り込み / 自動メッシュ / AI 呼び出し            │
└────────────────────────────────────────────────────────────────┘
```

### 責務の分け方

| 場所 | 持つもの | 持たないもの |
| --- | --- | --- |
| `ugoe-editor-core` | ドキュメントの状態、Command の適用と検証、Undo/Redo、選択状態、プロジェクトの読み書き | UI、Tauri、GPU |
| `src-tauri` | IPC コマンドの定義、ファイルダイアログ、ウィンドウ管理、非同期ジョブ (PSD・AI) の起動 | 編集ロジック |
| フロントエンド | 表示、入力、ギズモ、ビューポートの描画、パラメータスライダーのプレビュー | モデルの正となる状態 |

**ルール: 編集ロジックは必ず `ugoe-editor-core` に書き、ユニットテストします。** Tauri 側とフロントエンド側は、ユーザーの入力を `Command` に変換して送るだけにします。

## コマンド

- `Command` は serde でシリアライズできる enum です (`#[serde(tag = "op")]`)。JSON の例: `{"op":"set_parameter_range","id":"AngleX","min":-30,"default":0,"max":30}`
- **1 コマンド = ユーザーから見た 1 操作 = Undo の 1 単位** です。ドラッグ操作のように途中経過があるものは、フロントエンドでプレビューだけを行い、確定時に 1 つのコマンドを送ります。
- `Batch` は複数のコマンドを 1 つの Undo 単位にまとめます (AI の提案など)。
- `Document::apply` は、コマンドの検証に失敗した場合にモデルを一切変更しません。
- 対象は必ず ID で指定します。配列のインデックスは使いません (同時編集や AI の提案が古くなったときに、別の要素を壊さないようにするため)。
- JSON Schema を schemars で生成し、AI のツール定義と TypeScript の型の元にします (T0603)。
- `Document::apply` は適用後のモデルを `ugoe_model::validate::validate` で検査し、**適用前になかった検証エラーが生じたら拒否します** (`EditError::Invalid` に新しいエラーの一覧が入る)。検証を通るモデルは、どのコマンドを適用した後も検証を通ります。読み込んだ時点で不正なモデルでも、エラーを増やさない編集 (エラーを直す編集を含む) はできます。`Batch` は全体を適用した後に 1 回だけ検査するので、途中の状態は不正でもかまいません。
- 新しい要素の ID は `ugoe_editor_core::ids` (`new_part_id` など) で作ります。`[A-Za-z0-9_]` 以外の文字を `_` に直し、既存の ID と重なれば `_2`、`_3`、… を付けます。

### コマンド一覧

`op` の値と内容です。要素の追加は完全な値 (`Parameter`・`Part`・`Deformer`・`Mesh`) を渡します。「名前の変更」は表示名 (`name`) だけを変え、ID は変えません。

| 対象 | `op` |
| --- | --- |
| パラメータ | `add_parameter`、`remove_parameter`、`set_parameter_range`、`rename_parameter`、`set_parameter_group` (`null` で外す)、`set_parameter_mirror` (`null` で外す。相手側は変えない。自分自身と対にすると値の符号を反転して対応させる) |
| パーツ | `add_part`、`remove_part`、`rename_part`、`set_part_parent`、`set_part_keyforms` |
| デフォーマ | `add_deformer`、`remove_deformer`、`rename_deformer`、`set_deformer_parent`、`set_deformer_part`、`set_warp_keyforms` (`rows` / `cols` は変えない)、`set_rotation_keyforms` |
| メッシュ | `add_mesh` (`meshes` の末尾に追加)、`remove_mesh`、`rename_mesh`、`set_mesh_properties`、`set_mesh_parent`、`set_mesh_keyforms`、`set_mesh_keyform`、`reorder_mesh` |
| その他 | `batch` |

- `set_mesh_properties` は `part`・`texture`・`blend`・`culling`・`masks`・`mask_mode`・`mask_only` のうち、指定したフィールドだけを変えます。
- `set_mesh_keyform` は格子の 1 セルを置き換え、そのセルの由来を `authored` にします。セルは「軸のパラメータ ID → キーの値」の対応で指定します (例: `"cell": {"AngleX": 30, "AngleY": 0}`)。格子のすべての軸を指定し、値はキーと完全に一致させます。軸のない格子は `{}` です。
- `reorder_mesh` は `meshes` の中でメッシュを `before` の直前 (`null` なら末尾) に移します。描画順 (`draw_order`) が同じメッシュの前後関係が変わります。
- `set_deformer_parent` と `set_mesh_parent` は親を付け替えるだけで、座標は変えません (子の座標は新しい親の局所座標として解釈されるので、見た目は変わりうる)。見た目を保つ付け替えは T0607 で作ります。

### 削除の連鎖と拒否

| コマンド | 他の要素が参照しているとき |
| --- | --- |
| `remove_parameter` | **連鎖して直す。** このパラメータを軸に持つキーフォーム格子 (パーツ・デフォーマ・メッシュ・グルー) から軸を取り除き、パラメータの `default` と等しいキーの断面を残す。等しいキーがなければ最も近いキー (等距離なら小さい方) の断面を残す。セルの由来 (`origins`) も同じセルを残す。深度からの回転の `yaw` / `pitch` がこのパラメータなら外す (回転の中心などの設定は残す)。他のパラメータの `mirror` は `null` にし、エフェクト (目パチ・呼吸・リップシンク) の対象からは取り除く |
| `remove_part` | 子パーツ・所属するデフォーマ・所属するメッシュが残っていれば **拒否する** (`EditError::HasChildren` に子の一覧が入る)。ポーズからは取り除く: そのパーツの要素を消し、他の要素の `links` からも外し、空になったグループは消す |
| `remove_deformer` | 子デフォーマ・子メッシュが残っていれば **拒否する**。他のデフォーマの `mirror` からは外す |
| `remove_mesh` | **連鎖して直す。** 他のメッシュの `masks` と `mirror` から外し、このメッシュを使うグルーと当たり判定を削除する |

- 拒否する場合は、先に子を削除するか付け替えます。まとめて行うなら `batch` を使います。
- 物理・モーション・表情は `Model` の外のデータなので、`remove_parameter` はこれらを書き換えません。消したパラメータへの参照が残ることがあります。

### Undo

最初はスナップショット方式 (`Model` を丸ごと clone) で実装します。簡単で確実だからです。
モデルが大きくなってメモリや時間が問題になったら、各コマンドが逆操作を返す方式に移行します ([ADR 0005](adr/0005-editor-command-model.md))。

## ビューポート

[ADR 0003](adr/0003-editor-viewport-in-webview.md) のとおり、ビューポートは WebView 内の wasm で描画します。

- バックエンドはモデルが変わるたびに、モデルのバイト列をフロントエンドへ送ります (`tauri::ipc::Response` で生のバイト列を送る)。
- パラメータスライダーの操作はフロントエンドの wasm の中だけで完結させます (IPC を往復しない)。これにより 60fps を保ちます。スライダーの値はドキュメントの状態ではなく、表示の状態です。
- WebGPU が使えない WebView (Linux の WebKitGTK など) では、wgpu の WebGL2 バックエンドにフォールバックします。
- 大きなモデルで、モデル全体を送るのが遅くなった場合は、差分パッチを送る方式を検討します。

## 画面構成 (M2 の目標)

| 領域 | 内容 |
| --- | --- |
| 左 | パーツツリー、デフォーマツリー (ドラッグで親を付け替え) |
| 中央 | ビューポート (メッシュ編集、デフォーマのギズモ、キーフォームの編集) |
| 右 | パラメータ一覧 (スライダー、キーの追加と削除)、インスペクタ |
| 下 | タイムライン (M3: モーションの編集)、AI の提案パネル (M5) |

## フロントエンドの規約

- Svelte 5 の runes (`$state`、`$derived`、`$effect`) を使います。ストアのライブラリは入れません。
- バックエンドとの通信は `src/lib/ipc.ts` に集約します。コンポーネントから `invoke` を直接呼びません。
- 型は将来 ts-rs で Rust から生成します。それまでは `ipc.ts` に手で書き、Rust 側を変えたら同じ PR で更新します。
- lint と整形は Biome、型検査は `svelte-check` で行います (`just web-check`)。
