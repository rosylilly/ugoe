---
id: T0003
title: bindings/web の型検査が、古くなった型の写し (src/wasm/) を使い続けないようにする
status: todo
milestone: M7
crates: [bindings/web, justfile, docs]
depends_on: [T0722]
assignee:
---

# T0003: 型の写しを作り直す

## 目的

`bindings/web/scripts/check.mjs` は、生成物 (`src/wasm/ugoe_wasm.d.ts`) が無いときだけ手書きの型 (`stub/ugoe_wasm.d.ts`) を写す。
一度写すと、スタブが更新されても写しは古いまま残り、`just ci` の `web-check` が「関数が無い」と落ちる (T0661 の rebase の後に
`set_tracking_kept` で起きた。main のチェックアウトにも古い写しが残っていた)。

## 受け入れ条件

- [ ] スタブから写したものか、wasm-bindgen の生成物かを見分け、スタブの写しならスタブが変わったときに写し直す
      (写しに印を付ける、または写さずに tsconfig の paths でスタブを指すなど)
- [ ] 本物の生成物があるときは、今どおりそれで検査する
- [ ] 古い写しが残った状態から `just ci` が通ることを確かめる。`bindings/web/README.md` を更新する

## 参考

- `bindings/web/scripts/check.mjs`

## メモ
