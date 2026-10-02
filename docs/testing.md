# テスト方針

## 種類

| 種類 | 場所 | 実行 |
| --- | --- | --- |
| ユニットテスト | 各クレートの `#[cfg(test)] mod tests` | `just test` |
| 結合テスト | 各クレートの `tests/` | `just test` |
| スナップショットテスト | `insta` (`tests/snapshots/`) | `just test`。更新は `cargo insta review` |
| 互換性テスト | `crates/ugoe-format/tests/compat.rs` と資料 `assets/compat/` | `just test`。資料の作り直しは [互換性テスト](#互換性テスト) |
| 描画テスト (GPU) | `gpu-tests` feature を持つクレート (`ugoe-render` など) | `just test-gpu` |
| フロントエンドの検査 | Biome + svelte-check + Vitest (`apps/*/src/**/*.test.ts`・`packages/*/src/**/*.test.ts`) | `just web-check` |
| 依存方向の検査 | `scripts/check-layers.mjs` | `just layers` |
| CI のグループの検査 | `scripts/ci-groups.mjs` (すべてのクレートがどれか 1 つのグループに入っているか) | `just groups` ([CI のジョブ](#ci-のジョブ)) |
| 配布物の適合テスト | `tests/conformance` (`ugoe-conformance`)。C ABI・wasm (npm パッケージ。WebGL2 と WebGPU)・Unity (C# の層)・Godot の頂点と画像を参照と比べる | `just conformance` (C コンパイラ・Chrome・GPU が要る。dotnet・Godot は無ければ飛ばす。[下](#配布物の適合テスト)) |
| C ABI の検査 | ヘッダ `crates/ugoe-ffi/include/ugoe.h` が最新か、C の見本 `crates/ugoe-ffi/examples/c/sample.c` のビルドと実行 | `just ffi-check` (cbindgen が要る。[crates/ugoe-ffi/README.md](../crates/ugoe-ffi/README.md#ヘッダの生成)) |

`just ci` はこれらを (GPU テスト・C ABI の検査・配布物の適合テスト以外) すべて実行します。
C ABI の検査はワークスペースの外のツール (cbindgen) と C コンパイラが要るので `just ci` に入れず、CI の `rust (bindings)` ジョブで別に実行します ([CI のジョブ](#ci-のジョブ))。
配布物の適合テストも同じ理由で、CI の `conformance` ジョブで別に実行します (比べ合わせの単体テストは `just ci` に入る)。

## CI のジョブ

`.github/workflows/ci.yml` の Rust の検査は、クレートのまとまり (グループ) ごとの別のジョブで行います。
ワークスペース全体のテストを 1 つのジョブでビルドすると、GitHub Actions のランナーのディスク (空きは約 14GB) を超えるためです
(デバッグ情報ありで約 17GB。T0002)。グループの定義の正は `scripts/ci-groups.mjs` の `GROUPS` で、CI の matrix もここから作ります。

| ジョブ | 内容 |
| --- | --- |
| `rust-checks` | ワークスペースをビルドしない検査: fmt・`just layers`・`just groups`・`just licenses`・`just publish-check`・`just tasks` |
| `rust (runtime)` | ランタイム層 (`ugoe-core` から `ugoe` まで) の clippy とテスト |
| `rust (bindings)` | `ugoe-ffi`・`ugoe-wasm`・`ugoe-conformance` の clippy とテスト、wasm32 (`just wasm-check`)、C ABI (`just ffi-check`) |
| `rust (assets)` | `ugoe-psd`・`ugoe-mesher`・`ugoe-rigging`・`ugoe-headless` の clippy とテスト |
| `rust (tools)` | `ugoe-editor-core`・`ugoe-ai`・`ugoe-cli`・`ugoe-mcp` の clippy とテスト |
| `rust (apps)` | エディタとビューア (Tauri) の clippy とテスト、生成物の検査 (`just gen && git diff --exit-code`)。Tauri のシステムの依存はこのジョブでだけ入れる |
| `rust` | 上の全ジョブの結果をまとめる (ブランチの保護で必須にする検査の名前を、グループが増えても変えないため) |

- 手元では `just lint-group <グループ>` と `just test-group <グループ>` で、CI の 1 つのジョブと同じ clippy とテストを走らせられます。`just ci` は今までどおりワークスペース全体を検査します。
- `-p` で選んだクレートの間でだけ feature が統合されるので、ワークスペース全体では別のクレートが有効にする feature (例: `ugoe-editor-core` が有効にする `ugoe-model/schema`) は、グループの `features` に書きます。`required-features` を持つテストや例 (`gpu-tests` を除く) の feature が書かれていなければ `just groups` が失敗します。
- 各ジョブは、最初にランナーの使わない SDK (.NET・Android SDK・Haskell・CodeQL) を消してディスクを空けます。
- CI では `CARGO_PROFILE_DEV_DEBUG=0` でデバッグ情報を作りません。target がおよそ半分になります。パニックの位置 (ファイルと行) はデバッグ情報がなくても表示されますが、バックトレースには行番号が出ません。
- `Swatinem/rust-cache` の key はグループごとに分け、キャッシュは main でだけ保存します (PR は main のキャッシュから始まる)。

各グループの target の大きさの目安 (T0002 で手元で測った値。`CARGO_PROFILE_DEV_DEBUG=0` で clippy とテストのビルドの後):

| グループ | target |
| --- | --- |
| runtime | 約 0.8GB |
| bindings | 約 0.9GB (wasm32 の clippy を足して約 1.2GB) |
| assets | 約 0.7GB |
| tools | 約 1.4GB |
| apps | 約 2.7GB (`just gen` を足して約 2.8GB) |

### クレートを足したとき

1. `scripts/check-layers.mjs` の `ALLOWED` に登録する (依存方向)。
2. `scripts/ci-groups.mjs` の `GROUPS` の、近いグループの `crates` に足す。どのグループにも入っていないと `just groups` (`just ci` と CI の `rust-checks`) が失敗する。
3. ほかのクレートが有効にする feature の裏にテストがあるなら、そのグループの `features` に足す。
4. 1 つのグループの target が大きくなりすぎたら (目安は 5GB)、グループを分ける。`GROUPS` に足すだけで CI の matrix に入る。グループに固有の手順 (生成物の検査など) があれば、`ci.yml` の `rust-group` に `if: matrix.group == '<名前>'` のステップを足す。Tauri のシステムの依存が要るグループには `tauri: true` を書く。

## フィクスチャ

- ランタイムのテストの入力には `ugoe_model::fixtures` のモデルを使います。バイナリファイルを置くより、コードで組み立てたモデルのほうが差分を読みやすく、スキーマの変更にもコンパイラが追従させてくれるためです。
- 新しい機能には新しいフィクスチャ関数を追加します。**既存のフィクスチャの挙動は変えない** でください。他のエージェントのテストが壊れます。
- 画像や PSD のように、どうしてもファイルが必要な場合は `assets/fixtures/` に置きます。1 ファイル 1MB 以下にし、出所とライセンスを `assets/fixtures/README.md` に書きます。

## 互換性テスト

ADR 0013 の「新しいファイルを古いランタイムで劣化させて読める」「古いファイルを新しいランタイムで読める」ことを、
ファイルの資料 (`assets/compat/`) で検査します。テストは `crates/ugoe-format/tests/compat.rs` で、`just test` (`just ci`・CI の `rust (runtime)` ジョブ) に含まれます。
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
- ファイルの例 (`spec/examples/bundles/`、公開仕様の [File examples](../spec/examples/README.md#file-examples)) は、小さな `.ugoe` と `.ugoemod` のファイルと、
  その一覧 `bundles.json` です。一覧の各項目は、チャンク表 (タグと圧縮の有無の並び) と、読めるファイルなら読んだ中身
  (対応する JSON の文書・`META` の値・画像の全画素・報告される知らない任意の機能)、拒否されるファイルなら拒否する読み込みの手順の名前
  (`feature_declarations` など) と知らない必須の機能を持ちます。ファイルは仕様だけから作った別の書き出し (`tools/spec-check`) で作ります。
  参照実装の読み込み (`read_bundle_with_report` / `read_module_with_report`) が一覧のとおりに読むことを
  `crates/ugoe-format/tests/spec_bundles.rs` で確かめます。食い違う例は理由を付けてテストの `KNOWN_MISMATCHES` に挙げます
  (例のファイルを参照実装に合わせて作り直してはいけません)。
- 描画の例 (`spec/examples/render-*/`、公開仕様の [Rendering examples](../spec/examples/README.md#rendering-examples)) は、`model.json` と、
  テクスチャの画素・描画先の大きさ・ビュー変換 (省略するとキャンバスを描画先に引き伸ばす)・光源・調べる画素の期待値
  (乗算済み、0〜255 の尺度) を持つ `render.json` の組です。
  調べる画素の中心は三角形の辺から描画先の画素で 1 以上離し、テクスチャは 1 色を基本にして、GPU によるラスタライズとフィルタの違いに
  頼らないようにしています。許容誤差は各チャンネル 3 (rendering.md の Conformance と同じ) で、調べる画素はすべて満たします。
  期待値は仕様の式から画素ごとに導いた値で、`tools/spec-check` の CPU のラスタライザ (仕様だけから書いたもの) と一致します。
  使える光源が 8 個を超えるケース (レンダラは 8 個だけを使ってよいので互換の基準の外) は、どの 8 個を使っても、すべてを使っても
  調べる画素が同じになるように選んでいます。
  参照実装の描画が満たすことは GPU を使うテスト (`just test-gpu`、`crates/ugoe-render/tests/spec_render.rs`) で確かめます。
  食い違う画素は理由を付けてテストの `KNOWN_MISMATCHES` に挙げます。
- 時間の例 (`timeline.json` を持つもの、公開仕様の [Timeline examples](../spec/examples/README.md#timeline-examples)) は、フレームごとの
  ホストの操作と `dt` と、そのフレームの後のパラメータの値・パーツの不透明度の上書き・イベント・動的物理の加算量・評価の結果の期待値の列です。
  操作はパラメータの設定、モーションの開始と停止、表情の設定・追加・停止、リップシンクの音量、トラッキングの入力 (`null` で外す)、
  ポーズの要素の表示 (`show_pose`)、エフェクト設定の差し替え (`set_effects`)、物理のリセット、段の有効無効です。
  参照実装の `Puppet` が満たすことを `crates/ugoe/tests/spec_timelines.rs` で確かめ、食い違う例は理由を付けて `KNOWN_MISMATCHES` に挙げます。
- 互換性テストの資料 (`assets/compat/`) との違い: あちらはリリースごとのバイナリのファイルで、古い版のファイルが読めることと、
  知らない機能を劣化させて読めることを確かめる (評価の期待値は持たない)。こちらは仕様の一部 (CC BY 4.0) で、
  評価の規則とファイルの中身の並べ方を例で示し、他の実装が自分を確かめるためのもの。

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


## 配布物の適合テスト

同じモデル・同じパラメータから、すべての配布物が同じ頂点と画像を出すことを検査します ([ADR 0017](adr/0017-single-runtime-distribution.md) の決定 3)。
参照は `ugoe::Puppet` (`ModelInstance`) の評価の結果と `ugoe render` の画像で、許容誤差は公開仕様の評価と描画の適合の基準
(画像は不透明な背景 `#808080` の上で比べる)。ケースは `tests/conformance/cases.json` (見本のモデル・着せ替え・ライティング・
時間を進めた物理と、公開仕様の評価の例・バンドルの例)。

```sh
just conformance            # C ABI・wasm (ヘッドレスの Chrome の WebGL2 と WebGPU)・Unity (dotnet)・Godot。dotnet・Godot が無ければ飛ばす
just conformance --only c   # C ABI だけ
```

ランタイムの評価・描画・C ABI・wasm の API を変えたら実行します。手順・計画の形式・エンジン向けのバインディングを足す手順は
[tests/conformance/README.md](../tests/conformance/README.md)。

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
