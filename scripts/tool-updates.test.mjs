// scripts/tool-updates.mjs の単体テスト (`just script-check` が実行する)。上流には問い合わせない (fetch を差し替える)。
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  checkTool,
  compareVersions,
  listTable,
  PINNED_TOOLS,
  parseVersion,
  pickLatest,
  readPins,
  resultTable,
} from "./tool-updates.mjs";

describe("parseVersion", () => {
  it("v の有無を問わず数の並びにする", () => {
    assert.deepEqual(parseVersion("v8.30.1"), [8, 30, 1]);
    assert.deepEqual(parseVersion("1.98.1"), [1, 98, 1]);
  });

  it("先行版や形の違うタグは null", () => {
    assert.equal(parseVersion("v2.0.0-rc.1"), null);
    assert.equal(parseVersion("nightly"), null);
    assert.equal(parseVersion("4.7.1-stable"), null);
  });

  it("道具ごとのタグの形 (Godot の -stable)", () => {
    const tag = PINNED_TOOLS.find((t) => t.name === "Godot").tag;
    assert.deepEqual(parseVersion("4.7-stable", tag), [4, 7]);
    assert.equal(parseVersion("4.8-beta1", tag), null);
  });
});

describe("compareVersions", () => {
  it("桁ごとに数として比べる (文字列の順ではない)", () => {
    assert.equal(compareVersions([0, 10, 0], [0, 9, 9]), 1);
    assert.equal(compareVersions([1, 2, 3], [1, 2, 4]), -1);
  });

  it("足りない桁は 0 と見る", () => {
    assert.equal(compareVersions([4, 7], [4, 7, 0]), 0);
    assert.equal(compareVersions([4, 7], [4, 7, 1]), -1);
  });
});

describe("pickLatest", () => {
  it("並びの順ではなく版で最も新しいものと、同じメジャーで最も新しいものを選ぶ", () => {
    const tags = ["v22.23.3", "v26.11.1", "v27.0.0", "v26.9.0", "v27.1.0-rc.1"];
    assert.deepEqual(pickLatest(tags, [26, 10, 0]), { latest: "v27.0.0", sameMajor: "v26.11.1" });
  });

  it("合うものが無ければ null", () => {
    assert.deepEqual(pickLatest(["nightly"], [1, 0, 0]), { latest: null, sameMajor: null });
  });
});

/** 決めた中身を返す fetch の代わり。 */
function fakeFetch(body, { ok = true, status = 200 } = {}) {
  const calls = [];
  const impl = async (url, init) => {
    calls.push({ url, init });
    return { ok, status, json: async () => body };
  };
  return { impl, calls };
}

describe("checkTool", () => {
  const godot = PINNED_TOOLS.find((t) => t.name === "Godot");
  const cbindgen = PINNED_TOOLS.find((t) => t.name === "cbindgen");

  it("GitHub の Release から下書きと先行版を除いて比べ、トークンを渡す", async () => {
    const { impl, calls } = fakeFetch([
      { tag_name: "4.8-stable", draft: true, prerelease: false },
      { tag_name: "4.8-rc1", draft: false, prerelease: true },
      { tag_name: "3.6.3-stable", draft: false, prerelease: false },
      { tag_name: "4.7.2-stable", draft: false, prerelease: false },
    ]);
    const r = await checkTool(
      { tool: godot, pinned: "4.7.1-stable" },
      { fetchImpl: impl, token: "t" },
    );
    assert.equal(r.status, "update");
    assert.equal(r.latest, "4.7.2-stable");
    assert.match(calls[0].url, /repos\/godotengine\/godot\/releases/);
    assert.equal(calls[0].init.headers.Authorization, "Bearer t");
  });

  it("crates.io の取り下げた版を除き、固定と同じなら current", async () => {
    const { impl, calls } = fakeFetch({
      versions: [
        { num: "0.30.0", yanked: true },
        { num: "0.29.4", yanked: false },
        { num: "0.29.3", yanked: false },
      ],
    });
    const r = await checkTool({ tool: cbindgen, pinned: "0.29.4" }, { fetchImpl: impl });
    assert.equal(r.status, "current");
    assert.equal(r.latest, "0.29.4");
    assert.match(calls[0].url, /crates\.io\/api\/v1\/crates\/cbindgen$/);
    assert.ok(calls[0].init.headers["User-Agent"]);
  });

  it("上流を読めなければ error (投げない)", async () => {
    const { impl } = fakeFetch({}, { ok: false, status: 503 });
    const r = await checkTool({ tool: cbindgen, pinned: "0.29.4" }, { fetchImpl: impl });
    assert.equal(r.status, "error");
    assert.match(r.error, /503/);
  });
});

describe("readPins", () => {
  const tool = {
    name: "demo",
    pin: { file: "a.mjs", pattern: /VERSION = "([^"]+)"/ },
    mirrors: [{ file: "b.yml", pattern: /tool: demo@(\S+)/g }],
    upstream: { crate: "demo" },
    bump: "",
  };

  it("写した場所の版が揃っていれば問題なし", () => {
    const files = { "a.mjs": 'VERSION = "1.2.3"', "b.yml": "tool: demo@1.2.3\ntool: demo@1.2.3" };
    const { pins, problems } = readPins([tool], (f) => files[f]);
    assert.deepEqual(problems, []);
    assert.equal(pins[0].pinned, "1.2.3");
  });

  it("違う版の写しと、写しの無いファイルを見つける", () => {
    const stale = { "a.mjs": 'VERSION = "1.2.4"', "b.yml": "tool: demo@1.2.3" };
    assert.match(readPins([tool], (f) => stale[f]).problems[0], /b\.yml has 1\.2\.3/);
    const gone = { "a.mjs": 'VERSION = "1.2.3"', "b.yml": "" };
    assert.match(readPins([tool], (f) => gone[f]).problems[0], /no copy/);
  });

  it("固定した版が読めなければ問題にする", () => {
    const files = { "a.mjs": "", "b.yml": "" };
    assert.match(readPins([tool], (f) => files[f]).problems[0], /no version found/);
  });

  it("リポジトリの一覧はすべて読め、写した場所の版が揃っている", () => {
    const { pins, problems } = readPins();
    assert.deepEqual(problems, []);
    assert.equal(pins.length, PINNED_TOOLS.length);
  });
});

describe("表", () => {
  it("新しい版のあるものを先に並べ、同じメジャーの最新は最新と違うときだけ書く", () => {
    const [a, b] = PINNED_TOOLS;
    const table = resultTable([
      { tool: a, pinned: "1.0.0", latest: "1.0.0", sameMajor: "1.0.0", status: "current" },
      { tool: b, pinned: "1.0.0", latest: "2.0.0", sameMajor: "1.1.0", status: "update" },
    ]);
    const rows = table
      .split("\n")
      .filter((l) => l.startsWith(`| ${a.name}`) || l.startsWith(`| ${b.name}`));
    assert.match(
      rows[0],
      new RegExp(`^\\| ${b.name} \\| 1\\.0\\.0 \\| 2\\.0\\.0 \\| 1\\.1\\.0 \\| update`),
    );
    assert.match(rows[1], /\| 1\.0\.0 \| - \| current/);
    assert.match(table, /1 pinned tool\(s\) have a newer upstream version/);
  });

  it("一覧の表は道具ごとに 1 行", () => {
    const { pins } = readPins();
    assert.equal(listTable(pins).trim().split("\n").length, PINNED_TOOLS.length + 2);
  });
});
