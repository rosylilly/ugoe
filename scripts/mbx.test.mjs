// scripts/lib/mbx.mjs の単体テスト (`just script-check` が実行する)。
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { delimiter, join } from "node:path";
import { describe, it } from "node:test";
import {
  cargoEnvWithMbx,
  exportMbxPath,
  findCargoDir,
  mbxDisabledValue,
  mbxEnabled,
  mbxShimDir,
} from "./lib/mbx.mjs";

const SHIM = join("/home", "me", ".local", "share", "mbx", "bin");
const RUSTUP = join("/home", "me", ".cargo", "bin");

// 渡したパスだけがファイルとして存在するとみなす
const files = (...paths) => {
  const set = new Set(paths);
  return (path) => set.has(path);
};
const withShim = files(join(SHIM, "cargo"), join(SHIM, "mbx-target"), join(RUSTUP, "cargo"));
const path = (...dirs) => dirs.join(delimiter);

describe("mbxDisabledValue", () => {
  it("無い・0・false は無効にしない", () => {
    assert.equal(mbxDisabledValue(undefined), false);
    assert.equal(mbxDisabledValue("0"), false);
    assert.equal(mbxDisabledValue("FALSE"), false);
  });

  it("それ以外の値 (空も含む) は無効にする", () => {
    assert.equal(mbxDisabledValue("1"), true);
    assert.equal(mbxDisabledValue("yes"), true);
    assert.equal(mbxDisabledValue(""), true);
  });
});

describe("findCargoDir", () => {
  it("PATH で最初に cargo のあるディレクトリを返す", () => {
    const opts = { isFile: withShim, names: ["cargo"] };
    assert.equal(findCargoDir(path("/usr/bin", SHIM, RUSTUP), opts), SHIM);
    assert.equal(findCargoDir(path(RUSTUP, SHIM), opts), RUSTUP);
    assert.equal(findCargoDir(path("/usr/bin"), opts), undefined);
    assert.equal(findCargoDir(undefined, opts), undefined);
  });
});

describe("mbxEnabled", () => {
  it("PATH の先頭の cargo が mbx の shim なら有効", () => {
    assert.equal(mbxEnabled({ PATH: path(SHIM, RUSTUP) }, { isFile: withShim }), true);
  });

  it("shim より前に素の cargo があれば無効", () => {
    assert.equal(mbxEnabled({ PATH: path(RUSTUP, SHIM) }, { isFile: withShim }), false);
  });

  it("MBX_DISABLE があれば shim が先頭でも無効", () => {
    const env = { PATH: path(SHIM, RUSTUP), MBX_DISABLE: "1" };
    assert.equal(mbxEnabled(env, { isFile: withShim }), false);
    assert.equal(mbxEnabled({ ...env, MBX_DISABLE: "0" }, { isFile: withShim }), true);
  });

  it("mbx-target の無い cargo (shim でないもの) は無効", () => {
    const isFile = files(join(SHIM, "cargo"));
    assert.equal(mbxEnabled({ PATH: path(SHIM) }, { isFile }), false);
  });
});

describe("mbxShimDir", () => {
  const home = join("/home", "me");

  it("Linux は XDG_DATA_HOME (絶対パスのとき) か ~/.local/share の下", () => {
    assert.equal(mbxShimDir({}, { platform: "linux", home }), SHIM);
    assert.equal(
      mbxShimDir({ XDG_DATA_HOME: "/data" }, { platform: "linux", home }),
      join("/data", "mbx", "bin"),
    );
    assert.equal(mbxShimDir({ XDG_DATA_HOME: "rel" }, { platform: "linux", home }), SHIM);
  });

  it("macOS は ~/Library/Application Support の下、Windows は扱わない", () => {
    assert.equal(
      mbxShimDir({}, { platform: "darwin", home }),
      join(home, "Library", "Application Support", "mbx", "bin"),
    );
    assert.equal(mbxShimDir({}, { platform: "win32", home }), undefined);
  });
});

