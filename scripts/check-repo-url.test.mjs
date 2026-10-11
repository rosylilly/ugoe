import assert from "node:assert/strict";
import { test } from "node:test";
import {
  findStaleManifestRepository,
  findStaleRepoUrls,
  parseRepositoryUrl,
  workspaceRepository,
} from "./check-repo-url.mjs";

// この中に食い違った URL をそのまま書くと、このファイル自身が検査に掛かるので、名前を `${…}` で差し込んで組み立てる
const expected = { owner: "acme", repo: "puppet" };
const [acme, puppet, other, puppet2, lib] = ["acme", "puppet", "someone", "puppet2", "lib"];
const gh = (owner, repo) => `https://github.com/${owner}/${repo}`;

test("the repository is read from [workspace.package] only", () => {
  const toml = [
    "[package]",
    'repository = "https://example.com/wrong"',
    "",
    "[workspace.package]",
    'version = "0.1.0"',
    `repository = "${gh(acme, puppet)}"`,
    "",
    "[workspace.dependencies]",
  ].join("\n");
  assert.equal(workspaceRepository(toml), gh(acme, puppet));
  assert.throws(() => workspaceRepository("[workspace.package]\nversion = 1\n"));
});

test("the repository URL is split into owner and repo", () => {
  assert.deepEqual(parseRepositoryUrl(gh(acme, puppet)), expected);
  assert.deepEqual(parseRepositoryUrl(`${gh(acme, puppet)}.git`), expected);
  assert.throws(() => parseRepositoryUrl("https://gitlab.com/acme/puppet"));
  assert.throws(() => parseRepositoryUrl(`${gh(acme, puppet)}/tree/main`));
});

test("URLs that match both owner and repo pass", () => {
  const text = [
    `see ${gh(acme, puppet)}/security/policy and ${gh(acme, puppet)}.git`,
    `[x](${gh("ACME", "Puppet")}/blob/main/LICENSE). And ${gh(acme, puppet)}.`,
    `git@github.com:${acme}/${puppet}.git ssh://git@github.com/${acme}/${puppet}`,
    `"$id": "https://raw.githubusercontent.com/${acme}/${puppet}/main/spec/a.json"`,
    `"repository": "github:${acme}/${puppet}"  gh attestation verify x --repo ${acme}/${puppet}`,
  ].join("\n");
  assert.deepEqual(findStaleRepoUrls(text, expected), []);
});

test("URLs of other repositories are not checked", () => {
  const text = `${gh(other, lib)} ${gh("serde-rs", "serde")} git@github.com:${other}/${lib}.git`;
  assert.deepEqual(findStaleRepoUrls(text, expected), []);
});

test("URLs that differ only in owner or repo are found", () => {
  const text = [
    "ok",
    `moved owner: ${gh(other, puppet)}/issues`,
    `renamed repo: ${gh(acme, puppet2)}`,
    `raw: https://raw.githubusercontent.com/${other}/${puppet}/main/x  ssh: git@github.com:${acme}/${puppet2}.git`,
    `npm: "github:${other}/${puppet}"  gh: --repo ${acme}/${puppet2}`,
  ].join("\n");
  assert.deepEqual(findStaleRepoUrls(text, expected), [
    { line: 2, match: `github.com/${other}/${puppet}` },
    { line: 3, match: `github.com/${acme}/${puppet2}` },
    { line: 4, match: `raw.githubusercontent.com/${other}/${puppet}` },
    { line: 4, match: `github.com:${acme}/${puppet2}.git` },
    { line: 5, match: `github:${other}/${puppet}` },
    { line: 5, match: `--repo ${acme}/${puppet2}` },
  ]);
});

test("the repository of a package manifest must be this repository", () => {
  const pkg = (repository) => JSON.stringify({ name: "x", repository }, null, 2);
  const check = (file, text) => findStaleManifestRepository(file, text, expected);
  assert.deepEqual(check("a/package.json", pkg(`${acme}/${puppet}`)), []);
  assert.deepEqual(check("package.json", pkg(`github:${acme}/${puppet}`)), []);
  assert.deepEqual(
    check("a/package.json", pkg({ type: "git", url: `${gh(acme, puppet)}.git` })),
    [],
  );
  // 片方だけが違うものは本文の検査が拾う
  assert.deepEqual(check("a/package.json", pkg({ type: "git", url: gh(other, puppet) })), []);
  // 両方が違うものと短い形はここで拾う
  assert.deepEqual(check("a/package.json", pkg(`${other}/${puppet}`)), [
    { line: 3, match: `${other}/${puppet}` },
  ]);
  assert.deepEqual(check("a/package.json", pkg({ type: "git", url: gh(other, lib) })), [
    { line: 5, match: gh(other, lib) },
  ]);
  assert.deepEqual(check("a/package.json", pkg("https://example.com/x")), [
    { line: 3, match: "https://example.com/x" },
  ]);
  assert.deepEqual(check("a/package.json", "{ not json"), []);
  assert.deepEqual(check("a/package.json", JSON.stringify({ name: "x" })), []);

  const cargo = (repository) =>
    `[package]\nname = "x"\nrepository.workspace = true\nrepository = "${repository}"\n`;
  assert.deepEqual(check("b/Cargo.toml", cargo(gh(acme, puppet))), []);
  assert.deepEqual(check("b/Cargo.toml", cargo(gh(other, lib))), [
    { line: 4, match: gh(other, lib) },
  ]);
  // ほかの名前のファイルは見ない
  assert.deepEqual(check("b/other.toml", cargo(gh(other, lib))), []);
});
