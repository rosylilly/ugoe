# 0026: Godot のバインディング (godot-rust) をワークスペースの外に置く

- 状態: Accepted
- 日付: 2026-10-01
- 関連: T0406、T0411、ADR 0017 (単一ランタイムの配布)、ADR 0018 (ライセンス)、[portability.md](../portability.md)

## 背景

Godot 4 向けの配布物 (ADR 0017) は GDExtension で作る。Rust から GDExtension を作る標準的な方法は
[godot-rust (gdext)](https://github.com/godot-rust/gdext) で、`ugoe` クレートをそのまま使える (C ABI を経由しなくてよい)。
ただし godot-rust のクレート (`godot`・`godot-core` など) は **MPL-2.0** である。

ADR 0018 は「MIT のクレートは寛容なライセンスの依存だけを持つ」と決め、`scripts/check-licenses.mjs` (`just licenses`) が
ルートのワークスペースのすべてのクレートを検査している。godot-rust に依存するクレートをワークスペースに入れると、この検査に落ちる。

利用者の決定 (2026-10-01): Godot のバインディングは `crates/` の外の独立したクレートにし、バインディング自体は MIT のまま
godot-rust に依存してよい。MPL-2.0 はファイル単位のコピーレフトなので、MPL のファイルを変えずに使う限り、
このクレートのコードと `ugoe` のクレートは MIT のまま配れる。

## 決定

1. **`bindings/godot` を、ルートのワークスペースに入らない独立したクレート (`ugoe-godot`、MIT) にする。**
   - `bindings/godot/Cargo.toml` に空の `[workspace]` を書き、独自の `Cargo.lock` を持つ。ルートの `Cargo.toml` の `members` には入れない
     (`crates/*` と `apps/*/src-tauri` だけなので、`exclude` も要らない)。
   - `ugoe` にはパスで依存する (`ugoe = { path = "../../crates/ugoe" }`)。依存の向きは「バインディング → `ugoe`」だけで、
     `crates/` のクレートは godot-rust に依存しない。
   - ルートの `cargo metadata` に現れないので、`just licenses`・`just layers`・`just ci` の対象にならない (今の検査はそのまま通る)。
2. **バインディングの依存のライセンスは `scripts/check-licenses.mjs --godot` で別に検査する** (T0411 で、T0406 の
   `bindings/godot/check-licenses.mjs` をこの小さな分岐にまとめた。SPDX の式の読み方と寛容なライセンスの集合を 1 か所に保つため)。
   許すのは寛容なライセンス (ルートの検査と同じ集合) と、godot-rust のクレート (`godot`・`godot-*`・`gdextension-api`)
   に限った MPL-2.0。ほかのコピーレフトの依存が入ったら落ちる。`--godot` を付けたときだけ bindings/godot の
   `cargo metadata` を読むので、`just licenses` (と `just ci`) の検査と所要時間は変わらない。
   `node bindings/godot/build.mjs --check` (`just godot-check`) がこれを実行する。
3. **`ugoe-godot` にはワークスペースと同じ lint を書き写す** (`[workspace.lints]` を継承できないため)。`unsafe` の禁止も同じだが、
   godot-rust の入口 (`#[gdextension] unsafe impl ExtensionLibrary`) だけは `#[allow(unsafe_code)]` で許す。中で unsafe な操作はしない。
4. **描画は Godot の 2D (RenderingServer のキャンバスアイテムとシェーダ) で行う** (portability.md の段階 B)。
   評価は `ugoe::Puppet` が行い、描画の式は `docs/spec/rendering.md` に従う (ライティングも参照レンダラと同じ式)。
   合成モードは Godot の `blend_premul_alpha` と `blend_mul` で表す (詳しくは `bindings/godot/src/shader.rs`)。
   `ugoe render` との比べ合わせ (`build.mjs --conformance`) で、互換の基準を満たすことを確かめる。
   - マスクのカバレッジは 1 枚のアトラス (`SubViewport`) に並べて描き、要素は浮動小数点のテクスチャの表から、読むマスクと
     アトラスでの場所を `texelFetch` で読む (T0411。T0406 の「マスク用メッシュごとの `SubViewport` と、要素ごとに 8 つまでの
     サンプラ」をやめ、読めるマスクの数の上限をなくした)。
   - 入れ子のマスクは段に分け、段 `k − 1` のアトラスを段 `k` のビューポートの **子** にする。Godot は子のビューポートを先に
     描くので、アニメーション中も入れ子のマスクは同じフレームのうちに描かれる (1 フレーム遅れない。`moving-masks` と
     `avatar-moving` の比べ合わせで、値を変えた直後のフレームを確かめる)。
5. **`.ugoe` と `.ugoemod` を Godot のリソースとして読む部分 (`ResourceFormatLoader` と `UgoeModel`・`UgoeModule`) は
   GDScript で書く** (`example/addons/ugoe/`。T0411)。Godot は読み込み器とリソースの生成をワーカースレッド
   (エディタのファイルシステムの走査・スレッドでの読み込み) からも呼ぶが、godot-rust のクラスはメインスレッド以外から呼ぶと
   パニックする (`experimental-threads` の機能を使えば呼べるが、godot-rust 自身が「健全性の保証がまだない」としている)。
   リソースはバイト列 (`data`) を持つだけで、中身は Rust のノードがメインスレッドで読むときに検査する。
## 検討した代替案

| 案 | 利点 | 欠点 |
| --- | --- | --- |
| ワークスペースに入れ、`scripts/check-licenses.mjs` で例外にする | ビルドと検査が 1 つにまとまる | ルートの `Cargo.lock` と `cargo build` (全クレート) に MPL のクレートと Godot の API の生成が入り、`just ci` が重くなる。MIT のクレートに MPL が紛れ込む事故を検査で区別しにくくなる |
| C ABI (`ugoe-ffi`) と Godot の C++ (godot-cpp) で作る | godot-cpp は MIT | C++ のビルド系 (SCons) が別に要る。`ugoe` の API を C ABI に出す分だけ遅れる (着せ替え・ライティングなどは未公開) |
| `ugoe-ffi` を GDScript から直接呼ぶ | 追加の依存がない | GDScript から C の関数は呼べない (GDExtension が要る) |
| リソースの読み込み器を godot-rust で書き、`experimental-threads` を有効にする | すべて Rust で書ける | godot-rust がまだ健全でないとしている機能に、ノードを含むすべてのクラスが頼ることになる |
| 依存のライセンスの検査を `bindings/godot/check-licenses.mjs` のまま別に持つ (T0406) | ルートの検査に触らない | SPDX の式の読み方と寛容なライセンスの集合が 2 か所に分かれ、食い違いうる |

## 結果

- `crates/` と `just ci` は変わらない。Godot のバインディングのビルド・検査は `node bindings/godot/build.mjs`
  (`just godot-build` / `just godot-check`) で別に行う (`--check` でライセンスとヘッドレスの Godot での動作、`--conformance` で
  描画の比べ合わせ)。CI では別のジョブ `godot` で、版を固定した Godot (公式のビルドを SHA-512 で確かめる) と Xvfb (ubuntu-24.04)
  の上で `just godot-check` を実行する (T0411)。
- ルートの `target/` を共有すると `ugoe` のクレートのビルド済みの成果物を使い回せるので、`build.mjs` は既定でルートの `target/` を使う。
  そのため `bindings/godot/Cargo.toml` の `[profile]` はルートと揃える。
- 独立した `Cargo.lock` は、ルートと同じ版を保つように、ルートの `Cargo.lock` を写したものから始めた。ルートの依存を上げたら、
  `bindings/godot` でも `cargo update` が要ることがある (`--locked` のビルドが落ちて気づける)。
- godot-rust は 0.x で、版を上げると API が変わる。Godot の API の最低の版は `api-4-4` (Godot 4.4 以降) にしている。
