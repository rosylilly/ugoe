---
id: T1673
title: "エディタ: キーの保管庫を開くのに失敗すると、アプリを開き直すまで API キーが「使えない」のままになるのを直す"
status: todo
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee:
---

# T1673: エディタ: キーの保管庫を開くのに失敗すると、アプリを開き直すまで API キーが「使えない」のままになるのを直す

## 目的

`apps/editor/src-tauri/src/ai.rs` の `ApiKeys` は、保管庫 (Secret Service など) を最初に使うときに開き、結果を `OnceLock` に覚える。
開くのが失敗 (T1665 のやり直しの後も `NoReply` が 3 回続くなど) すると、その失敗も覚えるので、アプリを閉じるまで API キーの状態は
「使えない」(`KeyStatus::Unavailable`) のままになる。一時的な失敗なら、次に使うときに開き直す。

## 受け入れ条件

- [ ] 先に失敗するテスト: `ApiKeys::new` に最初は失敗し 2 回目に成功する `Opener` を渡すと、2 回目の `status`・読み書きで保管庫が使える
- [ ] 開くのに成功した保管庫は覚えたまま、失敗は覚えずに次に使うときに開き直す (同時に呼ばれても二重に開かない・開き直しが続けて重くならない
      ことを確かめるか、扱いをコメントに書く)。失敗の種類 (保管庫が無い環境など、開き直しても変わらないもの) で分けるかを決めて書いた
- [ ] docs/editor.md (API キーの保存先) に、開くのに失敗したときの振る舞いを書いた
- [ ] `just ci` と `just cross-check` (Windows・macOS の clippy) が通る

## 参考

- T1665 のメモ (やったこと・起票の候補)、T0050 (鍵の保管庫)
- apps/editor/src-tauri/src/ai.rs (`ApiKeys`・`store`)、docs/editor.md の API キーの保存先

## メモ

- T0058 で起票した (元の候補: T1665 (開くのに失敗すると OnceLock に覚える))。
