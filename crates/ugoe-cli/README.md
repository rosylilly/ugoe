# ugoe-cli

**層:** T4 アプリ (依存してよいクレートは `scripts/check-layers.mjs` を参照)

`ugoe` コマンド。モデルの検証・情報表示・フィクスチャの出力・スナップショット描画・配布用バンドルの書き出し。

## 使い方

リポジトリ内では `just cli <args>` (= `cargo run -q -p ugoe-cli -- <args>`) で実行できる。

入力 (`INPUT`) は次のどれか。`validate` / `info` / `render` で共通。

- モデル JSON (`model.json`)
- `.ugoe` バンドル (拡張子ではなく先頭のマジック `UGOE` で判別する)
- 組み込みフィクスチャ `fixture:<name>` (`ugoe_model::fixtures` の関数名。`single-quad` のような kebab-case も可)

`validate` と `info` はこのほかに着せ替えのモジュール `.ugoemod` も受け付ける (下の「モジュールの検査」「モジュールの概要」)。
バンドルと同じコンテナなので、これも拡張子ではなく中身 (`MODL` の代わりに `MODU` チャンクを持つ) で判別する。`render` にモジュールを
渡すとエラーになる (モジュールは本体に付けないと描けないので、`render` では `--module` で付ける)。

入力のファイルは、利用者が自分で渡すものなので通常のファイルに限らない (`/dev/stdin` や `<(...)` の FIFO も読める。
`ugoe build` の入力も同じ)。外からパスを受ける MCP サーバやアプリは通常のファイルだけを読む (T0530 の線引き)。
`render` の `--texture`・`build` の `--meta` などの補助のファイルと、`import-psd` の PSD は通常のファイルだけを読む (下の各節)。

```sh
ugoe validate model.json                 # 検証。問題なければ "ok"
ugoe validate model.ugoe --profile avatar  # アバタープロファイルへの準拠も検査 (準拠レベルを表示)
ugoe validate model.json --physics hair.physics.json  # 物理の設定も検査 (バンドルは同梱の物理を使う)
ugoe validate long-hair.ugoemod           # 着せ替えのモジュールを単体で検査
ugoe validate dress-up.ugoe --module long-hair.ugoemod  # 本体に付けた結果を検査
ugoe info fixture:single_quad            # 概要
ugoe info model.ugoe --json              # 機械で読みやすい JSON (パラメータの ID・値域を含む)
ugoe info long-hair.ugoemod              # 着せ替えのモジュールの概要
ugoe fixture list                        # フィクスチャ名の一覧 (名前を省略しても同じ)
ugoe fixture single_quad > quad.json     # フィクスチャを JSON で書き出す
ugoe standard-params                     # 標準パラメータ・拡張パラメータの一覧
```

### モジュールの検査

- `ugoe validate <module>.ugoemod`: モジュールを単体で検査する (`ugoe_model::validate::validate_module`。
  [docs/spec/modules.md](../../docs/spec/modules.md))。本体との整合 (スロット・パラメータ・マスクの参照先) は調べない。
  `--profile` `--physics` `--module` とは一緒に使えない。
- `ugoe validate <base>.ugoe --module <module>.ugoemod ...`: `render` の `--module` と同じ規則でモジュールを本体に付け
  (付けられなければエラー)、付けた結果のモデルと物理を検査する。`--physics` はバンドルの物理の代わりになり、モジュールのリグはそれに加わる。
  `--profile` は付けた結果のモデルを検査する。入力は `.ugoe` バンドル。
- モデルやモジュールが検証に通らないときは、各エラーを標準エラーに `error: ...` で 1 行ずつ (件数の上限なしに) 書き、最後に件数 (`N validation error(s)`) で失敗する (バンドルとその物理も同じ)。バンドルのモーションと表情の数値のエラーは `error: <名前>: ...`、モーションのカーブの時刻が減る点は ``warning: motion `<名前>`: ...`` (失敗にしない。`docs/spec/model.md#モーションと表情の検証`)。

### モジュールの概要

`ugoe info <module>.ugoemod` はモジュールの名前・取り付け先のスロット・本体に求める条件 (`requires` のアバタープロファイルの
レベル・スロット・パラメータ)・使うパラメータ (`Module::referenced_parameters`)・要素の数 (パーツ・デフォーマ・メッシュ・グルー・
物理のリグと衝突体・ポーズのグループと追従)・テクスチャ (名前・大きさ・形式・法線マップ) を書く。`--json` では同じ内容を
`"kind": "module"` の JSON で書く (モデルの JSON は `"kind": "model"`)。

