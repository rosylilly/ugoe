---
id: T1673
title: "エディタ: キーの保管庫を開くのに失敗すると、アプリを開き直すまで API キーが「使えない」のままになるのを直す"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1673
---

# T1673: エディタ: キーの保管庫を開くのに失敗すると、アプリを開き直すまで API キーが「使えない」のままになるのを直す

## 目的

`apps/editor/src-tauri/src/ai.rs` の `ApiKeys` は、保管庫 (Secret Service など) を最初に使うときに開き、結果を `OnceLock` に覚える。
開くのが失敗 (T1665 のやり直しの後も `NoReply` が 3 回続くなど) すると、その失敗も覚えるので、アプリを閉じるまで API キーの状態は
「使えない」(`KeyStatus::Unavailable`) のままになる。一時的な失敗なら、次に使うときに開き直す。

## 受け入れ条件

- [x] 先に失敗するテスト: `ApiKeys::new` に最初は失敗し 2 回目に成功する `Opener` を渡すと、2 回目の `status`・読み書きで保管庫が使える
- [x] 開くのに成功した保管庫は覚えたまま、失敗は覚えずに次に使うときに開き直す (同時に呼ばれても二重に開かない・開き直しが続けて重くならない
      ことを確かめるか、扱いをコメントに書く)。失敗の種類 (保管庫が無い環境など、開き直しても変わらないもの) で分けるかを決めて書いた
- [x] docs/editor.md (API キーの保存先) に、開くのに失敗したときの振る舞いを書いた
- [x] `just ci` と `just cross-check` (Windows・macOS の clippy) が通る

## 参考

- T1665 のメモ (やったこと・起票の候補)、T0050 (鍵の保管庫)
- apps/editor/src-tauri/src/ai.rs (`ApiKeys`・`store`)、docs/editor.md の API キーの保存先

## メモ

- T0058 で起票した (元の候補: T1665 (開くのに失敗すると OnceLock に覚える))。
- 決めたこと:
  - 開けた保存先は `OnceLock` に覚え、失敗は覚えずに次に使うときに開き直す (`ApiKeys::store`)。開くのは `opening` の錠で 1 つずつにし、
    錠を待つ間にほかの呼び出しが開くのに失敗していたら (失敗の回数 `failed_opens` を錠の前後で比べる)、その失敗を返して続けて開き直さない
    (答えの来ない保存先を、待っていた呼び出しの数だけ待たないため)。時間での開き直しの間隔は設けない (開くのは利用者の操作のときだけ)。
  - 失敗の種類では分けない: `Opener` は文言だけを返し、保存先の無い環境 (D-Bus のセッションが無い・Secret Service が無い) は開き直しても
    すぐに失敗するので重くない。キーリングを後から動かした・解錠したときにも、開き直せば使える。
- やったこと:
  - `apps/editor/src-tauri/src/ai.rs`: `ApiKeys` の `store` を `OnceLock<Box<dyn SecretStore>>` + `opening: Mutex<Option<String>>` +
    `failed_opens: AtomicU64` にし、上の扱いを型の doc コメントに書いた。
  - テスト (ai_tests.rs、先に失敗することを確かめてからコミット): `store_is_reopened_after_a_failed_open` (1 回目に失敗する `Opener` で、
    2 回目の `status`・保存・読み出し・削除が使える。開けた後は開き直さない)、`concurrent_callers_share_one_open_attempt` (開いている間に
    待つ 3 つの呼び出しは同じ失敗を受け取り、開くのは 1 回。その後の呼び出しは開き直す)。後者を 20 回続けて流して通った。
  - docs/editor.md (API キーの保存先) に書いた。
- `just ci`・`just cross-check` (Windows の clippy。apps は x86_64-pc-windows-gnu、vcam は Windows と macOS) が通った。
  `just e2e --only editor` は 16 件すべて通った (鍵の保管庫の筋書きは e2e の使い捨ての保管庫で。利用者の `~/.local/share/keyrings` の更新の時刻は変わっていない)。
- 確かめていないこと: macOS・Windows の実機 (振る舞いは OS に依らない Rust のコード)。本物の保管庫が一時的に失敗して後で開ける場面は、偽の `Opener` でだけ確かめた。
- 起票の候補: なし。
