---
id: T0129
title: 公開仕様: 独立の読み込み器の型検査を CI に入れる・差し替えたパーツを足す順を決める
status: done
milestone: M5
crates: [spec, tools/spec-check, docs, justfile, package.json, pnpm-workspace.yaml, pnpm-lock.yaml, .github/workflows/ci.yml]
depends_on: [T0120]
assignee: claude-worker-T0129
---

# T0129: 公開仕様: 独立の読み込み器の小さな残り

## 目的

T0120 の後続の候補 (小さなもの 2 つ)。

## 受け入れ条件

- [x] `tools/spec-check` の TypeScript の型検査 (tsc) を `just ci` で走らせる (今は手で 1 回通しただけ)。依存を増やすなら理由をメモに書く
- [x] 複数のスロットがパーツを差し替えるとき、隠した `#replaced/...` パーツを足す順を `spec/model.md` の Composition に書く (実行時の結果は変わらないが、合成の結果を比べるときに順が要る)。参照実装の順は team-lead に確かめてもらう。`docs/spec/` の同期は team-lead に頼む
- [x] 最初のセグメントの規則を破るモーションのカーブを、spec/model.md は「無視してよい (MAY)」、spec/motion.md は「値を持たない」としていて強さが食い違う。参照実装の振る舞いを team-lead に確かめてもらい、揃える (T0126 の (7))
- [x] 表情をすべて止めるホストの操作が仕様にない。参照実装の API を team-lead に確かめてもらい、spec/motion.md に書く (T0126 の (8))
- [x] `Parameter.mirror` は「作るときのためだけ」とあるが Authoring data の節に無く、ツールが落としてよいか不明。参照実装の扱いを team-lead に確かめてもらい、spec/model.md で揃える (T0128 の (3))
- [x] `.ugoe` のファイルの例 (spec/examples/bundles) を、時間の例としても走らせるかを決める (T0128 の後続の候補)
- [ ] `just ci` が通る

## 参考

- `docs/tasks/T0120-spec-independent-reader.md` のメモ

## メモ

### 独立性を保った手順

- 作業者 (claude-worker-T0129) は `crates/` と `docs/spec/` を読まず、`spec/`・`tools/spec-check/`・ワークフローの文書だけを参照した。
  参照実装の振る舞い (下の 4 点) は team-lead が調べて言葉で伝えた。`just ci` の失敗は、テスト名とパニックの 1 行だけを見た。

### やったこと

- 型検査: `tools/spec-check` を pnpm のワークスペースに入れ (`pnpm-workspace.yaml` に `tools/*`)、開発用の依存に `typescript` と
  `@types/node` (エディタ・ビューアと同じ版。`pnpm install --offline` で入る) を足し、`check` (`tsc -p .`) を `just spec` から呼ぶ。
  GitHub の CI (`web` のジョブ) はこれまで spec-check のテストを走らせていなかったので、型検査とテストの 2 行を足した。
  実行時の依存は無いまま。範囲外の `pnpm-workspace.yaml`・`pnpm-lock.yaml`・`.github/workflows/ci.yml` は team-lead の承認を得て crates: に足した。
- 隠すパーツの順 (spec/model.md Composition の手順 3): 参照実装どおり「モジュールの順、スロットの中は replaces の順、同じパーツは最初の 1 回だけ、
  モジュールのパーツの後ろ」にした。名前は `#replaced/<元のパーツの name>`、キーフォームは 1 つで束縛なし (不透明度 0) と明記。
  ホストがモジュールの順を決める規則 (末尾に足す・同じスロットか同じ名前なら置き換える) は informative。spec-check はこれまで「モデルのスロットの順」
  で、名前を ID から作っていたので直し、テストを足した。
