---
id: T0439
title: npm: <ugoe-avatar> の作成の失敗と、作り直しを諦めた後の扱い (ready・やり直し)
status: in-progress
milestone: M9
crates: [bindings/web, docs]
depends_on: [T0425]
assignee: claude-worker-T0439
---

# T0439: <ugoe-avatar> の作成の失敗と、諦めた後の扱い

## 目的

T0425 (bindings/web のレビュー) で見つけた、失敗の後に要素が動かなくなる・待ち続ける点を決めて直す。公開の挙動
(`ready` の約束) を変えるので、レビューでは直さずに起票した。

- `ready` は `WebPuppet` ができたら解決するが、作成に失敗すると解決も reject もしない (`await avatar.ready` が待ち続ける)。
  T0425 で `load()` は作成の失敗で reject するようにした (`ready` は変えていない)。
- 作成に失敗した後や、GPU を続けて失って諦めた後 (`the GPU device was lost repeatedly`) は、繋ぎ直す (DOM から外して戻す) か
  `backend` を変える以外にやり直す方法が無い。諦めた後は canvas も消えるので `backend` の変更も効かない (`attributeChangedCallback` が
  `#canvas` の無いときは何もしない)。
- `src` の取得は `fetch` の既定 (同じオリジンだけ資格情報を送る) で、`crossorigin` に当たる指定や中断 (`AbortController`) が無い。
  置き換わった取得は結果を捨てるだけで、ダウンロードは続く。

## 受け入れ条件

- [ ] `ready` を作成の失敗で reject するか、別の口 (`error` を待つ・`state` プロパティ) にするかを決めて直す。README に書く
- [ ] 失敗した後・諦めた後にやり直す方法 (`restart()` のようなメソッドか、属性の変更で作り直す) を決めて足す
- [ ] `src` の取得の資格情報 (`crossorigin` 属性など) と、置き換わった取得の中断を足すかを決める
- [ ] 振る舞いを `bindings/web/src/avatar.test.ts` (happy-dom と `WebPuppet` の偽物) でテストする
- [ ] `just ci` が通る

## 参考

- bindings/web/src/avatar.ts (`#start`・`#restart`・`#fetchSrc`)、bindings/web/README.md#ugoe-avatar

## メモ
