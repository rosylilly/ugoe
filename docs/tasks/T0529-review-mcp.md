---
id: T0529
title: コードレビュー: ugoe-mcp
status: done
milestone: M9
crates: [ugoe-mcp, docs]
depends_on: []
assignee: claude-worker-T0529
---

# T0529: コードレビュー: ugoe-mcp

## 目的

ugoe-mcp のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-mcp` のすべて (stdio と HTTP の transport・relay・host・tools・autorig)

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - HTTP の transport の認証・ループバック限定・Origin の検査、ルートの外のファイルを読めないか (T0515)
  - ツールの入力の検証、提案と適用の分離 (AI はモデルを直接書き換えない)
  - エディタとの relay (T0642) の切断・再接続・競合
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0551〜T0555 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### やったこと

`crates/ugoe-mcp` のすべて (`src/` 16 ファイルと `tests/` 3 ファイル、README) を読み、`docs/ai-integration.md#mcp-サーバ` と突き合わせた。
`cargo clippy -p ugoe-mcp -- -W clippy::pedantic` も流した (`must_use`・`# Errors`・`Option<&T>` などの書き方だけで、直すべきものは無かった)。

- **HTTP の守り (T0515):** 既定はループバックに束ね、ループバック以外はトークン必須。トークンは 128 ビット以上で定数時間の比較、
  トークンが無ければ `Host` がループバックの名前で転送ヘッダが無い要求だけ、`Origin` はループバックか許したものだけ (トークンが
  あっても確かめる)。上限 (本文・ヘッダ・時間・接続数) もある。`--root` は正規化してから判定し、`..`・絶対パス・外へのリンク・
  行き先の無いリンク・モジュールの本体を拒否する (TOCTOU は README に書いた非目標)。穴は見つからなかったが、`/mcpXY` を
  エンドポイントとして受けること、`Content-Length` と `chunked` の両方がある要求で接続を続けることを直した。
- **入力の検証と提案・適用の分離:** 引数はすべて `deny_unknown_fields` で読み、`propose_*` は複製に適用するだけ、`apply_proposal` は
  ID と `exclude` だけを受け、確認の前後で `preview` し直して古い提案を拒否する (ADR 0006・AI はモデルを直接書き換えない)。
  `--read-only` は呼び出しも断る。差分 (`changes`) が並べ替えと同じ大きさのテクスチャの差し替えを示さず、承認する人に
  「何も変わらない」と見えたのを直した。
- **エディタとの中継 (T0642):** 要求のたびに発見用のファイルを読み直し、つながらない・401 のときだけ (要求が届いていないと
  分かるときだけ) 読み直して 1 回送り直す。エディタの停止・再起動・古いファイル・知らない版・確認の間のモデルの変更は
  `tests/editor.rs` が確かめている。発見用のファイルの `endpoint` に `@` を入れると URL の利用者情報になり、トークンを
  ほかのホストへ送れたのと、WSL から Windows の共有プロファイル (`Public` など。ほかの利用者が書ける) を探していたのを直した。
  中継が要求を 1 つずつしか送らない (確認の間はほかの要求も止まる) のは T0551 に起票した。