- 規則を破るカーブ: 空・知らない種類を含む・最初が `start` でない → 値なし (これまでどおり。参照実装と一致)。2 つ目以降に `start` がある
  カーブは、仕様は値を定めず「全体を無視してよい (MAY)」、適合の対象外とし、参照実装の振る舞い (その `start` の前まで値を持ち、後は値なし) を
  informative で書いた。model.md の言い方も合わせた。理由: 書き手の MUST を破ったデータの結果を規定すると参照実装のたまたまの挙動を全実装に強いる。
  また「最後の点の後は最後の値を保つ」規則とも食い違う。spec-check は全体を無視する (MAY の側)。参照実装を変える必要はない。
- 表情を止める: spec/motion.md の表情の節に、止める・終わるの定義 (モーションと同じ `end = min(end, elapsed + max(fade_out, 0))`) と、
  ホストの操作の表 (足す・すべて止める・名前で止める・set = すべて止めてから足す) を書いた。時間の例の操作に `{ "stop_expressions": true }` を足し
  (spec/examples/README.md)、`expressions` の例に 6 フレームを足した (fade_out 0 の表情がすぐ消える・フェードアウト・フェードインの途中で止める・
  止め直しで早い終わりを保つ)。期待値は手で導いた (w = 0.5、0.25 = 0.5 × 0.5、0.125 = (1 − cos²(π/4))/4 など)。名前で止める操作は時間の例に入れていない。
- `Parameter.mirror`: 表の説明を「Authoring data」にし、Mirror pairs の節にパラメータの対の意味 (相手の同じキー / 自分自身なら符号を反転したキー) と、
  双方向を SHOULD (メッシュ・デフォーマと違い検証しない。参照実装の検証が検査しないため) を書いた。Authoring data の節の「既定値なら省くべき」は、
  nullable なものは `null` を書いても省いてもよい (ほかのフィールドと同じ) と書き直した (参照実装は `null` を書く。仕様側を直したので後続は不要)。
  Authoring data の節の対象を「表で Authoring data と書いたフィールド」と明記し、「評価・再生・物理・描画に影響しない」とした。
- ファイルの例を時間の例として走らせる: `spec/` には足さず、spec-check のテスト (`test/timeline.test.ts`) で、時間の例と同じ中身のファイルの例
  (`physics-pendulum.ugoe`・`motion-crossfade.ugoe`・`expressions.ugoe`) を、バンドルから読んだデータで同じフレームに通すようにした
  (f32 に丸めた値でも許容誤差に収まる)。中身が JSON と同じことは bundles.json の試験ですでに確かめているので、仕様の例としては増やさない。

### just ci

- `crates/ugoe/tests/spec_timelines.rs` の `puppet_meets_the_expected_values_of_every_timeline_example` だけが失敗する:
  `expressions/timeline.json: data did not match any variant of untagged enum Action at line 157 column 7`。
  参照実装の側の照合テストが新しい `stop_expressions` の操作を知らないため (crates/ は範囲外。team-lead が拡張すると連絡済み)。
  `cargo test --workspace --no-fail-fast` で失敗はこれ 1 つ。fmt-check・clippy・layers・licenses・spec・web-check は通った。

### docs/spec の同期が必要な箇所 (team-lead に依頼)

- spec/model.md: Parameters の表の `mirror`、Mirror pairs (パラメータの対)、Authoring data の冒頭、Motions のカーブの規則、Composition の手順 3 と informative。
- spec/motion.md: Curves (規則 1 と規則を破るカーブの段落)、Expressions (止める・ホストの操作)、Determinism、Conformance。
- spec/examples/README.md: 時間の例の操作 `stop_expressions`。

### 確かめていないこと

- 参照実装が新しい `expressions` の時間の例のフレームに一致するか (照合テストの拡張の後)。
- 名前で止める操作の例 (時間の例の操作にしていない)。

### 後続の候補

- 時間の例の操作に「名前で表情を止める」を足す (参照実装の ffi と Puppet に専用の操作が無いため、足すなら API を揃えてから)。
- spec-check の書き出しは、`explicit` の書き方で `Parameter.mirror` の値なしを `null` と書く (ほかの作るときのデータは常に省く)。
  仕様はどちらも許すので直していない。揃えるならファイルの例の作り直しが要る。