describe("cargoEnvWithMbx", () => {
  const opts = { isFile: withShim, platform: "linux", home: join("/home", "me") };

  it("PATH の cargo がすでに shim ならそのまま", () => {
    const env = { PATH: path(SHIM, RUSTUP) };
    assert.deepEqual(cargoEnvWithMbx(env, opts), { env, viaMbx: true, installed: true });
  });

  it("shim が PATH に無ければ先頭に足す", () => {
    const { env, viaMbx, installed } = cargoEnvWithMbx({ PATH: path(RUSTUP), HOME: "x" }, opts);
    assert.equal(env.PATH, path(SHIM, RUSTUP));
    assert.equal(env.HOME, "x");
    assert.equal(viaMbx, true);
    assert.equal(installed, true);
  });

  it("MBX_DISABLE があれば PATH を変えず、mbx を通らない", () => {
    const env = { PATH: path(RUSTUP), MBX_DISABLE: "1" };
    assert.deepEqual(cargoEnvWithMbx(env, opts), { env, viaMbx: false, installed: true });
  });

  it("shim が入っていなければ PATH を変えない", () => {
    const env = { PATH: path(RUSTUP) };
    const isFile = files(join(RUSTUP, "cargo"));
    assert.deepEqual(cargoEnvWithMbx(env, { ...opts, isFile }), {
      env,
      viaMbx: false,
      installed: false,
    });
  });
});

describe("exportMbxPath", () => {
  const opts = { isFile: withShim, platform: "linux", home: join("/home", "me") };

  it("shim が PATH に無ければ、渡した環境の PATH の先頭に足す", () => {
    const env = { PATH: path(RUSTUP) };
    assert.equal(exportMbxPath(env, opts), true);
    assert.equal(env.PATH, path(SHIM, RUSTUP));
  });

  it("PATH の cargo がすでに shim なら変えない", () => {
    const env = { PATH: path(SHIM, RUSTUP) };
    assert.equal(exportMbxPath(env, opts), true);
    assert.equal(env.PATH, path(SHIM, RUSTUP));
  });

  it("MBX_DISABLE があるか shim が無ければ変えない", () => {
    const disabled = { PATH: path(RUSTUP), MBX_DISABLE: "1" };
    assert.equal(exportMbxPath(disabled, opts), false);
    assert.equal(disabled.PATH, path(RUSTUP));
    const none = { PATH: path(RUSTUP) };
    assert.equal(exportMbxPath(none, { ...opts, isFile: files(join(RUSTUP, "cargo")) }), false);
    assert.equal(none.PATH, path(RUSTUP));
  });

  it("Windows では変えない", () => {
    const env = { PATH: path(RUSTUP) };
    assert.equal(exportMbxPath(env, { ...opts, platform: "win32" }), false);
    assert.equal(env.PATH, path(RUSTUP));
  });
});

// just を通らずに cargo を呼ぶスクリプトが mbx を通すか (T0034)。cargo を子プロセスで呼ぶ (`"cargo", ...` を
// spawnSync・execFileSync などに渡す) か、cargo を呼ぶ `tauri build` を動かすスクリプトは、`exportMbxPath` か
// `cargoEnvWithMbx` を呼ぶ。新しいスクリプトがこれを忘れると、ここで失敗する。
describe("cargo を呼ぶスクリプト", () => {
  const root = join(import.meta.dirname, "..");
  const CALLS_CARGO = /[([]\s*["']cargo["']\s*,|["']tauri["']\s*,\s*["']build["']/;
  const USES_MBX = /\b(exportMbxPath|cargoEnvWithMbx)\(/;
  const scripts = execFileSync("git", ["ls-files", "-z", "--", "*.mjs", "*.js", "*.cjs"], {
    cwd: root,
    encoding: "utf8",
  })
    .split("\0")
    .filter((f) => f && !f.endsWith(".test.mjs") && !f.startsWith("scripts/lib/mbx.mjs"));

  it("どれも mbx を通す", () => {
    const callers = scripts.filter((f) => CALLS_CARGO.test(readFileSync(join(root, f), "utf8")));
    // 検出の正規表現が古くなって何も見つけなくなったら気づけるように
    assert.ok(callers.includes("tests/conformance/run.mjs"), callers.join(", "));
    const missing = callers.filter((f) => !USES_MBX.test(readFileSync(join(root, f), "utf8")));
    assert.deepEqual(missing, [], `exportMbxPath を呼んでいない: ${missing.join(", ")}`);
  });
});
