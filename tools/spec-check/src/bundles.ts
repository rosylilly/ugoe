// ファイルの例 (spec/examples/bundles/、spec/examples/README.md#file-examples) の目録の読み込み、
// 目録からのファイルの書き出し、読んだ結果と目録の比較。

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { isDeepStrictEqual } from "node:util";
import {
  type Bundle,
  type Image,
  type ModuleFile,
  ReadError,
  type ReadErrorKind,
  readBundle,
  readChunkTable,
  readJsonDocument,
  readModelJson,
  readModuleFile,
  readModuleJson,
} from "./bundle.ts";
import * as D from "./decode.ts";
import type { MsgValue } from "./msgpack.ts";
import { decodeRgbaPng, solidPng } from "./png.ts";
import type * as M from "./types.ts";
import {
  type ChunkSpec,
  EXPLICIT,
  type FeatureDecl,
  MINIMAL,
  writeBundle,
  writeModuleFile,
} from "./write.ts";

type Rgba = [number, number, number, number];

export type ErrorStep =
  | "header"
  | "chunk_table"
  | "feature_declarations"
  | "schema_version"
  | "chunks"
  | "counts"
  | "validation";

/** この実装の読み込みの誤りの種類と、手順名の対応。 */
const STEP_OF_KIND: Record<ReadErrorKind, ErrorStep> = {
  header: "header",
  container_version: "header",
  chunk_table: "chunk_table",
  features: "feature_declarations",
  unsupported_features: "feature_declarations",
  schema_version: "schema_version",
  chunks: "chunks",
  decode: "chunks",
  counts: "counts",
  validation: "validation",
};

export interface ImageSpec {
  width: number;
  height: number;
  rgba: Rgba;
}

export interface FileEntry {
  file: string;
  summary: string;
  style: "minimal" | "explicit";
  chunks: { tag: string; compressed?: boolean; payload?: string }[];
  payload_order?: number[];
  features?: FeatureDecl[];
  result: "ok" | "error";
  /** 拒否する手順 (spec/bundle.md#reading の手順名)。 */
  error?: ErrorStep;
  unknown_required_features?: string[];
  warnings?: { unknown_features: string[] };
  model?: string;
  physics?: string;
  motions?: string[];
  expressions?: string[];
  module?: string;
  meta?: unknown;
  textures: (ImageSpec & { normal_map?: ImageSpec })[];
}

export interface Manifest {
  summary: string;
  files: FileEntry[];
}

export function loadManifest(dir: string): Manifest {
  return JSON.parse(readFileSync(join(dir, "bundles.json"), "utf8")) as Manifest;
}

/** 目録が指す JSON の文書を読んだデータ (ファイルを読んだ結果と同じになるはずのもの)。 */
export interface ExpectedData {
  model: M.Model | null;
  physics: M.PhysicsSettings | null;
  motions: M.Motion[];
  expressions: M.Expression[];
  module: M.Module | null;
}

function noWarnings<T extends { warnings: string[] }>(path: string, r: T): T {
  if (r.warnings.length > 0) throw new Error(`${path}: ${r.warnings.join("; ")}`);
  return r;
}

export function expectedData(dir: string, e: FileEntry): ExpectedData {
  const text = (f: string) => readFileSync(join(dir, f), "utf8");
  const doc = <T>(f: string, conv: (c: D.Ctx, v: MsgValue) => T): T =>
    noWarnings(f, readJsonDocument(text(f), conv)).value;
  return {
    model: e.model ? noWarnings(e.model, readModelJson(text(e.model))).model : null,
    physics: e.physics ? doc(e.physics, D.physicsSettings) : null,
    motions: (e.motions ?? []).map((f) => doc(f, D.motion)),
    expressions: (e.expressions ?? []).map((f) => doc(f, D.expression)),
    module: e.module ? noWarnings(e.module, readModuleJson(text(e.module))).module : null,
  };
}

const png = (s: ImageSpec) => solidPng(s.width, s.height, s.rgba);

/** 目録の項目から、ファイルを書く (この実装の書き出しで)。 */
export function buildFile(dir: string, e: FileEntry): Uint8Array {
  const data = expectedData(dir, e);
  const known = new Set(["FEAT", "MODL", "MODU", "PHYS", "MOTN", "EXPR", "META", "TEXR", "NRML"]);
  const extraChunks: ChunkSpec[] = e.chunks
    .filter((c) => !known.has(c.tag))
    .map((c) => ({
      tag: c.tag,
      data: new TextEncoder().encode(c.payload ?? ""),
      // 知らないチャンクの中身はそのまま置き、flags だけを立てる
      flags: c.compressed ? 1 : 0,
    }));
  const options = {
    style: e.style === "explicit" ? EXPLICIT : MINIMAL,
    compress: [
      ...new Set(e.chunks.filter((c) => c.compressed && known.has(c.tag)).map((c) => c.tag)),
    ],
    features: e.features ?? [],
    extraChunks,
    payloadOrder: e.payload_order,
  };
  const textures = e.textures.map(png);
  const normalMaps = e.textures.flatMap((t) => (t.normal_map ? [png(t.normal_map)] : []));
  if (data.module) {
    return writeModuleFile({ module: data.module, meta: e.meta, textures, normalMaps }, options);
  }
  if (!data.model) throw new Error(`${e.file}: neither model nor module`);
  return writeBundle(
    {
      model: data.model,
      physics: data.physics,
      motions: data.motions,
      expressions: data.expressions,
      meta: e.meta,
      textures,
      normalMaps,
    },
    options,
  );
}

