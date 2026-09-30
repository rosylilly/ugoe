---
id: T0124
title: 着せ替え: 付け直しで外れたモジュールを構造化されたデータで返す
status: done
milestone: M4
crates: [ugoe-wasm, apps/viewer, apps/editor, docs]
depends_on: [T0123, T0706]
assignee: claude-worker-T0124
---

# T0124: 着せ替え: 付け直しで外れたモジュールを構造化されたデータで返す

## 目的

利用者の要望 (T0706 の後続の候補)。モデルを開き直したときに合わなくなって外れたモジュールの警告を、wasm は英語の文で返している。
ビューアはその文を照らし合わせて日本語にしているので、文面が変わると英語のまま出る。

## 受け入れ条件

- [x] wasm が、付け直しで外れたモジュールを構造化されたデータ (モジュールの名前・スロット・`DressUpError` と同じ `code` と項目) で返す。今の文の警告はそのまま残すか、置き換えるかを決めて README に書く
- [x] ビューア (`dressUp.ts`) とエディタのプレビュー (T0649) が、文の照らし合わせをやめて構造化されたデータを使う
- [x] Rust のテストと Vitest。`just wasm-check`
- [x] `just ci`・ビューアとエディタのビルドが通る

## 参考

- `docs/tasks/T0706-viewer-dress-up.md`・`docs/tasks/T0123-wasm-dress-up.md` のメモ、`crates/ugoe-wasm/README.md#着せ替え-t0123`

## メモ

- **wasm:** `WebPuppet.detached_modules_json()` を足した。直前の `load_json` / `load_bundle` で付け直せずに外したモジュールの
  `[{module, slot, error: {code, message, module?, slot?, parameter?, mesh?, attached?}, warning}]` (付けていた順)。`error` は
  付けるときの `DressUpError` と同じ `code` と補足の値 (`dress_up::error_info` をそのまま使う) に英語の文を添えたもの。
  `Wardrobe::load_base` が外したものを `Detached` として持ち (`Wardrobe::detached`)、文の警告は `Detached::warning` から作る。
- **文の警告は残した** (理由は `crates/ugoe-wasm/README.md#着せ替え-t0123`): 戻り値の `string[]` はほかの警告と同じ一覧で、ログに出すだけの
  組み込み先はそのままでよく、消すと知らせが黙って消えるため。代わりに JSON の各要素に `warning` (戻り値の文と同じ文字列) を入れ、
  言い換える側は文を解析せずに、同じ文字列を一覧から除けるようにした。
- **ビューア:** `dressUp.ts` の正規表現 (`describeLoadWarning`) を `parseDetachedModules`・`describeLoadWarnings` に置き換えた。
  原因は付けられないときと同じ日本語の文 (`describeDressUpInfo`。`describeDressUpError` から中身の部分を分けた)。
  `player.svelte.ts` の 1 か所と `stub.d.ts` だけを変えた (パネルとサンプルのメニューには触れていない)。
- **エディタのプレビュー:** `DressUp.loaded` が `detached_modules_json` を読み、外れたものを知らせて付けたい組から外す
  (`describeProblem` は `code` と `message` を持つオブジェクトも読む)。その後で付けたい組と比べる 1 つずつ試す処理は残した
  (プレビューに入ったときは、出るときに外しているので WebPuppet は付け直しておらず、一覧が空のため)。同じ一覧で `loaded` が
  もう一度呼ばれても、付け直したものは知らせない。パネル (`DressUpSection.svelte`) は変えていない。
- **テスト:** Rust は `reloading_the_base_reattaches_modules_and_drops_leftovers` に構造の検査 (JSON の形・文の警告と同じ文字列)。
  Vitest はビューアの `dressUp.test.ts` に 2 件 (旧 `describeLoadWarning` の 2 件を置き換え)、エディタの `dressUp.test.ts` の
  読み直しの件を WebPuppet の一覧から知らせる形にし (set_modules を試し直さない・入り直しの 1 つずつの経路も)、偽の WebPuppet に
  `detached_modules_json`。
- **確かめたこと:** `just ci`・`just wasm-check`・ビューアとエディタの `vite build`・`cargo check -p ugoe-editor -p ugoe-viewer`。
  ヘッドレスの Chrome (SwiftShader) でビルドしたビューアに T0706 の見本 (`base.ugoe` に LongHair と Dress を付け、`hair-only.ugoe` を開く) を
  読ませ、警告が「このモデルに合わないため、着せ替えの「Dress」を外しました (「Dress」を付けるスロット「Outfit」が、このモデルにありません)」
  になることを確かめた。スクリーンショット: scratchpad `T0124/shots/reopen-detached.png`。
- **確かめていないこと:** エディタのプレビューの画面 (ブリッジで開くこと) は Vitest だけ。Tauri のウィンドウ。
- 文書: `crates/ugoe-wasm/README.md` (API と、文の警告を残す理由)・`apps/viewer/README.md`・`docs/editor.md#プレビューの着せ替え`。
  `docs/spec/modules.md` の 1 文 (外して警告する) は API の名前を書いていないので変えていない。
- **後続の候補:** 読み直しのほかの警告 (物理を外した・テクスチャを代用した) も構造で返すこと。今は英語のまま出る。
