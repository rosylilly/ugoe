// scripts/lib/worktree-cargo-config.mjs の単体テスト (`just script-check` が実行する)。
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { CARGO_CONFIG, cargoConfigAction, MARKER } from "./lib/worktree-cargo-config.mjs";

describe("cargoConfigAction", () => {
  it("無ければ書く", () => {
    assert.equal(cargoConfigAction(undefined), "write");
  });

  it("同じ内容なら何もしない", () => {
    assert.equal(cargoConfigAction(CARGO_CONFIG), "keep");
  });

  it("前に書いたもの (1 行目が MARKER) で内容が違えば書き直す", () => {
    assert.equal(cargoConfigAction(`${MARKER}\n[profile.dev]\ndebug = 0\n`), "write");
  });

  it("手で書いたものは上書きしない", () => {
    assert.equal(cargoConfigAction("[profile.dev]\ndebug = true\n"), "skip");
    assert.equal(cargoConfigAction(""), "skip");
  });
});

describe("CARGO_CONFIG", () => {
  it("1 行目が MARKER で、dev のデバッグ情報を行の表だけにする", () => {
    assert.ok(CARGO_CONFIG.startsWith(MARKER));
    assert.match(CARGO_CONFIG, /^\[profile\.dev\]\ndebug = "line-tables-only"$/m);
  });
});