### `ugoe render`

ヘッドレスの wgpu で描画して PNG を書き出す。AI エージェントが見た目を確かめる手段でもある。
描画の中身は [`ugoe-headless`](../ugoe-headless/README.md) にあり、MCP サーバの `render_png` (`ugoe-mcp`) も同じ関数を呼ぶ。

```sh
ugoe render fixture:single_quad -o out.png
ugoe render model.ugoe -o out.png --param AngleX=10 --param EyeLOpen=0 --width 512
ugoe render model.json -o out.png --size 800x600 --background '#ffffff' --texture 0=atlas.png
ugoe render fixture:nested_masks -o out.png --scale 2 --background black --backend gl
ugoe render model.ugoe -o shake.png --motion Shake --time 0.25   # モーションの 0.25 秒の絵 (物理も進める)
ugoe render model.ugoe -o lit.png --ambient 0.3 --light directional:1,1,-1   # 左上の手前から照らす
ugoe render model.ugoe -o lamp.png --ambient 0.2 --light point:512,300,200:1.5,1.2,1:900
ugoe render dress-up.ugoe -o dressed.png --module long-hair.ugoemod --module sailor.ugoemod   # 着せ替えのモジュールを付けて描く
ugoe render fixture:single_quad -o flipped.png --size 256x256 --view=-1,0,0,1,256,0   # 左右を鏡映して描く
```

| オプション | 内容 |
| --- | --- |
| `-o, --output <PATH>` | 出力する PNG |
| `--param ID=VALUE` | パラメータの値 (複数可)。未知の ID はエラー (既知の ID を列挙する)。値域の外はクランプされ、警告を出す |
| `--size WxH` | 出力の大きさ |
| `--width` / `--height` | 片方だけならキャンバスの縦横比から他方を決める |
| `--scale <F>` | 大きさを指定しないときの、キャンバスに対する倍率 (既定 1 = キャンバスと同じ大きさ)。長い辺が 16384 を超えるなら縮める (下の「上限」) |
| `--fit contain\|stretch` | `contain` (既定) は縦横比を保って中央に収める。`stretch` は出力全体に引き伸ばす |
| `--view A,B,C,D,E,F` | ビュー変換を直接与える (`--fit` と同時には使えない)。キャンバスの点 `(x, y)` を出力の `(A*x + C*y + E, B*x + D*y + F)` (画素、左上が原点・Y 下向き) に写す。鏡映・回転もできる。公開仕様の描画の例の `view` と同じ意味 (T0414) |
| `--background <COLOR>` | `transparent` (既定)、`white`、`black`、`#RRGGBB`、`#RRGGBBAA` |
| `--texture INDEX=PATH` | テクスチャを PNG で差し替える (複数可) |
| `--time <SECONDS>` | `ugoe::Puppet` でこの秒数だけ時間を進めてから描く (物理・モーション・表情・目パチ・呼吸・ポーズ) |
| `--motion <NAME\|PATH>` | 再生するモーション。バンドルのモーションの名前か `.motion.json` のパス。未知の名前はエラー (既知の名前を列挙する) |
| `--expression <NAME\|PATH>` | 使う表情。バンドルの表情の名前か `.expression.json` のパス |
| `--physics <PATH>` | 物理の設定の JSON。省略するとバンドルの物理を使う。モデルと合わなければエラー。`--module` のモジュールのリグはこれに加わる |
| `--no-physics` / `--no-effects` | 時間を進めるときに物理 / 目パチと呼吸を止める |
| `--fps <N>` | 時間を進めるときの 1 秒あたりの `update` の回数 (既定 60) |
| `--ambient <COLOR>` | 環境光の色。`V` (灰色) か `R,G,B` (0 以上、1 を超えてもよい)。既定 1 (白) |
| `--light <LIGHT>` | 光源 (複数可)。`directional:DX,DY,DZ[:COLOR]` か `point:X,Y,Z[:COLOR[:RANGE]]` (下の「ライティング」) |
| `--module <PATH>` | 本体に付ける着せ替えのモジュール (`.ugoemod`、複数可。付ける順は指定の順)。入力は `.ugoe` バンドル。合成 (`ugoe_runtime::compose`) と同じ規則で付け、付けられなければエラー。モジュールの物理のリグは本体の物理に加わる (本体に物理が無ければ重力 `(0, 1)`・60 Hz の設定に加える) |
| `--backend <NAME>` | wgpu のバックエンド (`vulkan` `metal` `dx12` `gl`、カンマ区切り)。省略時は `WGPU_BACKEND`、それも無ければ wgpu の既定 |
| `--gpu-timeout <SECONDS>` | GPU を待つ時間の上限 (既定 30 = `ugoe_headless::DEFAULT_WAIT_TIMEOUT`)。超えたらエラーで終わる。`0` は上限なし。負・`inf`・`NaN` はエラー (下の「GPU」) |
| `--stats` | 描画の統計 (`ugoe_render::FrameStats`) を標準出力に `stats: {"drawables":21,...}` の 1 行 (JSON、キーは名前順) で書く |

