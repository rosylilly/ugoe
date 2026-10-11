// scripts/ci-groups.mjs の検査のテスト (`just groups`)
import assert from "node:assert/strict";
import { test } from "node:test";
import {
  affected,
  cargoArgs,
  crateArgs,
  GROUPS,
  isDocsOnly,
  OS_CRATES,
  osCargoArgs,
  validate,
  validateDocsReads,
  validateOs,
  workspaceCrates,
} from "./ci-groups.mjs";

const pkg = (name, { features = {}, targets = [] } = {}) => ({
  name,
  features,
  targets: [{ name, kind: ["lib"] }, ...targets],
});

test("すべてのクレートがちょうど 1 つのグループに入っていれば問題なし", () => {
  const metadata = { packages: [pkg("a"), pkg("b")] };
  assert.deepEqual(validate({ x: { crates: ["a"] }, y: { crates: ["b"] } }, metadata), []);
});

test("どのグループにも入っていないクレートと、2 つのグループに入っているクレートを見つける", () => {
  const metadata = { packages: [pkg("a"), pkg("b"), pkg("c")] };
  const errors = validate({ x: { crates: ["a", "b"] }, y: { crates: ["b"] } }, metadata);
  assert.equal(errors.length, 2);
  assert.match(errors[0], /^b: x と y/);
  assert.match(errors[1], /^c: どのグループにも入っていない/);
});

test("ワークスペースにないクレートと空のグループを見つける", () => {
  const metadata = { packages: [pkg("a")] };
  const errors = validate({ x: { crates: ["a", "gone"] }, y: { crates: [] } }, metadata);
  assert.equal(errors.length, 2);
  assert.match(errors[0], /gone はワークスペースのクレートではない/);
  assert.match(errors[1], /^y: クレートが 1 つもない/);
});

test("required-features はグループの features に書く (gpu-tests は除く)", () => {
  const targets = [
    { name: "spec", kind: ["test"], "required-features": ["schema"] },
    { name: "gpu", kind: ["test"], "required-features": ["gpu-tests"] },
  ];
  const metadata = {
    packages: [pkg("a", { features: { schema: [], "gpu-tests": [] }, targets })],
  };
  const missing = validate({ x: { crates: ["a"] } }, metadata);
  assert.equal(missing.length, 1);
  assert.match(missing[0], /test spec は feature schema が要る/);
  assert.deepEqual(validate({ x: { crates: ["a"], features: ["a/schema"] } }, metadata), []);
});

test("features はグループのクレートの、実在する feature に限る", () => {
  const metadata = { packages: [pkg("a", { features: { schema: [] } }), pkg("b")] };
  const errors = validate(
    { x: { crates: ["a"], features: ["b/schema", "a/nope", "schema"] }, y: { crates: ["b"] } },
    metadata,
  );
  assert.equal(errors.length, 3);
});

test("cargo の引数は -p の並びと --features", () => {
  assert.deepEqual(cargoArgs({ crates: ["a", "b"] }), ["-p", "a", "-p", "b"]);
  assert.deepEqual(cargoArgs({ crates: ["a"], features: ["a/x", "a/y"] }), [
    "-p",
    "a",
    "--features",
    "a/x,a/y",
  ]);
});

test("--all-features はグループの features の代わりに付ける", () => {
  assert.deepEqual(cargoArgs({ crates: ["a"], features: ["a/x"] }, { allFeatures: true }), [
    "-p",
    "a",
    "--all-features",
  ]);
});

test("OS_CRATES はワークスペースのクレートに限る", () => {
  const metadata = { packages: [pkg("a")] };
  assert.deepEqual(validateOs(["a"], metadata), []);
  assert.deepEqual(validateOs(["a", "gone"], metadata), [
    "OS_CRATES: gone はワークスペースのクレートではない",
  ]);
  assert.deepEqual(validateOs([], metadata), ["OS_CRATES: クレートが 1 つもない"]);
  assert.ok(OS_CRATES.length > 0);
});

