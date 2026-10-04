---
id: T0527
title: コードレビュー: ugoe-ai (プロバイダと通信)
status: done
milestone: M9
crates: [ugoe-ai, docs]
depends_on: []
assignee: claude-worker-T0527
---

# T0527: コードレビュー: ugoe-ai (プロバイダと通信)

## 目的

ugoe-ai (プロバイダと通信) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-ai` のプロバイダ抽象・Claude と OpenAI 互換のプロバイダ・transport・turn (自動リギングの本体は T0528)

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - API キーなどの秘密がログ・エラー・デバッグ出力に出ないか
  - タイムアウト・再試行・キャンセル、応答の形が想定と違うときの扱い
  - AI が `Command` の列を提案するだけで、モデルを直接書き換えていないか
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0541〜T0545 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

範囲: `lib.rs`・`fake.rs`・`claude.rs`・`openai.rs`・`transport.rs`・`depth.rs`・`turn.rs` と、`tests/claude_replay.rs`・`tests/openai_replay.rs`・`tests/depth_turn.rs`・`tests/common/mod.rs`・録画。

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 高 | `src/transport.rs` (`UreqTransport::new`) | ureq は転送 (3xx) に従い、転送のときに `authorization` しか外さない。Claude の `x-api-key` が転送先の別のホストへ送られる | 直した (`max_redirects(0)`。3xx は `AiError::Api`。ループバックのサーバで確かめるテストを追加) |
| 高 | `src/transport.rs` (`retry_after`) | `retry-after: 1e30` のような値で `Duration::from_secs_f64` がパニックする (サーバの応答で止まる) | 直した (`try_from_secs_f64`、収まらなければ `Duration::MAX` で諦める。テストを追加) |
| 中 | `src/transport.rs` (`UreqTransport::send`) | 応答の本文を ureq の既定の上限 10 MB までしか読まない。深度マップの JSON (2048² で数十 MB) が「Other」の失敗になる | 直した (上限を 256 MiB に。12 MB の応答を読むテストを追加) |
| 中 | `src/turn.rs` (`in_triangle`) | 1 点につぶれた三角形・NaN を含む三角形が、どの点も覆うと判定される。最も手前の部位を取り違え、制御点の深度とラベルが誤る | 直した (面積が 0 か非有限なら覆わない。テストを追加) |
| 中 | `src/claude.rs` (`status_error`) | JSON でないエラーの本文 (プロキシの HTML など) を長さの制限なしにエラーの文言に入れる (OpenAI 互換と深度は 500 文字で切る) | 直した (共通の `transport::error_excerpt` で 500 文字に。テストを追加) |
| 中 | `src/claude.rs` (`complete_json`) | `stop_reason: model_context_window_exceeded` を切れた答えとして扱わず、「JSON でない」という分かりにくいエラーになる | 直した (`AiError::Truncated`。テストを追加。`docs/ai-integration.md` を更新) |
| 中 | `src/lib.rs`・`claude.rs`・`openai.rs`・`depth.rs` | 取り消しの手段がなく、1 回の提案の時間の合計に上限がない (600 秒 × 再試行 × 問い直し) | T0541 を起票 |
| 中 | `src/depth.rs` (`HttpDepthEstimator`) | 再試行しない・認証のヘッダを送れない・通信の失敗の種類を落とす (プロバイダと規則が違う) | T0542 を起票 |
| 低 | `src/transport.rs` (`RecordedRequest`・`HttpRequest` の `Debug`) | URL の利用者の部分 (`user:pass@`) が録画と `Debug` に残る | 直した (`<redacted>` に置き換える。テストを追加) |
| 低 | `src/depth.rs` (`DepthMap::sample`) | フィールドを直接埋めた空の深度マップで `n - 1` があふれる (debug でパニック) | 直した (空なら 0。テストを追加) |
| 低 | `src/turn.rs` (`proposal_from_plans`) | 体だけを振り向かせる提案も題が「Head turn」 | 直した (`proposal_title`。テストを追加) |
| 低 | `src/claude.rs`・`openai.rs`・`depth.rs` | `PNG_SIGNATURE` とエラーの本文の切り詰めが 3 か所で重複 | 直した (`crate::PNG_SIGNATURE`・`transport::error_excerpt`) |
| 低 | `src/claude.rs` (`ApiKey`) | 共通の `ApiKey` が `claude` モジュールにある | T0543 を起票 (公開 API) |
| 低 | `src/lib.rs` (`AiError::RateLimited`) | 待ち時間の表示が `as_secs()` の切り捨てで 0.5 秒が「0s」になる (エディタも同じ) | T0543 を起票 |
| 低 | `src/lib.rs` (`AiError`) | `Provider`・`InvalidProposal` に doc コメントがない | T0543 を起票 |
| 低 | `src/transport.rs` (`is_loopback_url`) | `localhost.`・`[::ffff:127.0.0.1]`・`[0:0:0:0:0:0:0:1]` をループバックと見なさない | 見送り (外部として扱う安全側の誤り。`\` を含む URL を手元と見なすが、ureq が不正な URI として拒むので送られないことを確かめた) |
| 低 | `src/turn.rs` (`new_cells`) | 軸のキーが空の束縛で `binding.keys[...]` が範囲の外になる | 見送り (キーの空の束縛は `ugoe_model::validate` が拒み、`Builder` は検証を通したドキュメントだけを扱う) |
| 低 | `src/openai.rs` (`check_type`) | `integer` に `1e300` のような巨大な浮動小数点を通す | 見送り (下流の型への変換と `validate` で拒まれる) |

秘密 (受け入れ条件の追加の観点): `ApiKey`・`HttpRequest`・`ReplayTransport` の `Debug`、エラーの文言 (`redact`)、録画 (ヘッダを残さない・画像を省く) はキーを出さないことをテストで確かめてある。転送 (上の 1 行目) と URL の利用者の部分を直した。
タイムアウトと再試行: 1 回のリクエストの上限 (ureq の `timeout_global`)、408・409・429・5xx・接続の失敗・タイムアウトの再試行、`retry-after` の上限はテストがある。応答の形が違うとき (`choices` がない・`content` が配列・JSON でない・スキーマに合わない) はエラーか問い直しになる。
提案だけ: `turn.rs` は `Document::detached` の複製に `Command` を適用して確かめ、`Proposal` を返すだけで、呼び出し側のモデルを変えない。

### 横断

- **ureq の転送と秘密のヘッダ:** ureq 3 は転送で `Authorization` しか外さない。`x-api-key` のような独自のヘッダで秘密を送る HTTP クライアントは、ほかのクレート (MCP の HTTP・スタジオの API・エディタの更新確認など) でも `max_redirects(0)` か転送先の確認が要る。
- **ureq の本文の上限 (10 MB):** `read_to_vec`・`read_to_string` は黙って 10 MB で止まる。大きな応答を読むところは上限を明示する。
- **`Duration::from_secs_f64` などの外から来た値の変換:** 外部の値を `Duration`・整数に変える箇所は `try_*` を使う (あふれでパニックする)。
- **三角形の内外判定:** `ugoe-runtime/src/hit.rs` の `in_triangle` も、1 点につぶれた三角形と NaN でどの点にも当たる (T0137 の worker-T0137 に連絡済み)。点と三角形の判定は ugoe-ai・runtime・mesher のテストにそれぞれあり、共通化の候補。
- **エラーの本文の切り詰め:** 外部のエラーの本文を文言に入れるところは長さを制限する (ugoe-ai は 500 文字にそろえた)。

### just ci

すべて通った。`just ci` を 590 秒の `timeout` で包んで流したところ、`fmt-check`・`lint`・`test`・`layers`・`groups`・`licenses`・`publish-check`・`spec` が通った後、`web-check` の途中で時間切れになった (失敗ではなく、並行のビルドで遅いため)。残りの `just web-check i18n app-build-check` を流し直して通った。