成功すると標準出力に 1 行 (`wrote out.png (256x256, adapter: ...)`) を書く。警告は標準エラーに `warning: ...` で出す。

- **テクスチャ:** バンドルの PNG と KTX2 はデコードして乗算済みアルファにする。KTX2 は圧縮していない 8bit の RGB / RGBA と、ブロック圧縮の
  BC1〜BC5・BC7 (`_UNORM`・`_SRGB`。超圧縮は無しか zstd) だけを、保存された値のまま読み (BCn は展開してから乗算する)、レベル 0 だけを使う
  (`docs/spec/rendering.md#前提`)。ほかの形式 (ETC2・ASTC・Basis Universal など) はエラーになる (`--texture` で PNG を渡せば描ける。`--texture` には KTX2 も渡せる)。
  JSON とフィクスチャはテクスチャを持たないので、`model.textures` の各項目に不透明な白 1x1 を使う。
- **上限:** 外から来る値で止まったりメモリを使い果たしたりしないように、次を超えるとエラーにする。
  - 出力の大きさ: 一辺 1..=16384 (`ugoe_headless::MAX_SIZE`)。`--size`・`--width`・`--height` で明示した値と、`--width`・`--height` の
    片方から縦横比で決めた他方が超えたらエラー。**大きさを指定しないとき** (キャンバスの大きさ × `--scale`) は、長い辺が 16384 を超えるなら
    縦横比を保って 16384 に収まるよう縮めて描き、``warning: the canvas (WxH) is larger than 16384 pixels on a side; rendered at ...`` を出す
    (`ugoe_headless::RenderOptions::shrink_to`。MCP の `render_png` も同じ規則で、上限が 4096)。
  - 時間を進める回数: `floor(time * fps)` が `ugoe_headless::animate::MAX_UPDATES` (216000 = 60 fps で 1 時間) まで。
    `--time` は 0 以上の有限の数、`--fps` は 0 より大きい有限の数。
  - テクスチャ (`--texture` とバンドルの PNG) と法線マップ: 一辺 16384 まで (PNG の見出しの大きさで、デコードの前に確かめる)。
  - `--texture`・`--physics`・`--motion`・`--expression` のパス: 通常のファイルだけを読む (装置・FIFO・ディレクトリはエラー)。
  - MCP サーバの `render_png` は、明示した `width`・`height` (片方だけなら縦横比から決めた他方も) を 4096 まで、`time` を 3600 秒 (60 fps) までに限り、大きさを指定しないときは
    4096 に収まるよう縮める (`ugoe-mcp` の `src/render.rs`。描いた PNG をエージェントに返すので小さくする)。
- **色:** 値はリニア化しない (`docs/spec/rendering.md` の「前提」)。描画先は `Rgba8Unorm`・乗算済みアルファで、
  PNG に書くときストレートアルファに戻す (アルファ 0 の画素は RGB も 0)。
- **背景:** 既定は透明。透明な背景の上に描いた結果は、そのまま別の画像に合成できる。
- **ビュー:** キャンバス座標 (Y 下向き) を出力画像の上を上として写す。`--view` を与えればそのアフィン変換で写す
  (負の数から始まるので `--view=-1,0,0,1,256,0` とも書ける)。裏面カリングは表裏をキャンバス座標で決めるので、鏡映しても変わらない。
  逆変換の無いビュー (`A*D - B*C = 0`) は何も描かない (`docs/spec/rendering.md#ビュー変換`)。描画の例 (`spec/examples/`) の `view` の
  ケースも、このオプションで黒箱として比べられる (`tools/spec-check/src/render-reference.ts` はまだ飛ばす。T0414 のメモ)。