test("OS のジョブの clippy はすべての feature で警告をエラーにし、test は引数をそのまま渡す", () => {
  assert.deepEqual(osCargoArgs("clippy", ["a", "b"]), [
    "clippy",
    "-p",
    "a",
    "-p",
    "b",
    "--all-features",
    "--all-targets",
    "--",
    "-D",
    "warnings",
  ]);
  assert.deepEqual(osCargoArgs("test", ["a"], ["--", "--nocapture"]), [
    "test",
    "-p",
    "a",
    "--",
    "--nocapture",
  ]);
  assert.throws(() => osCargoArgs("build", ["a"]), /unknown command: build/);
});

test("1 つのクレートの引数は、グループの features のうちそのクレートのものだけを付ける", () => {
  const groups = {
    g: { crates: ["a", "b"], features: ["a/x", "b/y", "a/z"] },
    h: { crates: ["c"] },
  };
  assert.deepEqual(crateArgs(groups, "a"), ["-p", "a", "--features", "a/x,a/z"]);
  assert.deepEqual(crateArgs(groups, "c"), ["-p", "c"]);
  assert.deepEqual(crateArgs(groups, "unknown"), ["-p", "unknown"]);
  // png を使う結合テストは required-features なので、just crate ugoe-format でも流れる (T0186)
  assert.deepEqual(crateArgs(GROUPS, "ugoe-format"), [
    "-p",
    "ugoe-format",
    "--features",
    "ugoe-format/png",
  ]);
});

// 変わったパスでの絞り込み (affected)。グループとクレートは本物に似せた小さな偽物
const crates = [
  { name: "core", dir: "crates/core/", deps: [] },
  { name: "ffi", dir: "crates/ffi/", deps: ["core"] },
  { name: "cli", dir: "crates/cli/", deps: ["core"] },
  { name: "editor", dir: "apps/editor/src-tauri/", deps: ["cli"] },
  { name: "studio", dir: "apps/studio/src-tauri/", deps: ["core", "vcam"] },
  { name: "vcam", dir: "apps/studio/vcam/", deps: [] },
];
const groups = {
  runtime: { crates: ["core"], covers: ["ffi"] },
  bindings: { crates: ["ffi"] },
  tools: { crates: ["cli"] },
  apps: { crates: ["editor", "studio", "vcam"], tauri: true },
};

test("文書だけの変更は何も動かさない (テストが読む文書とクレートの中の .md は除く)", () => {
  assert.deepEqual(
    affected(["docs/tasks/README.md", "AGENTS.md", "apps/editor/README.md"], groups, crates),
    {
      groups: [],
      web: false,
    },
  );
  assert.equal(isDocsOnly("docs/manual/images/a.png", crates), true);
  assert.equal(isDocsOnly("docs/spec/file-formats.md", crates), false);
  assert.equal(isDocsOnly("docs/editor.md", crates), false);
  assert.equal(isDocsOnly("docs/studio-api.schema.json", crates), false);
  assert.equal(isDocsOnly("crates/cli/README.md", crates), false);
  assert.equal(isDocsOnly("spec/schema/model.json", crates), false);
});

test("クレートの変更は、そのクレートと依存するクレートのグループを動かす", () => {
  assert.deepEqual(affected(["crates/cli/src/main.rs"], groups, crates), {
    groups: ["tools", "apps"],
    web: true,
  });
  assert.deepEqual(affected(["crates/core/src/lib.rs"], groups, crates).groups, [
    "runtime",
    "bindings",
    "tools",
    "apps",
  ]);
  assert.deepEqual(affected(["apps/studio/vcam/src/lib.rs"], groups, crates).groups, ["apps"]);
});

test("ほかのグループのクレートも検査するグループ (covers) は、そのクレートの変更でも動く", () => {
  assert.deepEqual(affected(["crates/ffi/src/lib.rs"], groups, crates).groups, [
    "runtime",
    "bindings",
  ]);
});

