// scripts/lib/cargo-locked.mjs の単体テスト (`just script-check` が実行する)。
import assert from "node:assert/strict";
import { join } from "node:path";
import { describe, it } from "node:test";
import { staleLockMessage, staleLockPath } from "./lib/cargo-locked.mjs";

const ROOT = join("/home", "me", "ugoe");
// cargo 1.9x の出力 (fuzz/ の依存がランタイムのクレートの新しい依存に追いついていないとき)
const stderr = (lock) =>
  [
    "    Updating crates.io index",
    `error: cannot update the lock file ${lock} because --locked was passed to prevent this`,
    "help: to generate the lock file without accessing the network, remove the --locked flag and use --offline instead.",
    "",
  ].join("\n");

describe("staleLockPath", () => {
  it("古い lock のファイルのパスを取り出す", () => {
    const lock = join(ROOT, "fuzz", "Cargo.lock");
    assert.equal(staleLockPath(stderr(lock)), lock);
  });

  it("古い版の cargo の文言も見分ける", () => {
    const lock = join(ROOT, "Cargo.lock");
    assert.equal(
      staleLockPath(
        `error: the lock file ${lock} needs to be updated but --locked was passed to prevent this`,
      ),
      lock,
    );
  });

  it("ほかの失敗では undefined", () => {
    assert.equal(staleLockPath("error: failed to parse manifest at `/x/Cargo.toml`"), undefined);
    assert.equal(staleLockPath(""), undefined);
    assert.equal(staleLockPath(undefined), undefined);
  });
});

describe("staleLockMessage", () => {
  it("fuzz/Cargo.lock が古いとき、ファイルと直し方を短く出す", () => {
    assert.equal(
      staleLockMessage(stderr(join(ROOT, "fuzz", "Cargo.lock")), ROOT),
      [
        "error: fuzz/Cargo.lock が古い (Cargo.toml の依存と合わず、--locked で止めた)",
        "直し方: cargo update --manifest-path fuzz/Cargo.toml --workspace で足りない依存を足し、lock のファイルをコミットする",
      ].join("\n"),
    );
  });

  it("bindings/godot とルートの lock も、そのマニフェストで直し方を出す", () => {
    assert.match(
      staleLockMessage(stderr(join(ROOT, "bindings", "godot", "Cargo.lock")), ROOT),
      /^error: bindings\/godot\/Cargo\.lock が古い.*\n直し方: cargo update --manifest-path bindings\/godot\/Cargo\.toml --workspace /,
    );
    assert.match(
      staleLockMessage(stderr(join(ROOT, "Cargo.lock")), ROOT),
      /^error: Cargo\.lock が古い.*\n直し方: cargo update --manifest-path Cargo\.toml --workspace /,
    );
  });

  it("ルートの外の lock は絶対パスのまま出す", () => {
    const lock = join("/elsewhere", "Cargo.lock");
    assert.match(staleLockMessage(stderr(lock), ROOT), /^error: \/elsewhere\/Cargo\.lock が古い/);
  });

  it("lock が古いための失敗でなければ undefined (呼び出し側は cargo の出力をそのまま出す)", () => {
    assert.equal(staleLockMessage("error: could not find `Cargo.toml`", ROOT), undefined);
  });
});
