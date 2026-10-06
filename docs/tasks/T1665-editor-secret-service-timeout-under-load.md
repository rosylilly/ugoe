---
id: T1665
title: "エディタ: 重い負荷の下で Secret Service の呼び出しが 2 秒で諦め、API キーの保存・読み出しが失敗するのを直す"
status: todo
milestone: M10
crates: [apps/editor, tests, docs]
depends_on: []
assignee:
---

# T1665: エディタ: 重い負荷の下で Secret Service の呼び出しが 2 秒で諦め、API キーの保存・読み出しが失敗するのを直す

## 目的

T0050 で e2e に鍵の保管庫 (gnome-keyring) を足したところ、重い負荷の下 (並行のビルドで load average 20 ほど) で API キーの保存が
「DBus error: Did not receive a reply」で 1 回失敗した。dbus-secret-service 4.1 は Secret Service の呼び出しを 2 秒で諦める
(`proxy/mod.rs` の `Duration::from_millis(2000)`)。e2e の筋書きは、この失敗のときだけ知らせを閉じて 1 回やり直している。
Linux の利用者も、重いときに保存・読み出し・削除が失敗し得る。

## 受け入れ条件

- [ ] 時間の上限を延ばせるか (dbus-secret-service-keyring-store・dbus-secret-service の口や feature) を調べ、延ばすか、
      「返事が無い」失敗のときだけ 1 回 (か上限のある回数) やり直すかを決めた。理由と調べた版をメモに書いた
- [ ] 保存・読み出し・削除のすべての経路 (`apps/editor/src-tauri/src/ai.rs` の保管庫の trait の実装) に同じ扱いをした。やり直しは Linux の
      Secret Service だけで、ほかの OS の保管庫の振る舞いは変えない。呼び出しはメインのスレッドを止めない (async のコマンドのまま)
- [ ] Rust のテスト: 偽の保管庫で、1 回目が「返事が無い」で失敗し 2 回目が通ると成功し、続けて失敗すると今のエラーのコードを返す。ほかの失敗はやり直さない
- [ ] tests/e2e の筋書きの「1 回やり直す」を外せるなら外し、外さないなら理由を書いた。`just e2e --only editor` が通る
- [ ] docs/editor.md (AI の設定・API キーの保存) に書いた。`just ci` が通る

## 参考

- T0050 のメモ (分かったこと・起票の候補)、tests/e2e/README.md の「鍵の保管庫」
- apps/editor/src-tauri/src/ai.rs、ルートの Cargo.toml (`dbus-secret-service-keyring-store`・`keyring-core`)

## メモ

- T0052 で起票した (元の候補: T0050 (Secret Service の 2 秒の上限))。
- 新しい依存や feature を足すなら PR の本文に理由を書く (AGENTS.md)。