test("アプリのフロントエンドはそのアプリのグループとフロントエンドの検査を、packages/ などはフロントエンドの検査だけを動かす", () => {
  assert.deepEqual(affected(["apps/studio/src/App.svelte"], groups, crates), {
    groups: ["apps"],
    web: true,
  });
  assert.deepEqual(
    affected(["packages/ui/src/Button.svelte", "bindings/web/src/index.ts"], groups, crates),
    {
      groups: [],
      web: true,
    },
  );
});

test("クレートの外の Rust に関わるファイルは、すべてのグループを動かす", () => {
  for (const file of [
    "Cargo.lock",
    "justfile",
    ".github/workflows/ci.yml",
    "assets/samples/a.ugoe",
    "spec/schema/model.json",
  ]) {
    assert.deepEqual(affected([file], groups, crates).groups, Object.keys(groups), file);
  }
});

test("cargo metadata から、ルートからの相対のディレクトリとワークスペースの中の依存を取り出す", () => {
  const metadata = {
    workspace_root: "/repo",
    packages: [
      { name: "a", manifest_path: "/repo/crates/a/Cargo.toml", dependencies: [] },
      {
        name: "b",
        manifest_path: "/repo/apps/b/src-tauri/Cargo.toml",
        dependencies: [
          { name: "a", path: "/repo/crates/a", kind: "dev" },
          { name: "a", path: "/repo/crates/a" },
          { name: "serde" },
        ],
      },
    ],
  };
  assert.deepEqual(workspaceCrates(metadata), [
    { name: "a", dir: "crates/a/", deps: [] },
    { name: "b", dir: "apps/b/src-tauri/", deps: ["a"] },
  ]);
});

// テストが読む文書 (DOCS_READ_BY_TESTS) は本物のクレートの名前で書いてあるので、その名前の偽物で確かめる
const readers = [
  { name: "ugoe-format", dir: "crates/ugoe-format/", deps: [] },
  { name: "ugoe-editor-core", dir: "crates/ugoe-editor-core/", deps: ["ugoe-format"] },
  { name: "ugoe-editor", dir: "apps/editor/src-tauri/", deps: ["ugoe-editor-core"] },
];
const readerGroups = {
  runtime: { crates: ["ugoe-format"] },
  tools: { crates: ["ugoe-editor-core"] },
  apps: { crates: ["ugoe-editor"] },
};

test("テストが読む文書は、読むクレートの変更として扱う (フロントエンドの検査は動かさない)", () => {
  assert.deepEqual(affected(["docs/editor.md"], readerGroups, readers), {
    groups: ["tools", "apps"],
    web: false,
  });
  assert.deepEqual(affected(["docs/spec/file-formats.md"], readerGroups, readers).groups, [
    "runtime",
    "tools",
    "apps",
  ]);
});

test("Rust のソースが読む文書が、文書だけの変更か読むクレートの外なら見つける (コメントとメッセージは見ない)", () => {
  const sources = [
    {
      file: "crates/ugoe-editor-core/src/lib.rs",
      text: [
        '    let a = include_str!("../../../docs/editor.md");',
        '        "/../../docs/spec/file-formats.md"',
        '    let b = include_str!("../../../docs/guide.md");',
        '    // include_str!("../../../docs/other.md")',
        '    const URL: &str = "https://example.com/docs/manual/mcp.md";',
        '    eprintln!("over the limit (docs/testing.md)");',
        '    let c = Path::new("docs/manual/images/a.png");',
      ].join("\n"),
    },
  ];
  const errors = validateDocsReads(sources, readers);
  assert.equal(errors.length, 3);
  assert.match(
    errors[0],
    /^crates\/ugoe-editor-core\/src\/lib\.rs:2: docs\/spec\/file-formats\.md を読んでいるが、DOCS_READ_BY_TESTS の読むクレートに ugoe-editor-core が無い/,
  );
  assert.match(errors[1], /:3: docs\/guide\.md を読んでいるが、CI では文書だけの変更/);
  assert.match(errors[2], /:7: docs\/manual\/images\/a\.png を読んでいる/);
});
