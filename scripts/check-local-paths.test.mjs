import assert from "node:assert/strict";
import { test } from "node:test";
import { findLocalPaths } from "./check-local-paths.mjs";

// この中に本物の形をそのまま書くと、このファイル自身が検査に掛かるので、名前を `${…}` で差し込んで組み立てる
const tmp = (rest) => `/tmp/${"claude"}-${rest}`;
const [alice, bob, carol, dave, erin, frank, grace] = [
  "alice",
  "bob",
  "carol",
  "dave",
  "erin",
  "frank",
  "grace",
];

test("home directories of real-looking users are found", () => {
  assert.deepEqual(findLocalPaths(`see /home/${alice}/src/ugoe/.worktrees/T0001/x.md\n`), [
    { line: 1, match: `/home/${alice}` },
  ]);
  assert.deepEqual(findLocalPaths(`a\n/Users/${bob}/Library\n/mnt/c/Users/${carol}/x`), [
    { line: 2, match: `/Users/${bob}` },
    { line: 3, match: `/Users/${carol}` },
  ]);
  assert.deepEqual(
    findLocalPaths(
      `C:\\Users\\${dave}\\AppData and "C:\\\\Users\\\\${erin}\\\\x" and C:/Users/${frank}`,
    ).map((f) => f.match),
    [`C:\\Users\\${dave}`, `C:\\\\Users\\\\${erin}`, `C:/Users/${frank}`],
  );
  assert.deepEqual(
    findLocalPaths(`\\\\wsl.localhost\\Ubuntu\\home\\${grace}\\x`).map((f) => f.match),
    [`\\home\\${grace}`],
  );
});

test("placeholder users and one-letter names are allowed", () => {
  const text = [
    "/home/u/a.psd /home/a /home/me/show /home/you/character.ugoe /home/audit/x",
    String.raw`C:\Users\a\x C:\\Users\\A B\\x /mnt/c/Users/a /Users/<name> /home/<名前> /home/… /Users/*`,
    "/home/runner/work /home/User/x",
  ].join("\n");
  assert.deepEqual(findLocalPaths(text), []);
});

test("agent temporary directories are found", () => {
  assert.deepEqual(findLocalPaths(`x\nimages: ${tmp("1000")}/-home-x/scratchpad/a.png`), [
    { line: 2, match: tmp("1") },
  ]);
  assert.deepEqual(findLocalPaths(`${tmp("<n>")}/…`), []);
});
