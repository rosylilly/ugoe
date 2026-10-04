// npm の依存で、package.json に SPDX の式を持たないものの、確かめたライセンスの一覧。
// scripts/check-licenses.mjs (検査) と scripts/third-party-notices.mjs (アプリの同梱物の表記) の両方がこれを読む。
// 単体テストは scripts/npm-license-exceptions.test.mjs (`just script-check`)。
//
// 登録のしかた: パッケージの tarball の LICENSE などを読んで実際のライセンスを確かめ、名前 -> { 確かめた版・
// ライセンス (SPDX の式)・確かめ方 } を書く。版が変わると scripts/check-licenses.mjs の検査が失敗するので、
// 同じ方法で確かめ直して版を書き換える。

/**
 * 名前 -> `{ version, license, reason }`。`version` は確かめた版 (node_modules のものと完全に一致するときだけ例外が効く)、
 * `license` は SPDX の式、`reason` は確かめ方。
 */
export const NPM_LICENSE_EXCEPTIONS = Object.freeze({
  "svelte-toolbelt": Object.freeze({
    version: "0.10.6",
    license: "MIT",
    reason:
      "package.json に license の欄が無い。npm の tarball の LICENSE が MIT の条文 (Hunter Johnston・Thomas G. Lopes) であることを読んで確かめた (2026-10-02、ADR 0029)",
  }),
});
