---
id: T0524
title: MCP: --root のとき、モジュールのプロジェクトが参照する本体も root の下に限る
status: todo
milestone: M6
crates: [ugoe-mcp, ugoe-editor-core, docs]
depends_on: [T0515, T0662]
assignee:
---

# T0524: モジュールの本体の参照を --root で限る

## 目的

T0662 の報告で見つかった穴を塞ぐ。`--root` を付けても、`open_project` で開いたモジュールのプロジェクトが本体の参照で root の外を
指していると、その本体を読んでしまう (T0515 の前から)。`--http` で外のエージェントに開くときに、root の外のファイルが読まれうる。

## 受け入れ条件

- [ ] `--root` のとき、モジュールのプロジェクトの本体の参照 (`.ugoeproj`・`.ugoe`) が root の外を指していたら、読まずに分かるエラーを返す
      (どの参照が外を指しているかを書く。シンボリックリンクで外へ出る場合も含む)
- [ ] `import_psd` の `base`・そのほか本体やモジュールを読むツールで、パスの検査に漏れが無いことを洗い出してテストする
- [ ] `--root` を付けないとき (stdio の既定) の挙動は変えない
- [ ] ugoe-mcp の README の「パスを限る」と `docs/manual/mcp.md` を更新する。`just ci` が通る

## 参考

- `docs/tasks/T0515-mcp-root-dir.md`・`docs/tasks/T0662-mcp-module-autorig-and-sample-shoulders.md` のメモ

## メモ