function compareImage(where: string, image: Image, spec: ImageSpec): string[] {
  if (image.format !== "png") return [`${where}: expected a PNG`];
  const { width, height, pixels } = decodeRgbaPng(image.bytes);
  if (width !== spec.width || height !== spec.height) {
    return [`${where}: size ${width}x${height}, expected ${spec.width}x${spec.height}`];
  }
  for (let i = 0; i < pixels.length; i += 4) {
    if (!isDeepStrictEqual([...pixels.subarray(i, i + 4)], spec.rgba)) {
      return [
        `${where}: pixel ${i / 4} is ${[...pixels.subarray(i, i + 4)]}, expected ${spec.rgba}`,
      ];
    }
  }
  return [];
}

/** ファイルを読み、目録の項目と食い違うところを返す。 */
export function checkFile(dir: string, e: FileEntry, bytes: Uint8Array): string[] {
  const failures: string[] = [];
  const fail = (m: string) => failures.push(`${e.file}: ${m}`);

  const table = readChunkTable(bytes).map((c) => ({
    tag: c.tag,
    compressed: (c.flags & 1) === 1,
  }));
  const expectedTable = e.chunks.map((c) => ({ tag: c.tag, compressed: c.compressed ?? false }));
  if (!isDeepStrictEqual(table, expectedTable)) {
    fail(`chunk table ${JSON.stringify(table)}, expected ${JSON.stringify(expectedTable)}`);
  }

  const isModule = e.file.endsWith(".ugoemod");
  let read: Bundle | ModuleFile;
  try {
    read = isModule ? readModuleFile(bytes) : readBundle(bytes);
  } catch (err) {
    if (e.result === "error" && err instanceof ReadError) {
      const step = STEP_OF_KIND[err.kind];
      if (step !== e.error) fail(`refused at ${step}, expected ${e.error}: ${err.message}`);
      else if (err.kind === "unsupported_features") {
        const got = [...err.details].sort();
        const want = [...(e.unknown_required_features ?? [])].sort();
        if (!isDeepStrictEqual(got, want)) fail(`refused for ${got}, expected ${want}`);
      }
      return failures;
    }
    fail(`not read: ${(err as Error).message}`);
    return failures;
  }
  if (e.result === "error") {
    fail("read, but it should be refused");
    return failures;
  }

  const expected = expectedData(dir, e);
  if ("module" in read) {
    if (!isDeepStrictEqual(read.module, expected.module)) fail("module differs from the JSON");
  } else {
    if (!isDeepStrictEqual(read.model, expected.model)) fail("model differs from the JSON");
    if (!isDeepStrictEqual(read.physics, expected.physics)) fail("physics differ from the JSON");
    if (!isDeepStrictEqual(read.motions, expected.motions)) fail("motions differ from the JSON");
    if (!isDeepStrictEqual(read.expressions, expected.expressions)) {
      fail("expressions differ from the JSON");
    }
  }
  if (!isDeepStrictEqual(read.meta, e.meta ?? null)) fail(`META ${JSON.stringify(read.meta)}`);

  const specs = e.textures;
  if (read.textures.length !== specs.length) fail(`${read.textures.length} textures`);
  read.textures.forEach((t, i) => {
    if (specs[i]) failures.push(...compareImage(`${e.file}: texture ${i}`, t, specs[i]));
  });
  const normalSpecs = specs.flatMap((t) => (t.normal_map ? [t.normal_map] : []));
  if (read.normalMaps.length !== normalSpecs.length) fail(`${read.normalMaps.length} normal maps`);
  read.normalMaps.forEach((t, i) => {
    if (normalSpecs[i])
      failures.push(...compareImage(`${e.file}: normal map ${i}`, t, normalSpecs[i]));
  });

  const unknownFeatures = read.warnings
    .map((w) => /^unknown optional feature "(.*)"$/.exec(w)?.[1])
    .filter((x) => x !== undefined);
  const otherWarnings = read.warnings.filter((w) => !w.startsWith("unknown optional feature"));
  if (!isDeepStrictEqual(unknownFeatures, e.warnings?.unknown_features ?? [])) {
    fail(`unknown features reported: ${unknownFeatures}`);
  }
  if (otherWarnings.length > 0) fail(`warnings: ${otherWarnings.join("; ")}`);
  return failures;
}
