---
id: T0146
title: ID から添字への引き当てを毎フレームの線形探索にしない
status: done
milestone: M9
crates: [ugoe-runtime, ugoe, ugoe-ffi, docs]
depends_on: [T0137]
assignee: claude-worker-T0146
---

# T0146: ID から添字への引き当てを毎フレームの線形探索にしない

## 目的

T0137 (ugoe-runtime のコードレビュー) の性能の所見。

`ParameterValues::index_of` と `CompiledModel::part_index`・`deformer_index`・`mesh_index`・`hit_area_index` は、ID を先頭から比べる線形探索。
ロード時や問い合わせで使うぶんには問題ないが、毎フレームの経路でも呼ばれている。

- `ugoe::Puppet::apply_part_opacities` (`crates/ugoe/src/lib.rs`): モーションのパーツの不透明度のカーブごとに `part_index` を呼ぶ (カーブ数 × パーツ数の文字列比較)
- C ABI の ID によるパラメータの一括設定 (`crates/ugoe-ffi/src/puppet.rs`): 値ごとに `index_of` を呼ぶ (値の数 × パラメータ数)

ランタイムの目標 (1 体 1ms、roadmap.md の M7) に対して、パラメータとパーツが数百あるモデルで無視できない可能性がある。
まず中規模のモデル (`crates/ugoe-runtime/examples/compose_timing.rs` の規模) で毎フレームの費用を測り、効くなら次のどちらか (または両方) にする。

- `CompiledModel` に ID → 添字の表をコンパイル時に作り、`*_index` を定数時間にする (`compile` は既に ID の表を作っているので、それを持ち続ける)。
  `ParameterValues` は複製される型なので、表は `CompiledModel` 側に置き、`Arc` で共有するなどして複製の費用を増やさない
- 毎フレームの呼び出し側で、添字の解決をロード時 (モーションの開始時など) に前倒しする

## 受け入れ条件

- [x] 変更の前後で、ID による引き当てを含む 1 フレームの費用を測り、結果をメモに書く
- [x] 効果があれば、公開 API を変えずに引き当てを定数時間にするか、毎フレームの呼び出しを前倒しする (テストを足す)
- [x] `ModelInstance::update` の毎フレームの確保がないこと (既存のテスト) を保つ
- [x] `just ci` と `just wasm-check` が通る

## 参考

- docs/tasks/T0137-review-runtime.md (メモの所見の表)
- docs/roadmap.md (M7 の性能の目標)
- docs/spec/evaluation.md#デフォーマのワールド変換を読む (ID からの添字の引き当て)

## メモ

### やったこと

- 計測用の例 `crates/ugoe-runtime/examples/id_lookup_timing.rs` を足した (`cargo run --release -p ugoe-runtime --example id_lookup_timing`)。
  毎フレームの呼び出し側 (Puppet のパーツの不透明度のカーブ、C ABI の ID による一括設定) を模して、すべてのパーツを `part_index`、すべてのパラメータを `index_of` で引いて書き込み、`update` で評価する。
  ID は `ParamHairFrontSway123` のような実際に近い長さ。メッシュは 4 頂点の四角形なので `update` は軽い (引き当ての割合が大きく出る条件)。
- 結果 (WSL2、release、101 回の中央値。2 回ずつ流した範囲):

  | 規模 | 引き当て (変更前) | 引き当て (変更後) | update |
  | --- | --- | --- | --- |
  | パラメータ 20・パーツ 20・メッシュ 20 | 0.0010–0.0012 ms | 0.0005–0.0006 ms | 0.001 ms |
  | パラメータ 300・パーツ 300・メッシュ 200 | 0.22–0.26 ms | 0.009–0.010 ms | 0.009–0.019 ms |

  数百の規模では 1 フレームの目標 (1ms) の 2 割を引き当てだけで使っていたので、効果があると判断した。小さいモデルでも遅くならない。
- `CompiledModel` にコンパイル時に作る ID → 添字の表 (`id_index::IdIndex`、`HashMap<Box<str>, u32>`) を持たせ、`part_index`・`deformer_index`・`mesh_index`・`hit_area_index` を定数時間にした。重複した ID は先頭の添字 (線形探索と同じ)。
- `ParameterValues` の ID の列と表を `Arc` で共有する形にした (`Layout`)。`clone`・`clone_from` は ID を複製しない (前は `clone` が ID の文字列をすべて複製していた)。
  `CompiledModel` が既定値の `ParameterValues` を持ち、`ModelInstance::new` はそれを複製するので、同じコンパイル済みモデルのインスタンスは表を共有する。`carry_over` は一時的な `HashMap` を作らずに `index_of` を使う。
- 公開 API は変えていない。呼び出し側 (`ugoe`・`ugoe-ffi`) は変更不要 (同じ API がそのまま定数時間になる)。
- テスト: 4 種の引き当てがモデルの並びと一致すること (当たり判定を足したモデルを含む)、`index_of` の一致と複製で ID を共有すること、インスタンス間で共有すること、重複 ID は先頭。既存の確保なしのテスト (`update_does_not_reallocate_buffers` ほか) はそのまま通る。
- 文書: `docs/spec/evaluation.md#デフォーマのワールド変換を読む` に引き当てが定数時間であることを、`crates/ugoe-runtime/README.md` の「守ること」に追記。英語の公開仕様にはこの API の記述がないので変更なし。
- `just ci`・`just wasm-check` は通った。

### やらなかったこと

- 呼び出し側での添字の前倒し (モーション開始時の解決など) はしなかった。引き当てが定数時間になったので、`ugoe` の `apply_part_opacities` はカーブ数に比例する費用だけになった (毎フレーム `MotionManager::part_opacities` が Vec を作るのは ugoe-motion 側で、範囲外)。
- T0319 からの候補のうち「ugoe-motion の MotionPlayer/Expression が最初の apply で解決した添字をその後確かめない」は ugoe-motion が範囲外なので触っていない。

### 起票の候補

- ugoe-motion: `MotionPlayer`・`Expression` が解決した添字を、パラメータの並びが変わったときに解決し直す (T0319 の候補)。
- `ParameterValues` の並びの変化を O(1) で見分ける公開 API: 内部で ID の列を `Arc` で共有するようになったので、例えば並びの識別子 (`Arc` のポインタ比較で同じかを返す `same_layout(&self, other)` や、共有の手札を返す `layout()`) を足せば、ugoe-physics の `resolve_parameters` が毎フレーム ID の列を文字列で比べている箇所 (`resolved_ids == parameters.ids()`) を定数時間にできる。公開 API の追加になるので別タスクで。
- `Puppet::apply_part_opacities` のポーズの判定 (`PosePlayer::contains`) とカーブの `Vec` の毎フレームの確保 (ugoe-motion)。