### 所見

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 中 | `src/http.rs:265` | 接続のスレッドが panic すると同時の接続の数を戻さず、32 回で以後すべての接続が `503` になる | 直した (`ConnectionSlot` を捨てるときに戻す。`panics_in_tools_do_not_use_up_the_connections`) |
| 中 | `src/server.rs:195` | ツールや持ち主の panic で stdio のサーバ (唯一のスレッド) ごと落ち、未保存の変更を失う | 直した (`catch_unwind` で `-32603` を返す。`a_panicking_tool_answers_with_an_internal_error`) |
| 中 | `src/autorig.rs:90` | `get_psd_layers`・`import_psd` の `psd` に `/dev/zero` などを渡すと終わらずに読み、メモリを使い果たす (エディタのブリッジは既定で `--root` が無く、`get_psd_layers` は断らないので、エディタごと落とせる) | 直した (通常のファイルだけを読む。テストを追加) |
| 中 | `src/diff.rs` | 提案の `changes` が並べ替え (`reorder_mesh`・`reorder_physics_rig`) と同じ大きさの画像への差し替え (`set_texture_image`) を示さず、承認する人には空に見える | 直した (`reordered` と画像の指紋。テストを追加。README と ai-integration.md) |
| 中 | `src/tools.rs:447` | `--root` が無いとき、まだ無いパスを正規化しないので、シンボリックリンクの下 (macOS の `/tmp` など) に作ったプロジェクトは保存の後に鍵が変わり、開いている文書と提案が 2 つに分かれる | 直した (ある祖先を正規化する。`a_new_project_keeps_its_key_after_it_is_created`) |
| 中 | `src/discovery.rs:176` | 発見用のファイルの `endpoint` を検査せず、`@evil/…` で URL の利用者情報になり、中継がトークンと要求をほかのホストへ送る。`port` 0 も受ける | 直した (`/` で始まる URL のパスの文字だけ。テストを追加) |
| 中 | `src/wsl.rs:43` | 相互運用が無効な WSL で `Users/*` を探すとき、どの利用者も書ける `Public` などの共有プロファイルの発見用のファイルも読み、新しければそれにつなぐ | 直した (共有プロファイルを見ない。テストを追加)。ほかの利用者の個人のプロファイルは Windows の ACL で読めない前提 |
| 中 | `src/relay.rs:296` | 中継が要求を 1 つずつ送るので、確認を待つ適用の間はほかの要求と `notifications/cancelled` が届かない | T0551 を起票 |
| 中 | `src/host.rs:248` | `FileHost` が表全体の錠を握ったまま読み込み・保存・書き出しをするので、HTTP でほかのプロジェクトへの要求も待たされる。`save` は保存してから書き出すので、書き出しの失敗で半端になる | T0552 を起票 |
| 中 | `src/host.rs:316` | `save` が `.ugoe` の書き出し先を一時ファイルを経ずに `fs::write` で直接上書きするので、途中で失敗すると既存の `.ugoe` が壊れる (T0669 のレビューで指摘) | T0552 に含めた (一時ファイルに書いて名前を変える) |
| 低 | `src/autorig.rs:480` | レイヤーの無い PSD で `start` が大きいと `start + count` があふれる (debug では panic) | 直した (`saturating_add`。テストを追加) |
| 低 | `src/http.rs:662` | トークンがあると `/mcpXY` (`/mcp` で始まるだけのパス) をエンドポイントとして受ける | 直した (`404`。テストを追加) |
| 低 | `src/http.rs:522` | `Transfer-Encoding` と `Content-Length` の両方がある要求の後も接続を続ける (RFC 9112 6.1 は閉じることを求める)。`gzip, chunked` を `chunked` として読む | 直した (閉じる・`chunked` だけを受ける。テストを追加) |
| 低 | `tests/stdio.rs` | `apply_proposal` の `exclude` で一部を外して適用する成功の経路にテストが無い (範囲の外の番号の誤りだけ) | 直した (`apply_part_of_a_proposal`) |
| 低 | `src/relay.rs:42` | `import_psd` の `base` を絶対パス・Windows のパスに変えない | 見送り (エディタは `import_psd` を断るので届かない) |
| 低 | `src/http.rs` | セッションを持たないので、`Change::Proposed` の `client` は最後に握手したクライアントで、同時の複数のクライアントを区別できない | 見送り (README・host.rs に書いた設計。必要になったら `Mcp-Session-Id` を検討) |
| 低 | `src/bridge.rs` | エディタのブリッジは既定で `root` が無く、トークンを持つクライアントは利用者の権限で PSD やモーションの JSON を読める | 見送り (ADR 0023・README に書いた設計。装置を読む穴は上で直した) |
| 低 | `src/root.rs` | 判定と読み書きの間にリンクに差し替える競合 (TOCTOU) | 見送り (README・root.rs に書いた非目標) |
| 低 | `src/mcpb.rs:173` | `to_vec_pretty(...).unwrap_or_default()` で失敗を握りつぶす | 見送り (`Value` の文字列化は失敗しない) |
| 低 | `src/tools.rs`・`src/autorig.rs` | `render_png` と自動リギングのツールが呼ぶたびに文書を丸ごと複製する | 見送り (錠を短くするための設計。毎フレームの処理ではない) |

### 横断

- **接続ごとのスレッドの数え方:** `fetch_add` / `fetch_sub` で数え、スレッドの終わりで戻す形は、panic で戻らない。スタジオの
  API など、ほかの自前のサーバも RAII で戻しているか確かめるとよい。
- **長く動くサーバでの panic:** stdio のように 1 つのスレッドで要求を処理するサーバは、1 つの不具合で状態ごと失う。境界
  (要求の処理) で `catch_unwind` するか、ほかのアプリ (スタジオ・エディタの IPC) でも方針を決めるとよい。
- **外から渡されたパスを `fs::read` する箇所:** 装置・FIFO を読むと終わらない。PSD・モーション・物理の JSON など、利用者や
  エージェントが渡すパスを読む箇所 (CLI・エディタの IPC・ugoe-headless) は、通常のファイルかと大きさを確かめるとよい。
- **差分・要約の「並び」:** ID の集合だけで比べると並べ替えを見落とす。エディタの AI タブの差分の表示 (提案の差分) も同じ形か
  確かめるとよい。
- **まだ無いパスの正規化:** `canonicalize().unwrap_or(path)` は、無いパスで鍵が変わる。パスを鍵にする箇所 (エディタの
  開いているプロジェクト・最近のファイル) で同じ形が無いか確かめるとよい。

