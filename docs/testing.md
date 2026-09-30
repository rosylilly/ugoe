# テスト方針

## 種類

| 種類 | 場所 | 実行 |
| --- | --- | --- |
| ユニットテスト | 各クレートの `#[cfg(test)] mod tests` | `just test` |
| 結合テスト | 各クレートの `tests/` | `just test` |
| スナップショットテスト | `insta` (`tests/snapshots/`) | `just test`。更新は `cargo insta review` |
| 互換性テスト | `crates/ugoe-format/tests/compat.rs` と資料 `assets/compat/` | `just test`。資料の作り直しは [互換性テスト](#互換性テスト) |
| 描画テスト (GPU) | `gpu-tests` feature を持つクレート (`ugoe-render` など) | `just test-gpu` |
| フロントエンドの検査 | Biome + svelte-check + Vitest (`apps/editor/src/**/*.test.ts`) | `just web-check` |
| 依存方向の検査 | `scripts/check-layers.mjs` | `just layers` |
| C ABI の検査 | ヘッダ `crates/ugoe-ffi/include/ugoe.h` が最新か、C の見本 `crates/ugoe-ffi/examples/c/sample.c` のビルドと実行 | `just ffi-check` (cbindgen が要る。[crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md#ヘッダの生成)) |

`just ci` はこれらを (GPU テストと C ABI の検査以外) すべて実行します。
C ABI の検査はワークスペースの外のツール (cbindgen) と C コンパイラが要るので `just ci` に入れず、CI の `rust` ジョブで別に実行します。

## フィクスチャ

- ランタイムのテストの入力には `ugoe_model::fixtures` のモデルを使います。バイナリファイルを置くより、コードで組み立てたモデルのほうが差分を読みやすく、スキーマの変更にもコンパイラが追従させてくれるためです。
- 新しい機能には新しいフィクスチャ関数を追加します。**既存のフィクスチャの挙動は変えない** でください。他のエージェントのテストが壊れます。
- 画像や PSD のように、どうしてもファイルが必要な場合は `assets/fixtures/` に置きます。1 ファイル 1MB 以下にし、出所とライセンスを `assets/fixtures/README.md` に書きます。

## 互換性テスト

ADR 0013 の「新しいファイルを古いランタイムで劣化させて読める」「古いファイルを新しいランタイムで読める」ことを、
ファイルの資料 (`assets/compat/`) で検査します。テストは `crates/ugoe-format/tests/compat.rs` で、`just test` (`just ci`・CI の `rust` ジョブ) に含まれます。
仕様は [spec/file-formats.md#機能の宣言と段階的な劣化](spec/file-formats.md#機能の宣言と段階的な劣化) です。

### 資料の置き場所と名前

| ディレクトリ | 中身 | テスト |
| --- | --- | --- |
| `assets/compat/dev/` | 今のコードの書き出し (`write_bundle` / `write_module`) で作ったファイル。1.0 の基準の機能をすべて含むように、フィクスチャごとに 1 ファイル | 今のコードで作り直したものとバイト単位で一致する (`corpus_is_up_to_date`)。今のランタイムで警告なしに読め、評価できる |
| `assets/compat/<版>/` (例 `1.0.0/`) | リリースの時点の `dev/` の写し。**作り直さない、書き換えない** | 今のランタイムで警告なしに読め、評価できる (`every_release_loads_without_warnings`) |
| `assets/compat/future/` | 新しい版で書き出したファイルを模したもの (下の表) | 劣化させて読める / 拒否される |

- ファイル名は中身を表す kebab-case で、拡張子はそのまま (`dynamic-hair.ugoe`・`long-hair.ugoemod`)。版のディレクトリの間で、同じ中身には同じ名前を使います。
- 各版のディレクトリには、モジュールを取り付ける本体 `dress-up-base.ugoe` を必ず置きます (`.ugoemod` はこれに `compose` して評価する)。
- 1 ファイル数 KB に収めます。画像は 1×1 の PNG です。

`future/` の資料:

| ファイル | 中身 | 期待する結果 |
| --- | --- | --- |
| `optional-feature.ugoe` | `dev/dress-up-base.ugoe` に、知らない任意の機能の宣言 (`test.future_optional`)・知らない enum の値 (合成モード・マスクの結合・デフォーマの種類・パラメータの種類・カーブの対象とセグメント・表情の演算)・知らないフィールド・知らないチャンク (`XFUT`) を加えたもの | 警告付きで読める。未知の値には仕様のフォールバックを適用し、評価の結果 (頂点・合成モードなど) は `dev/` の元のファイルと同じになる。書き出しは `UnknownValues` で拒否する |
| `optional-feature.ugoemod` | `dev/long-hair.ugoemod` に同様の変更 (物理の入力の種類を含む) を加えたもの | 警告付きで読め、本体に取り付けた評価の結果が元のモジュールと同じになる |
| `required-feature.ugoe` / `.ugoemod` | 知らない必須の機能 (`test.future_required`) を宣言し、本体のチャンクが今の版では読めない形のもの | `UnsupportedFeatures(["test.future_required"])` で拒否する (デコードより先に機能の宣言を調べる) |
| `newer-schema.ugoe` / `.ugoemod` | `schema_version` を今の版より 1 つ上げ、本体のチャンクを今の版では読めない形にしたもの (`schema_version` は先頭のフィールドでない) | `UnsupportedVersion` で拒否する (デコードより先に `schema_version` だけを読む) |

- `test.` で始まる機能 ID は試験のために予約されていて、本物の機能には使いません。
- `future/` のコンテナはテストの中で仕様だけから組み立て、`ugoe-format` の書き出しのコードを使いません (書き出しの誤りを読み込みのテストに持ち込まないため)。

### 作り直す

形式・フィクスチャ・書き出しを変えて `corpus_is_up_to_date` が失敗したら、資料を作り直してコミットします。

```sh
UGOE_UPDATE_FIXTURES=1 cargo test -p ugoe-format --test compat
cargo test -p ugoe-format --test compat   # 作り直した資料で、もう一度通す
```

- 作り直すのは `dev/` と `future/` だけです。余分なファイルは消します。版のディレクトリには触れません。
- 作り直している間は、ほかのテストが書きかけのファイルを読んで失敗することがあります。2 行目でもう一度実行して確かめます。
- 1.0 より前は形式を壊してよいので (ADR 0004)、`dev/` の差分はそのままコミットしてよいものとします。1.0 以降に `dev/` の既存のファイルの読み込みが変わるなら、それは互換性を壊す変更です (`SCHEMA_VERSION` を上げるか、機能の宣言で表す)。

### 資料を足す

- **リリースのとき**: `dev/` を `assets/compat/<版>/` に写してコミットします (`cp -r assets/compat/dev assets/compat/1.0.0`)。
  1.0 より前はリリースがないので、版のディレクトリはまだありません。1.0 より前の形式は互換性を保証しないため、作りません。
- **基準の機能やフィクスチャを足したとき**: `compat.rs` の `dev_corpus` にファイルを足して作り直します。
- **1.0 以降の機能を足したとき**: その機能を使うファイルを `dev/` に足します (`FEAT` に宣言が入る)。
  その機能を知らない古いランタイムは、ここでは実行できません。古いランタイムは知らない機能を ID によらず同じに扱い、
  知らない値を `Unknown` として読むので、`future/` の資料で「今のランタイムが知らないもの」の扱いを試しておくことが保証になります。
  これまで試していない場所 (新しい enum やチャンク) を拡張するときは、先に `future/` の変更に加え、そのフォールバックをテストします。

## 公開仕様の例

公開仕様の例 (`spec/examples/`、[spec/examples/README.md](../spec/examples/README.md)) は、小さなモデル (`model.json`) と、
パラメータの値ごとの頂点の位置・不透明度・描画順などの期待値 (`expected.json`) の組です。
ランタイムが期待値を満たすことを `crates/ugoe-format/tests/spec_examples.rs` で確かめます (`just test` に含まれる)。

- `model.json` は `ugoe_model::spec_files::example_models` のフィクスチャから `just gen` で作ります (手で編集しない)。
  最新かどうかは `ugoe-model` の `tests/spec_files.rs` と CI の `just gen && git diff --exit-code` が確かめます。
- `expected.json` は仕様の式から手で導いた値です (多くはフィクスチャの doc コメントとランタイムのテストの値)。
  **テストが落ちたときに、ランタイムの出力を写して期待値を合わせてはいけません。** 仕様とランタイムのどちらが誤っているかを調べます。
- 許容誤差は位置 0.01 px、描画順の値 0.001、不透明度・色・マスクの効き具合 0.0001 で、描画順の値の差が 0.001 以下のメッシュの間では描画の順番を問わない (公開仕様の [Tolerances](../spec/examples/README.md#tolerances)。評価の適合の基準と同じ)。
- 例を足すには、`example_models` にフィクスチャを足して `just gen` を実行し、`expected.json` を書きます。`spec/examples/README.md` の表にも行を足します。
- 互換性テストの資料 (`assets/compat/`) との違い: あちらはリリースごとのバイナリのファイルで、古い版のファイルが読めることと、
  知らない機能を劣化させて読めることを確かめる (評価の期待値は持たない)。こちらは仕様の一部 (CC BY 4.0) で、
  評価の規則を例で示し、他の実装が自分を確かめるためのもの。

## ランタイムの数値テスト

- 変形の結果は、許容誤差を付けて比較します (目安は `1e-4` px)。
- 仕様書の式に対応するテストには、テスト名かコメントで仕様書の節を示します。
- 評価の出力全体を `insta` の JSON スナップショットにしてよいですが、スナップショットを更新するときは、PR 本文に「なぜ値が変わったのか」を書きます。

## 描画テスト

- `ugoe-render` の描画テストは、オフスクリーンのテクスチャに描画して画素を読み戻し、期待する画像と比較します (許容誤差付き)。
- GPU を使うテストは各クレートの `gpu-tests` feature の裏に置きます。`just test-gpu` は `scripts/gpu-test-args.mjs` で `gpu-tests` feature を持つワークスペースのクレートを探し、まとめて `cargo test -p <crate> --features <crate>/gpu-tests` します。新しいクレートに描画テストを足すときは feature を定義するだけでよく、justfile や CI を直す必要はありません。
- アダプタが見つからないときは、飛ばさずに失敗させます (feature を明示的に有効にしたときだけ動くため)。
- 期待画像は `crates/ugoe-render/tests/golden/` に置きます。更新するときは PR に新旧の画像を載せます。
- `just test-gpu -- --nocapture` とすると、テストが選んだアダプタ (名前・ドライバ・バックエンド) が表示されます。

### CI での実行

GPU のない GitHub Actions の Linux ランナーでは、Mesa のソフトウェア実装で実行します (`.github/workflows/ci.yml` の `gpu` ジョブ)。バックエンドごとに matrix で 2 本走ります。

| matrix | ドライバ | 意味 |
| --- | --- | --- |
| `vulkan` | lavapipe (Vulkan) | ネイティブの主経路 |
| `gl` | llvmpipe (OpenGL ES / EGL) | WebGL2 相当のバックエンドでも動くことの確認 |

使う環境変数:

| 変数 | 値 | 目的 |
| --- | --- | --- |
| `WGPU_BACKEND` | `vulkan` / `gl` | wgpu が使うバックエンドを 1 つに絞る (テストは `InstanceDescriptor::new_without_display_handle_from_env` で読む) |
| `VK_DRIVER_FILES` (と旧名の `VK_ICD_FILENAMES`) | `/usr/share/vulkan/icd.d/lvp_icd*.json` | Vulkan ローダーが lavapipe 以外のドライバを読まないようにする |
| `LIBGL_ALWAYS_SOFTWARE` | `1` | GL を llvmpipe に固定する |

ジョブは Tauri のシステム依存を入れず、`just test-gpu` が指定するクレートのテストだけをビルドします。ログの `vulkaninfo --summary` (vulkan のみ) と `ugoe-render gpu tests: using adapter ...` の行で、どのドライバで走ったかを確認できます。

### 手元で CI と同じ条件で動かす

Mesa (`mesa-vulkan-drivers` `libegl-mesa0` `libgl1-mesa-dri`) が入っていれば、手元の GPU があってもソフトウェア実装に固定して再現できます。

```sh
icd="$(ls /usr/share/vulkan/icd.d/lvp_icd*.json | head -n 1)"
# lavapipe (Vulkan)
VK_DRIVER_FILES=$icd VK_ICD_FILENAMES=$icd WGPU_BACKEND=vulkan just test-gpu -- --nocapture
# llvmpipe (GL)
WGPU_BACKEND=gl LIBGL_ALWAYS_SOFTWARE=1 just test-gpu -- --nocapture
```

WSL2 では環境変数を付けないと dzn (Vulkan on D3D12) が選ばれることがあります。dzn はインスタンスの破棄時に SIGSEGV を起こすので、テストはデバイスを static に置いて破棄しないようにしています (`crates/ugoe-render/tests/gpu.rs`)。


## モーションの途中の絵を確かめる

モーション・物理・表情の結果は、自動テスト (`crates/ugoe/tests/m3_puppet.rs`・`crates/ugoe-editor-core/tests/m3_workflow.rs`) が
パラメータと頂点の値で確かめます。見た目は次の手順で PNG にして目で確かめます (GPU が無ければ lavapipe / llvmpipe で動きます)。

```sh
# エディタで作ったモデル (物理・モーション・表情を含む .ugoe) の、モーション `Shake` の途中のコマ
just cli render model.ugoe -o shake-0.25.png --scale 2 --motion Shake --time 0.25
just cli render model.ugoe -o shake-0.50.png --scale 2 --motion Shake --time 0.5
# 同じ時刻を物理なしで描き、髪の遅れ (物理の効果) を見比べる
just cli render model.ugoe -o shake-0.25-still.png --scale 2 --motion Shake --time 0.25 --no-physics
# 表情だけ・物理だけ (頭を 30 にした直後から 0.2 秒揺らす)
just cli render model.ugoe -o smile.png --expression Smile --no-effects
just cli render model.ugoe -o swing.png --param AngleX=30 --time 0.2
```

- `--time` は `Puppet::update(1 / fps)` を繰り返して進めるので (既定 60 fps)、物理の揺れもその時刻のものになります。
  `--param` の値は基準値になり、モーションの対象のパラメータはモーションの値で上書きされます。
- 目パチは乱数のシードで決まるので同じ時刻なら同じ絵ですが、見比べるときは `--no-effects` で止めると分かりやすくなります。
- 連番のコマは、`--time` を変えて繰り返し呼びます (例: `for t in 0 0.1 0.2 0.3; do just cli render m.ugoe -o f-$t.png --motion Shake --time $t; done`)。
- 自動の描画テスト `crates/ugoe-cli/tests/m3_render_gpu.rs` (`just test-gpu`) も同じ手順で `shake-*.png` などを
  一時ディレクトリの `ugoe-m3-render/` に残します。

## AI 関連

- 実際の API は CI で呼びません。`FakeProvider` に録画した応答を読ませてテストします。
- 実際の API を使うテストは `#[ignore]` を付け、環境変数 `UGOE_AI_LIVE=1` のときだけ手動で実行します。
