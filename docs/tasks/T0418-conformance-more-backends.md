---
id: T0418
title: 適合テスト: Godot と Unity を乗せる・wasm の描画結果の読み戻しで WebGPU も比べる
status: done
milestone: M7
crates: [tests/conformance, bindings/godot, bindings/unity, ugoe-wasm, bindings/web, docs, justfile]
depends_on: [T0408, T0411, T0405]
assignee: claude-worker-T0418
---

# T0418: 適合テストの対象を広げる

## 目的

T0408 の後続の候補をまとめる。

## 受け入れ条件

- [x] Godot の `build.mjs --conformance` を、tests/conformance の計画 (plan.json) と compare に乗せる (段の止め方の違いと、Godot にだけあるケースの移し方は tests/conformance/README.md に書いてある)
- [x] Unity の Unity に依存しない C# の層 (dotnet) を、頂点の比較に乗せる
- [x] wasm に描画結果を読み戻す API と、不透明度などを取り出す API を足し、ヘッドレスの Chrome の WebGPU でも画像を比べられるようにする (いまは WebGL2 だけ)。bindings/web の型のスタブも揃える
- [x] `just ci`・`just conformance` が通る

## 参考

- `docs/tasks/T0408-conformance-tests.md`・`docs/tasks/T0411-godot-followups.md`・`docs/tasks/T0405-unity-package.md` のメモ、`tests/conformance/README.md`

## メモ

### やったこと

- **wasm:** `WebPuppet::read_pixels()` (`Promise<Uint8Array>`。今の状態を canvas と同じ大きさ・ビュー・背景・光源で canvas とは別の
  テクスチャに描き、バッファに写してマップし、ストレートアルファの RGBA で返す) と `drawable_states_json()` (不透明度・描画順の値・
  乗算色・スクリーン色・マスクの効き具合・`visible`) を足した (`src/web/readback.rs`、GPU に触れない部分は `src/readback.rs` で単体テスト)。
  マップの完了は `setTimeout` でイベントループに戻りながら `poll` して待つ (WebGPU はブラウザが、WebGL2 は `poll` がコールバックを呼ぶ)。
  `bindings/web/stub/ugoe_wasm.d.ts` に 2 行 (生成物と同じ型)。`web.rs` の変更は `mod readback;` の 1 行だけ。
- **wasm の実行器:** 画像は canvas の読み戻しをやめて `read_pixels` で取り、頂点は C ABI と同じすべての値を比べる。
  `wasm` (WebGL2) と `wasm-webgpu` (WebGPU、SwiftShader) の 2 つのバインディングにした (`--backend` はやめた)。
- **Godot:** `build.mjs --conformance` は `node tests/conformance/run.mjs --only godot --no-godot-build` を呼ぶだけにし (`--only` はケース名)、
  `example/tests/conformance.gd` を計画の実行器にした (1 つの Godot の中で全ケースを描き `<ケース>.png` と `skipped.json` を書く)。
  比べ合わせは `ugoe-conformance compare --images-only`。時間を進めないケース (すべての段を止める) のために、ノードに
  `set_stage_enabled` / `is_stage_enabled` を足した (モデルを読み直しても保つ。読み込む前に設定すると読み込みの直後の評価から効く。
  smoke.gd に検査)。Godot だけにあったケースを `cases.json` に移した: マスク・合成モードのモデルは `examples/conformance_bundles.rs` を
  `tests/conformance/src/bundles.rs` に移して `reference` が `DIR/models/` に作る (`generated` フィールド)。値を変えた直後のフレームは
  計画の `before` で表す (Godot だけが使う。C・wasm・Unity・参照は使わない)。godot の Cargo.toml から png の dev-dependency を外した。
- **Unity:** `tests/conformance/unity/` に dotnet の実行器 (`Runtime/Core` の `.cs` をそのまま取り込むコンソールアプリ。net9.0)。
  bindings/unity のファイルは変えていない。
- **比べ合わせ:** `compare --images-only`、実行器の `skipped.json` (飛ばしたケースは失敗に数えない。1 つも比べなければ失敗)、
  `reference --only` (ケースを絞る)。
- **足りない道具 (README に書いた):** `--only` なしでは c・wasm・wasm-webgpu は止まり、unity (.NET SDK 9 以上)・godot (Godot・xvfb-run) は
  入れ方を出して飛ばす。`--only` で挙げたものはどれでも止まる。

### 結果 (WSL2、53 ケース)

- c・wasm・wasm-webgpu・unity: 評価の結果はすべて一致。wasm の画像は WebGL2・WebGPU とも 99.9% 以上 (最大 5〜7/255、回転の例 2 つは 1〜2 画素が 127/255)。
- godot: 扱う 20 ケース (バンドル) が 100% の画素で 3/255 以内。33 ケース (モデルの JSON) は飛ばす。時間を進めるケース (物理・目パチと呼吸・
  着せ替えの物理) も一致した。Godot の画像を別のケースのものに置き換えると落ちることを確かめた。
- `just ci`・`just wasm-check`・`just godot-check`・`just conformance` (すべてのバインディング) が通る。

### 確かめていないこと

- CI の上では動かしていない。`just conformance` の既定に wasm-webgpu と unity が入ったので、`conformance` ジョブ
  (ci.yml は変えていない) で WebGPU (ランナーの Chrome の SwiftShader) と、ランナーに .NET SDK 9 があれば Unity も走る。
  WebGPU がランナーで動かなければ、ジョブを `just conformance --only c,wasm,unity` にするのがよい。
  `godot` ジョブの `just godot-check` は同じコマンドのまま計画の比べ合わせになる (ランナーの上では未確認)。
- Unity のエンジンの上での描画 (比べているのは C# の層の頂点だけ)。Windows・macOS。

### 範囲外

- justfile のコメント 2 か所 (`godot-check` の例の `--only dress-up` は今は無いケース名、`conformance` の説明の「C ABI と wasm」) が古い。
  team-lead に相談したが返事の前なので変えていない。

### 後続の候補

- Godot の頂点の読み戻し (`get_drawable_*`) を足して評価の結果も比べる。モデルの JSON を読めるようにして公開仕様の例も描く。
- Unity の上での画像の比べ合わせ (Unity のライセンスが要る)。Unreal (T0407) ができたら同じ手順で足す。
- `read_pixels` をビューア・エディタのスクリーンショットに使う。

- team-lead が review の後に justfile のコメント 2 か所を直した (godot-check の --only の例、conformance の説明)。承認済み。