- **パラメータ:** `--time`・`--motion`・`--expression` のどれも無ければ、物理・モーション・ポーズのフェードは適用しない。
  指定した値 (と既定値) で 1 回 `update` した状態を描く。
- **時間を進める (`ugoe_headless::animate`):** `--time`・`--motion`・`--expression` のどれかがあれば、`--param` の値を基準値にして
  `Puppet::update(1 / fps)` を `floor(time * fps)` 回、余りがあればもう 1 回その長さで呼んでから描く (`--time` を省略すると 0 秒で、
  `update(0)` を 1 回)。モーションは優先度 `Force` で 0 秒から再生する。適用順は `docs/spec/motion.md#フレーム内の適用順`。
  固定の刻みで進めるので、同じ引数からは常に同じ絵になる。標準出力に `advanced 0.25s in 15 update(s)` も書く。
  手順は `docs/testing.md#モーションの途中の絵を確かめる`。
- **ライティング** (`docs/spec/rendering.md#ライティング`): バンドルの法線マップ (`NRML`) はアルファを掛けずにデコードしてレンダラに登録する
  (KTX2 はテクスチャと同じ形式を読む。読めない形式は警告を出して使わない。`--texture` で差し替えても法線マップはバンドルのもの)。`--ambient` も `--light` も無ければ光源を渡さない
  ので、法線マップやライティングの指定があっても出力は 1 画素も変わらない。どちらかがあれば、環境光 (既定 1) と光源を渡す。
  - 座標はキャンバス座標 (ピクセル・Y 下向き) + 奥行き (z、キャンバスから手前向き・ピクセル)。
  - `directional:DX,DY,DZ[:COLOR]`: 平行光源。`DX,DY,DZ` は **光が進む向き** (長さは問わない。0 はエラー)。
    例: 左上の手前から照らすなら `directional:1,1,-1`。
  - `point:X,Y,Z[:COLOR[:RANGE]]`: 点光源。`RANGE` (正の数か `inf`、既定 `inf`) で 0 になるように減衰する (`(1 − d / RANGE)²`)。
  - `COLOR` は `--ambient` と同じ形で、既定 1 (白)。
  - ライティングを受けるのは `Model::lighting` と `Mesh::lit` の両方が `true` のメッシュだけ。受けるメッシュは `rgb × (環境光 + Σ 光源)`
    になるので、光源を足すときは `--ambient` を 1 より下げないと白く飛ぶ。参照レンダラが使うのは先頭の 8 個まで。
- **GPU:** サーフェス無しでアダプタを要求する。見つからなければエラー。GPU の無い環境では Mesa の lavapipe (Vulkan) か llvmpipe (GL) を使う。
  `WGPU_POWER_PREF` も参照する。
- **GPU を待つ時間:** 読み戻しは `--gpu-timeout` (既定 30 秒) まで待ち、超えたら
  `rendering did not finish within --gpu-timeout (...): the GPU did not finish in time: ...` で失敗する (ドライバが止まっても戻るように。
  `ugoe_headless::Gpu::set_wait_timeout`)。遅いソフトウェアの GPU (lavapipe・llvmpipe) で大きなモデルや大きな画を描くときは延ばし、
  `0` なら上限なしで待つ。
- **WSL2 の注意:** dzn (Vulkan on D3D12) ドライバは wgpu のインスタンスやデバイスの破棄時に SIGSEGV を起こすことがあるので、
  CLI はこれらを破棄せずにプロセスを終える (`ugoe_headless::Gpu::leak`)。

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
入力の PSD は通常のファイルだけを、`ugoe_psd::MAX_PSD_FILE_BYTES` (2 GiB) まで読む (`ugoe_fs::read_regular_file`。エディタ・MCP サーバと同じ)。
装置 (`/dev/zero`)・FIFO・ディレクトリと上限より大きいファイルはエラーにする (`ugoe sample avatar` の `--psd` も同じ)。

### `ugoe build`

