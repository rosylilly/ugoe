---
id: T0032
title: M9 の後続のタスクの「起票の候補」をまとめて起票する (3 回目)
status: done
milestone: M9
crates: [docs]
depends_on: []
assignee: claude-worker-T0032
---

# T0032: M9 の後続のタスクの「起票の候補」をまとめて起票する (3 回目)

## 目的

M9 の後続のタスクを並列で進めたワーカーは、新しいタスクを起票せずに「起票の候補」をメモに書いた (番号の衝突を避けるため)。
T0026 の後に集めた候補の一覧を読み、重複をまとめ、既存のタスクに含まれるものは除いて、タスクとして起票する。

## 受け入れ条件

- [x] オーケストレーターの一覧 (下の「参考」) と、T0026 の後に done になった M9 のタスクのメモの「起票の候補」を読んだ
- [x] 同じことを言っている候補を 1 つのタスクにまとめ、既存のタスク (todo のもの) に含まれる候補はそのタスクのメモに書き足して起票しない
- [x] 残りを docs/tasks/README.md の形でタスクにした (milestone は M9。受け入れ条件は検証できる形。depends_on を正しく。`crates:` は狭く)
- [x] 公開 API の変更や仕様の判断が要るものは、タスクの本文に「判断が要る」と書いた
- [x] 番号: docs/tasks/README.md の番号帯の規則に従う (エディタは T1618〜、ほかは空いている番号)
- [x] 起票したタスクの一覧 (ID・題名・元の候補) をメモに書いた
- [x] `node scripts/tasks.mjs` が通る (未知の依存が無い)
- [x] `just ci` が通る

## 参考

- オーケストレーターの集計: /tmp/claude-1000/-home-rosylilly-src-github-com-rosylilly-ugoe/026b491e-8c79-47ed-ac14-04f23c5addc0/scratchpad/run2/candidates-for-T0032.md
- T0019・T0026 (1 回目・2 回目)
- docs/tasks/README.md、docs/workflow/multi-agent.md#タスクの書き方-オーケストレーター向け

## メモ

- T0026 (2 回目) の後に、M9 の後続のタスクのワーカーが出した候補。

### やったこと

- オーケストレーターの一覧 (`candidates-for-T0032.md`、48 行) と、T0026 の後に done になった M9 のタスク (T0010・T0021・T0025・T0027・T0028・T0030・T0031・
  T0159・T0164〜T0167・T0169・T0170・T0223・T0224・T0327・T0457〜T0460・T0463・T0554・T0556・T0557・T0559〜T0561・T0732・T0836〜T0838・T0915・
  T0935・T0937・T1607〜T1609・T1611・T1617) のメモの「起票の候補」を読んだ。一覧に無い候補は無かった (T0554・T0560・T0010・T0021 の候補はすでに起票・完了済み)。
- 候補ごとに今のコードを見て、残っていることを確かめた (例: `tests/conformance/run.mjs` は `cargo` を直に呼ぶ・`ugoe-mcp` の `PsdSource::load_with` は
  `std::fs::read`・spec-check の `PoseState.frame` は `Math.max(NaN, 0)` で NaN になる・エディタの 3 つのファイルが `localStorage` を直に使う)。
- 番号は各帯の最大の番号の次から付けた (T0019・T0026 と同じ)。エディタは T1618〜T1622。
- 並行中のタスク (T0029・T0168・T0461・T0839・T0934・T0936・T1610) のファイルは触っていない。それらと同じファイルを触る新しいタスクには、衝突を避ける依存を付けた
  (T0465→T0934、T0841→T0839・T0840、T0938→T0936、T1618→T0168、T1619→T1610、T1620→T1616、T0466→T0464)。

### 起票したタスク

