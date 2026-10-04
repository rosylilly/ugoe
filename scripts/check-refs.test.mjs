// scripts/check-doc-refs.mjs と scripts/check-ffi-bindings.mjs のテスト (`just refs`)
import assert from "node:assert/strict";
import { test } from "node:test";
import { hasAnchor, refs } from "./check-doc-refs.mjs";
import { compare, csharpFunctions, headerFunctions } from "./check-ffi-bindings.mjs";

test("根からのパスの参照とアンカーを集め、相対パスと URL の一部は除く", () => {
  const text = [
    "// 仕様 (docs/spec/model.md#検証)。`spec/model.md#model-validation` も見る",
    "// ../editor.md#x と https://example.com/docs/a.md#b と crates/x/docs/c.md は除く",
    "/// docs/editor.md、docs/spec/evaluation.md#3-デフォーマ。",
  ].join("\n");
  assert.deepEqual(
    refs(text).map((r) => `${r.line}:${r.path}#${r.anchor ?? ""}`),
    [
      "1:docs/spec/model.md#検証",
      "1:spec/model.md#model-validation",
      "3:docs/editor.md#",
      "3:docs/spec/evaluation.md#3-デフォーマ",
    ],
  );
});

test("見出しの文字のまま書いたアンカーも、GitHub の規則で崩して比べる", () => {
  const anchors = new Set(["id", "10-の基準の機能", "画面構成-m2-の目標"]);
  assert.ok(hasAnchor(anchors, "ID"));
  assert.ok(hasAnchor(anchors, "1.0-の基準の機能"));
  assert.ok(!hasAnchor(anchors, "画面構成"));
});

test("ヘッダの宣言だけを集め、コメントの中の名前は数えない", () => {
  const header = [
    "// `ugoe_puppet_update(puppet, dt)` を呼ぶ。",
    "UgoeResult ugoe_puppet_update(struct UgoePuppet *puppet, float dt);",
    "uint32_t ugoe_abi_version(void);",
  ].join("\n");
  assert.deepEqual([...headerFunctions(header)].sort(), ["ugoe_abi_version", "ugoe_puppet_update"]);
});

test("C# の DllImport の宣言を EntryPoint を含めて集める", () => {
  const cs = [
    "[DllImport(Library)] internal static extern uint ugoe_abi_version();",
    '[DllImport(Library, EntryPoint = "ugoe_puppet_update")]',
    "internal static extern int Update(PuppetHandle puppet, float dt);",
    "internal static int Helper() => 0;",
  ].join("\n");
  assert.deepEqual([...csharpFunctions(cs)].sort(), ["ugoe_abi_version", "ugoe_puppet_update"]);
});

test("漏れ・余り・不要な除外を誤りにする", () => {
  const binding = { name: "b", file: "B.cs", omit: { ugoe_c: "理由", ugoe_gone: "理由" } };
  const errors = compare(
    new Set(["ugoe_a", "ugoe_b", "ugoe_c"]),
    new Set(["ugoe_a", "ugoe_c", "ugoe_old"]),
    binding,
  );
  assert.deepEqual(errors, [
    "b: ヘッダの ugoe_b が B.cs に宣言されていない",
    "b: ugoe_c は宣言されているので OMIT から外す",
    "b: B.cs の ugoe_old がヘッダに無い",
    "b: OMIT の ugoe_gone がヘッダに無い",
  ]);
});
