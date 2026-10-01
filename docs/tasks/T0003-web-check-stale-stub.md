---
id: T0003
title: bindings/web の型検査が、古くなった型の写し (src/wasm/) を使い続けないようにする
status: done
milestone: M7
crates: [bindings/web, justfile, docs]
depends_on: [T0722]
assignee: claude-worker-T0003
---

# T0003: 型の写しを作り直す

## 目的

`bindings/web/scripts/check.mjs` は、生成物 (`src/wasm/ugoe_wasm.d.ts`) が無いときだけ手書きの型 (`stub/ugoe_wasm.d.ts`) を写す。
一度写すと、スタブが更新されても写しは古いまま残り、`just ci` の `web-check` が「関数が無い」と落ちる (T0661 の rebase の後に
`set_tracking_kept` で起きた。main のチェックアウトにも古い写しが残っていた)。

## 受け入れ条件

- [x] スタブから写したものか、wasm-bindgen の生成物かを見分け、スタブの写しならスタブが変わったときに写し直す
      (写しに印を付ける、または写さずに tsconfig の paths でスタブを指すなど)
- [x] 本物の生成物があるときは、今どおりそれで検査する
- [x] 古い写しが残った状態から `just ci` が通ることを確かめる。`bindings/web/README.md` を更新する

## 参考

- `bindings/web/scripts/check.mjs`

## メモ

- 見分け方は「wasm 本体 `src/wasm/ugoe_wasm_bg.wasm` があるか」にした (`scripts/build.mjs` の `--no-wasm` と同じ基準)。
  本体が無ければ `ugoe_wasm.d.ts` はスタブの写しとみなし、検査のたびに写し直す。印の行は付けない
  (印の無い古い写し、つまり main のチェックアウトに残っているものもそのまま直せるため)。
  tsconfig の paths は、`./wasm/ugoe_wasm.js` という相対の import を差し替えられないので使わなかった。
- 確かめたこと: 先頭 40 行だけの古い写しを置いて `pnpm check` と `just ci` が通り、写しがスタブと同じになる。
  `pnpm build` の後は生成物が残り (末尾に足した行が消えない)、それで検査する。写しだけの状態で
  `build.mjs --no-wasm` は「ugoe_wasm_bg.wasm が無い」で止まり、写しを本物と取り違えない。
- justfile と CI の手順は変えていない。コミットのスコープは SCOPES に web が無いので wasm にした。