| ID | 題名 | 元の候補 |
| --- | --- | --- |
| T0033 | cargo doc の出力先の衝突・wasm32 の cargo doc・cross-check の --all-features | T0027 の 3 つ |
| T0034 | just を通らずに cargo を呼ぶスクリプトも mbx を通す | T0028 |
| T0035 | ugoe-headless の read_file を ugoe-fs に寄せるか決める (判断が要る) | T0025 (ugoe-headless) |
| T0171 | T0159 の範囲の外の Id の as_str() == を置き換える | T0159 |
| T0172 | spec-check のポーズが有限でない dt で不透明度を NaN にしない | T0164 (ポーズの dt) |
| T0464 | Unity: 読み込みのエラーの説明とテスト、補足の値のキーを C ABI から列挙するか (判断が要る) | T0457 (Unity)・T0458 (2 つ) |
| T0465 | wasm の GPU の喪失のコード (判断が要る) と @ugoe/player の着せ替えの device_lost | T0457 (player・wasm) |
| T0466 | Unity: KTX2 を C ABI のデコーダで読む (判断が要る) | T0223 (Unity) |
| T0467 | Godot: bench.gd・Forward+ の適合テスト・BC1 の 3 色の形 | T0460 (2 つ)・T0223 (Godot) |
| T0468 | 見本のモデルにパラメータのグループとポーズを持たせるか | T0463 (見本) |
| T0562 | ugoe-mcp: 503 も RST で失われないようにする | T0556 |
| T0563 | ugoe-mcp: HostError::Failed とコードの無いツールのエラーのコード (判断が要る) | T0557 (2 つ) |
| T0564 | ugoe-mcp: 自動リギングの PSD を上限付きで読む | T0025 (ugoe-mcp) |
| T0565 | Rig::pendulum の引数と Layout の添字の計算 | T0561 (2 つ) |
| T0733 | ビューア: Windows・Linux の single-instance | T0732 (single-instance) |
| T0840 | スタジオ: 受け手ごとに読めなかったものをすべて知らせる | T0836 |
| T0841 | スタジオ: ホットキー・API の切り替えの読み込み中と、モジュールの合成をロックの外へ | T0837 (2 つ) |
| T0938 | WebGL2 の復元の時間切れと作り直すボタン (判断が要る) | T0935 (2 つ) |
| T1618 | editor-core: Coded::code を元のコードから組み立てる | T0167 |
| T1619 | エディタ: localStorage を readStoredItem などに寄せる | T0937 |
| T1620 | エディタ: 「名前を付けて保存」もロックの外で書く | T0025 (save_project_as) |
| T1621 | editor-core: keep_look で名前も変えたときの check_reparented_pairs | T1609 (keep_look) |
| T1622 | エディタ: 本体を直した後にモジュールに戻る道 | T1609 (戻る道) |

### 既存のタスクに含めたもの (起票しない)

- T0918 (todo、M10) のメモに書き足した: T0915 の `just ui-audit` の forced-colors の組と、フォーカスの印を画像の差で判定するかの検討。
- T0676・T0818 (todo、M10) のメモに書き足した: T0915 の偽のホストの状態 (AI の提案・モジュール・エラーの知らせ) を、各タスクの範囲の画面のぶん足す。
- T0421 (todo、Release) のメモに書き足した: T0166 の C ABI の共有ライブラリの大きさをリリースビルドで測る。
- T1616 (todo) のメモに書き足した: T0025 の `write_png_file` のビューアとエディタの共有 (場所の判断を含む)。
- T0731 (todo、手動確認) のメモに書き足した: T0732 の .deb の MIME の定義 (V08 で関連付けが効かなければ起票する)。
- T0673 には T0026 で T0926 の `third-party-notices.txt` の候補を書き足し済み (今回の一覧の同じ候補は重複)。
- **このタスクで行った小さな変更 (`crates:` の外):** T0915 の「AGENTS.md のコマンドの表に `just ui-audit`」を 1 行足し、T0463 の
  「crates/ugoe-wasm/README.md に `MotionSampler`」を README の例に 3 行足した。

### 起票しないもの (すでに済んでいる・見送り)

- T0926 の NPM_EXCEPTIONS と LICENSE_OVERRIDES を 1 か所に: T0031 で済んだ。
- T1607 の「editor-core の project/read.rs を ugoe-fs に」: T1607・T0025 で済んだ (`ugoe_fs::read_regular_file`)。
- T1607 の「エディタ・MCP で読み込みの上限を設定として持つか」: 候補自身が「必要になったら」とし、T0554 の利用者の判断 (エディタに PSD の上限の設定は出さない) と
  同じ方針になるので見送り。要望が出たら起票する。
- T0560 の PSD の上限を超えるレイヤー: T1617 で済んだ。T0554 の 2 つ: T0557・T0558 に入っている。T0010 の 3 つ: T0165・T0461・T0026 で済んだ。T0021: T0030 で済んだ。

### 検査

- `git rebase main` の後に `node scripts/tasks.mjs` (未知の依存なし・番号の重複なし) と `just ci` が通った。