`.ugoeproj` プロジェクトか `.ugoe` バンドル、または手で書いたモデルの JSON から、配布用の `.ugoe` バンドルを書き出す。
`.ugoemod` モジュールやモジュールの JSON からは `.ugoemod` を書き出す。`--atlas` でテクスチャをアトラスにまとめる
(規則は [docs/spec/file-formats.md#テクスチャのアトラス化](../../docs/spec/file-formats.md#テクスチャのアトラス化))。

```sh
ugoe build character.ugoeproj -o character.ugoe --atlas
ugoe build character.ugoe -o packed.ugoe --atlas --atlas-max-size 4096
ugoe build long-hair.ugoemod -o packed.ugoemod --atlas   # モジュールのテクスチャをモジュールの単位でまとめる
ugoe build quad.model.json -o quad.ugoe              # 手で書いたモデルの JSON から (画像は隣の textures/<name>.png)
ugoe build hair.module.json -o hair.ugoemod --compress   # 手で書いたモジュールの JSON から
ugoe build quad.model.json -o quad.ugoe --meta meta.json --physics quad.physics.json \
  --motion idle.motion.json --motion nod.motion.json --expression smile.expression.json   # META と物理・モーション・表情を添える
ugoe render packed.ugoe -o check.png                 # 見た目が変わっていないことを目で確かめる
```

| オプション | 内容 |
| --- | --- |
| `-o, --output <PATH>` | 書き出す `.ugoe` バンドル (入力がモジュールなら `.ugoemod`) |
| `--atlas` | テクスチャをアトラスにまとめる (モジュールはモジュールの単位で) |
| `--atlas-max-size <PX>` | アトラス 1 枚の幅と高さの上限 (既定 2048) |
| `--atlas-padding <PX>` | 各画像の周りに複製する端の画素の幅 (既定 2) |
| `--compress` | 画像 (`TEXR` `NRML`) 以外のチャンクを zstd で圧縮する (既定は無圧縮) |
| `--meta <PATH>` | `META` に書くメタデータ (作者やライセンスなど。自由な JSON)。どの入力にも使え、入力のバンドル・モジュールの `META` の代わりにする |
| `--physics <PATH>` | 添える物理の設定 (`*.physics.json`)。入力がモデルの JSON のときだけ |
| `--motion <PATH>` | 添えるモーション (`*.motion.json`。複数指定可で、並べた順に書く)。入力がモデルの JSON のときだけ |
| `--expression <PATH>` | 添える表情 (`*.expression.json`。複数指定可で、並べた順に書く)。入力がモデルの JSON のときだけ |

- 入力はディレクトリならプロジェクト、先頭のマジックが `UGOE` のファイルならバンドル (かモジュール)、ほかのファイルなら JSON として読む。バンドルの物理・モーション・表情・法線マップ・`META` は引き継ぐ。
  テクスチャと法線マップは PNG であること (KTX2 はエラー)。アトラスを使わなければ、読んだバンドルと同じ内容のバンドルを書く
  (`assets/compat/dev` の全ファイルで確かめている)。
- 入力が `.ugoemod` モジュールなら、モジュールとして書き直す。`--atlas` では、モジュールのテクスチャだけをモジュールの単位で
  まとめる (本体のアトラスには入れない。画像はメッシュの宣言順で詰める。`ugoe_editor_core::build_module_atlas`)。
  テクスチャと法線マップの名前が重なっていたり PNG でなければエラー。
- 入力が JSON なら、最上位に `slot` があればモジュール (`Module`)、なければモデル (`Model`) として読み (拡張子は見ない)、
  `ugoe_model::validate::validate` / `validate_module` で検証する。壊れた JSON は `parsing ...` で、検証に通らなければバンドルと同じく
  各エラーを `error: ...` で書いてから件数 (`N validation error(s)`) で失敗する。モデルの JSON からはバンドルを、
  モジュールの JSON からはモジュールを書く (`--atlas`・`--compress` はバンドル・モジュールの入力と同じ)。
  - **添えるもの:** モデルの JSON には `--physics`・`--motion`・`--expression` で物理・モーション・表情を添えられる (無ければ書かない)。
    物理はモデルとの参照を `validate_physics` で、モーションと表情は数値を `validate_motion` / `validate_expression` で検証し、
    通らなければ上と同じ形で失敗する。ほかの入力 (プロジェクト・バンドル・モジュール・モジュールの JSON) にこれらを渡すとエラー。
    `META` は `--meta` で添える (無ければモデル・モジュールの JSON からは `META` の無いものを書く)。
    添えるファイル (`--meta` も) は補助のファイルなので、通常のファイルだけをチャンクの上限 (256 MiB) まで読む。
  - **画像の置き場:** テクスチャ (`TextureRef::name`) と法線マップ (`TextureRef::normal_map`) の画像は、プロジェクトと同じく
    JSON の隣の `textures/<name>.png` から名前で読む ([docs/spec/file-formats.md#プロジェクト-ugoeproj](../../docs/spec/file-formats.md#プロジェクト-ugoeproj))。
    通常のファイルだけを、プロジェクトの画像と同じ上限まで読み、PNG でなければエラーにする。
  - **画像なし:** 見つからない画像 (名前がファイル名に使えないものも) は 1x1 の画像で埋め、標準エラーに
    ``warning: texture `<name>` has no image (...); using a 1x1 placeholder`` を書く。テクスチャは不透明な白
    (`ugoe render` が JSON のモデルに使うものと同じ)、法線マップは正面向き (`(128, 128, 255)` = 法線 `(0, 0, 1)`) で埋める。
    テストの資料のように絵が要らないなら、画像を置かずに書ける。
- `--atlas` では標準出力に `atlas: packed 24 texture(s) into 1 atlas(es) [994x1698], kept 0, removed 0 unused` のような要約を書き、
  まとめずに残したテクスチャを標準エラーに `warning: ...` で出す。

### `ugoe sample avatar`

アバタープロファイル ([docs/spec/avatar-profile.md](../../docs/spec/avatar-profile.md)) のレベル 3 に準拠した見本のモデルを、
PSD から `ugoe_editor_core::Command` の列で組み立てて `.ugoe` に書き出す (`src/sample.rs`)。
組み立て方と出来上がりは [assets/samples/README.md](../../assets/samples/README.md)。
モーションと表情 (`src/sample/animations/` の JSON) も入れる ([assets/samples/README.md#モーションと表情](../../assets/samples/README.md#モーションと表情))。
パラメータのグループとポーズ (耳飾りの有無) も持つ ([assets/samples/README.md#パラメータのグループとポーズ](../../assets/samples/README.md#パラメータのグループとポーズ))。

```sh
just cli sample avatar -o assets/samples/avatar.ugoe          # リポジトリのルートで実行する
ugoe sample avatar -o avatar.ugoe --psd path/to/parts.psd --project avatar.ugoeproj
```

| オプション | 内容 |
| --- | --- |
| `-o, --output <PATH>` | 書き出す `.ugoe` バンドル |
| `--psd <PATH>` | 入力の PSD (既定 `assets/fixtures/psd-real/comfyui-see-through-bustup-parts.psd`。レイヤー名 `face`・`eyelash-l`・`eyeclose-l`・`mouth-open` などで部位を探す) |
| `--project <DIR>` | `.ugoeproj` プロジェクトにも保存する (エディタで開いてリグを見る用) |
| `--lit` | 法線マップを付けてライティングを有効にした版を書き出す (`src/sample/lit.rs`。[assets/samples/README.md#avatar-litugoe](../../assets/samples/README.md#avatar-litugoe))。`just cli sample avatar --lit -o assets/samples/avatar-lit.ugoe` |
| `--atlas` | テクスチャを既定の設定でアトラスにまとめた版を書き出す (描画をまとめるエンジンの見本)。法線マップのあるテクスチャはまとめられないので `--lit` とは使えない。`just cli sample avatar --atlas -o assets/samples/avatar-atlas.ugoe` |

### `ugoe sample dress-up`

着せ替えの見本 (スロットと物理のリグを持ち、アバタープロファイルのレベル 2 を名乗る本体 `dress-up.ugoe` と、衣装のモジュール 4 つの `.ugoemod`) を、
指定したディレクトリに書き出す (`src/sample/dress_up.rs`)。絵はコードで描く (`src/sample/paint.rs`)。
中身は [assets/samples/README.md#dress-up](../../assets/samples/README.md#dress-up)。本体にはモーションと表情 (`src/sample/animations/`)、パラメータのグループとポーズ (手を振る) も入れる。

```sh
just cli sample dress-up -o assets/samples/dress-up          # リポジトリのルートで実行する
just cli sample dress-up -o out --project out/projects       # エディタで開くプロジェクトも書く
```

| オプション | 内容 |
| --- | --- |
| `-o, --output <DIR>` | 書き出すディレクトリ (無ければ作る) |
| `--project <DIR>` | エディタで開くプロジェクトも書き出す: 本体 `dress-up.ugoeproj` と、それを参照するモジュールのプロジェクト (`long-hair.ugoeproj` など) を並べる。すでにあるプロジェクトは上書きする |

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
