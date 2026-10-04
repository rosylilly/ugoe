---
id: T0439
title: npm: <ugoe-avatar> の作成の失敗と、作り直しを諦めた後の扱い (ready・やり直し)
status: done
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

- [x] `ready` を作成の失敗で reject するか、別の口 (`error` を待つ・`state` プロパティ) にするかを決めて直す。README に書く
- [x] 失敗した後・諦めた後にやり直す方法 (`restart()` のようなメソッドか、属性の変更で作り直す) を決めて足す
- [x] `src` の取得の資格情報 (`crossorigin` 属性など) と、置き換わった取得の中断を足すかを決める
- [x] 振る舞いを `bindings/web/src/avatar.test.ts` (happy-dom と `WebPuppet` の偽物) でテストする
- [x] `just ci` が通る

## 参考

- bindings/web/src/avatar.ts (`#start`・`#restart`・`#fetchSrc`)、bindings/web/README.md#ugoe-avatar

## メモ

### やったこと

- **`ready` は reject にした** (別の口の `state` プロパティは足していない)。作成の失敗 (`auto` は WebGL2 でも失敗したとき) と、
  GPU を続けて失って諦めたときに、その理由で reject し、`error` も送る。内部で `catch` を付けておき、誰も待っていなくても
  unhandledrejection にしない。作り直しを始める (`restart()`・`backend` の変更・繋ぎ直し) と、新しい `WebPuppet` を待つ Promise に変わる。
  - 失敗した状態で呼んだ `load()` は待たずに reject する (作成の失敗で settle しない Promise を作らないため。T0425 の横断の所見)。
- **やり直しは `restart()` メソッドと、`backend` の変更・繋ぎ直し** にした。`restart()` は作り直しの回数も数え直し、最後に読み込めた
  モデル (取得中の `src` があればその結果) を読み直す。パラメータの値は、諦めたときの値も引き継ぐ (GPU の喪失で作り直すときと同じ)。
  接続していなければ何もしない。属性の変更の受け付けを `#canvas` の有無から接続中かどうかに変え、諦めて canvas が無い間も
  `backend` の変更で作り直し、`src` の変更は取得して覚えるようにした。内部の `#restart` (GPU の喪失) は名前の衝突を避けて `#onLost` にした。
- **`crossorigin` 属性 (プロパティは `crossOrigin`) と取得の中断を足した。** `use-credentials` で `credentials: "include"`、
  省略・`anonymous`・知らない値は `"same-origin"` (`<img crossorigin>` と同じ対応。`parseCrossOrigin`)。属性を変えただけでは取り直さず、
  次の取得から効く (observedAttributes にも入れていない)。置き換わった取得 (`src` の変更・`src` を消す・`load()`) は
  `AbortController` で中断する。切断では中断しない (繋ぎ直したら結果を読む、という T0425 の振る舞いを保つ)。
- テスト: `src/avatar.test.ts` に 7 件 (作成の失敗で `ready`・`load` が reject、作成の失敗の後の `restart()` で `src` を取得、
  諦めた後の reject と `restart()` での読み直し・値の引き継ぎ・回数の数え直し、諦めた後の `backend` の変更、切断中の `restart()`、
  `crossorigin` の資格情報、置き換わった取得の中断)。`src/attributes.test.ts` に `parseCrossOrigin`。先に失敗することを確かめてから直した。
- README の `<ugoe-avatar>` に `crossorigin` の行と「失敗とやり直し」を書いた。

### やらなかったこと

- 本物のブラウザと wasm では確かめていない (happy-dom と `WebPuppet` の偽物だけ。本物のブラウザのテストは T0440)。
- `parseCrossOrigin` は index.ts から公開していない (属性の解釈の補助で、要素の外では使わないため)。
- `crossorigin` を変えたときに取り直すこと (`<img>` は取り直す) はしていない。要るなら observedAttributes に足すだけで済む。

### 起票の候補

- `ready` だけでは「今どの状態か」(作成中・使える・失敗) を同期的に知れない。UI でボタンの出し分けをしたい利用者が出たら、
  読み取り専用の `state` プロパティ (`"idle" | "creating" | "ready" | "failed"`) を足す。
