# ugoe-cli

**層:** T4 アプリ (依存してよいクレートは `scripts/check-layers.mjs` を参照)

`ugoe` コマンド。モデルの検証・情報表示・フィクスチャの出力・スナップショット描画。(予定) 変換。

## 使い方

リポジトリ内では `just cli <args>` (= `cargo run -q -p ugoe-cli -- <args>`) で実行できる。

入力 (`INPUT`) は次のどれか。`validate` / `info` / `render` で共通。

- モデル JSON (`model.json`)
- `.ugoe` バンドル (拡張子ではなく先頭のマジック `UGOE` で判別する)
- 組み込みフィクスチャ `fixture:<name>` (`ugoe_model::fixtures` の関数名。`single-quad` のような kebab-case も可)

```sh
ugoe validate model.json                 # 検証。問題なければ "ok"
ugoe validate model.ugoe --profile avatar  # アバタープロファイルへの準拠も検査 (準拠レベルを表示)
ugoe info fixture:single_quad            # 概要
ugoe info model.ugoe --json              # 機械で読みやすい JSON (パラメータの ID・値域を含む)
ugoe fixture list                        # フィクスチャ名の一覧 (名前を省略しても同じ)
ugoe fixture single_quad > quad.json     # フィクスチャを JSON で書き出す
ugoe standard-params                     # 標準パラメータ・拡張パラメータの一覧
```

### `ugoe render`

ヘッドレスの wgpu で描画して PNG を書き出す。AI エージェントが見た目を確かめる手段でもある。

```sh
ugoe render fixture:single_quad -o out.png
ugoe render model.ugoe -o out.png --param AngleX=10 --param EyeLOpen=0 --width 512
ugoe render model.json -o out.png --size 800x600 --background '#ffffff' --texture 0=atlas.png
ugoe render fixture:nested_masks -o out.png --scale 2 --background black --backend gl
```

| オプション | 内容 |
| --- | --- |
| `-o, --output <PATH>` | 出力する PNG |
| `--param ID=VALUE` | パラメータの値 (複数可)。未知の ID はエラー (既知の ID を列挙する)。値域の外はクランプされ、警告を出す |
| `--size WxH` | 出力の大きさ |
| `--width` / `--height` | 片方だけならキャンバスの縦横比から他方を決める |
| `--scale <F>` | 大きさを指定しないときの、キャンバスに対する倍率 (既定 1 = キャンバスと同じ大きさ) |
| `--fit contain\|stretch` | `contain` (既定) は縦横比を保って中央に収める。`stretch` は出力全体に引き伸ばす |
| `--background <COLOR>` | `transparent` (既定)、`white`、`black`、`#RRGGBB`、`#RRGGBBAA` |
| `--texture INDEX=PATH` | テクスチャを PNG で差し替える (複数可) |
| `--backend <NAME>` | wgpu のバックエンド (`vulkan` `metal` `dx12` `gl`、カンマ区切り)。省略時は `WGPU_BACKEND`、それも無ければ wgpu の既定 |

成功すると標準出力に 1 行 (`wrote out.png (256x256, adapter: ...)`) を書く。警告は標準エラーに `warning: ...` で出す。

- **テクスチャ:** バンドルの PNG はデコードして乗算済みアルファにする。KTX2 は未対応でエラーになる (`--texture` で PNG を渡せば描ける)。
  JSON とフィクスチャはテクスチャを持たないので、`model.textures` の各項目に不透明な白 1x1 を使う。
- **色:** 値はリニア化しない (`docs/spec/rendering.md` の「前提」)。描画先は `Rgba8Unorm`・乗算済みアルファで、
  PNG に書くときストレートアルファに戻す (アルファ 0 の画素は RGB も 0)。
- **背景:** 既定は透明。透明な背景の上に描いた結果は、そのまま別の画像に合成できる。
- **ビュー:** キャンバス座標 (Y 下向き) を出力画像の上を上として写す。
- **パラメータ:** 物理・モーション・ポーズのフェードは適用しない。指定した値 (と既定値) で 1 回 `update` した状態を描く。
- **GPU:** サーフェス無しでアダプタを要求する。見つからなければエラー。GPU の無い環境では Mesa の lavapipe (Vulkan) か llvmpipe (GL) を使う。
  `WGPU_POWER_PREF` も参照する。
- **WSL2 の注意:** dzn (Vulkan on D3D12) ドライバは wgpu のインスタンスやデバイスの破棄時に SIGSEGV を起こすことがあるので、
  CLI はこれらを破棄せずにプロセスを終える (`render::Gpu::leak`)。

### `ugoe import-psd`

PSD を取り込んで新しい `.ugoeproj` プロジェクトに保存する。取り込みの規則はエディタと同じ
([docs/editor.md#psd-の取り込み](../../docs/editor.md#psd-の取り込み))。キャンバスは PSD の大きさになる。

```sh
ugoe import-psd character.psd -o character.ugoeproj
ugoe import-psd character.psd -o character.ugoeproj --bundle character.ugoe --edge-spacing 12
ugoe render character.ugoe -o preview.png           # 取り込んだ結果を目で確かめる
```

| オプション | 内容 |
| --- | --- |
| `-o, --output <DIR>` | 保存する `.ugoeproj` ディレクトリ |
| `--bundle <PATH>` | `.ugoe` バンドルにも書き出す |
| `--name <NAME>` | プロジェクトと最上位のパーツの名前 (既定は PSD のファイル名) |
| `--alpha-threshold` / `--margin` / `--edge-spacing` / `--interior-spacing` | 自動メッシュの設定 (`ugoe-mesher` の `MeshOptions`) |

取り込みの警告は標準エラーに `warning: ...` で出す。

### `ugoe sample avatar`

アバタープロファイル ([docs/spec/avatar-profile.md](../../docs/spec/avatar-profile.md)) のレベル 3 に準拠した見本のモデルを、
PSD から `ugoe_editor_core::Command` の列で組み立てて `.ugoe` に書き出す (`src/sample.rs`)。
組み立て方と出来上がりは [assets/samples/README.md](../../assets/samples/README.md)。

```sh
just cli sample avatar -o assets/samples/avatar.ugoe          # リポジトリのルートで実行する
ugoe sample avatar -o avatar.ugoe --psd path/to/parts.psd --project avatar.ugoeproj
```

| オプション | 内容 |
| --- | --- |
| `-o, --output <PATH>` | 書き出す `.ugoe` バンドル |
| `--psd <PATH>` | 入力の PSD (既定 `assets/fixtures/psd-real/comfyui-see-through-full-body-parts.psd`。レイヤー名 `face`・`eyelash-l` などで部位を探す) |
| `--project <DIR>` | `.ugoeproj` プロジェクトにも保存する (エディタで開いてリグを見る用) |

## テスト

```sh
cargo test -p ugoe-cli                       # GPU を使わないテスト (just ci に含まれる)
cargo test -p ugoe-cli --features gpu-tests  # 実際に描画して PNG の画素を確かめるテスト
```

## 守ること

- 人間にも AI エージェントにも使いやすいように、出力は簡潔にし、機械で読みやすい形式 (`--json`) も用意する
- 引数の誤りは GPU を使う前に報告する (GPU の無い環境でも分かるように)

## 関連タスク

- T0204 (`docs/tasks/T0204-*.md`)
