// `.ugoe` と `.ugoemod` のコンテナの読み込み (spec/bundle.md#reading)、JSON の文書の読み込み
// (spec/bundle.md#standalone-json-files)。

import { zstdDecompressSync } from "node:zlib";
import * as D from "./decode.ts";
import { decode, type MsgValue, peekKey } from "./msgpack.ts";
import type * as M from "./types.ts";
import { validateModel, validateModule, validatePhysics } from "./validate.ts";

/** この実装が読めるコンテナの版。 */
export const SUPPORTED_CONTAINER_VERSION = 0;

/** 1.0 の基準の機能 (spec/bundle.md#baseline-features)。この実装はすべてを知っているとみなす。 */
export const KNOWN_FEATURES = new Set([
  "authoring.keyform_origins",
  "mask.intersection",
  "physics.dynamic",
  "lighting.normal_map",
  "modules",
  "avatar.profile",
  "deform.blend_shape",
  "deform.smooth_interpolation",
  "physics.additive_output",
]);

/** 読み込みの失敗の種類。テストで理由を確かめるために分ける。 */
export type ReadErrorKind =
  | "header"
  | "container_version"
  | "chunk_table"
  | "features"
  | "unsupported_features"
  | "schema_version"
  | "chunks"
  | "decode"
  | "counts"
  | "validation";

export class ReadError extends Error {
  readonly kind: ReadErrorKind;
  readonly details: string[];
  constructor(kind: ReadErrorKind, message: string, details: string[] = []) {
    super(`${kind}: ${message}${details.length ? `\n  ${details.join("\n  ")}` : ""}`);
    this.kind = kind;
    this.details = details;
  }
}

export interface Chunk {
  tag: string;
  flags: number;
  offset: bigint;
  length: bigint;
}

export interface Image {
  format: "png" | "ktx2";
  bytes: Uint8Array;
}

export interface Bundle {
  model: M.Model;
  physics: M.PhysicsSettings | null;
  motions: M.Motion[];
  expressions: M.Expression[];
  meta: unknown;
  textures: Image[];
  normalMaps: Image[];
  /** 知らない任意の機能と知らない値の場所 (spec/bundle.md#reading の手順 8)。 */
  warnings: string[];
}

export interface ModuleFile {
  module: M.Module;
  meta: unknown;
  textures: Image[];
  normalMaps: Image[];
  warnings: string[];
}

const HEADER = 16;
const ENTRY = 24;
const MAX_DECOMPRESSED = 268_435_456;

/** 各チャンクがバンドル・モジュールのファイルに現れてよい数。`null` は許されない。 */
const ALLOWED: Record<
  string,
  { bundle: "one" | "at_most_one" | "any" | null; module: "one" | "at_most_one" | "any" | null }
> = {
  FEAT: { bundle: "at_most_one", module: "at_most_one" },
  MODL: { bundle: "one", module: null },
  MODU: { bundle: null, module: "one" },
  PHYS: { bundle: "at_most_one", module: null },
  MOTN: { bundle: "any", module: null },
  EXPR: { bundle: "any", module: null },
  META: { bundle: "at_most_one", module: "at_most_one" },
  TEXR: { bundle: "any", module: "any" },
  NRML: { bundle: "any", module: "any" },
};

const PNG = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
const KTX2 = [0xab, 0x4b, 0x54, 0x58, 0x20, 0x32, 0x30, 0xbb, 0x0d, 0x0a, 0x1a, 0x0a];

const startsWith = (bytes: Uint8Array, sig: number[]): boolean =>
  bytes.length >= sig.length && sig.every((b, i) => bytes[i] === b);

/** 手順 1・2: ヘッダとチャンクの表。 */
export function readChunkTable(bytes: Uint8Array): Chunk[] {
  if (bytes.length < HEADER) throw new ReadError("header", "file shorter than 16 bytes");
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (!startsWith(bytes, [0x55, 0x47, 0x4f, 0x45])) throw new ReadError("header", "bad magic");
  const version = view.getUint32(4, true);
  if (version !== SUPPORTED_CONTAINER_VERSION) {
    throw new ReadError("container_version", `unsupported container version ${version}`);
  }
  if (view.getUint32(8, true) !== 0) throw new ReadError("header", "header flags are not 0");
  const count = view.getUint32(12, true);
  const tableEnd = BigInt(HEADER) + BigInt(count) * BigInt(ENTRY);
  const size = BigInt(bytes.length);
  if (tableEnd > size) throw new ReadError("chunk_table", "chunk table does not fit in the file");
  const chunks: Chunk[] = [];
  for (let i = 0; i < count; i++) {
    const at = HEADER + i * ENTRY;
    const tag = String.fromCharCode(...bytes.subarray(at, at + 4));
    const flags = view.getUint32(at + 4, true);
    const offset = view.getBigUint64(at + 8, true);
    const length = view.getBigUint64(at + 16, true);
    const where = `entry ${i} (${JSON.stringify(tag)})`;
    if ((flags & ~1) !== 0) throw new ReadError("chunk_table", `${where}: reserved flag bits set`);
    if (offset % 16n !== 0n) throw new ReadError("chunk_table", `${where}: offset not aligned`);
    // BigInt なので桁あふれしない
    if (offset < tableEnd || offset + length > size) {
      throw new ReadError("chunk_table", `${where}: payload outside the file or over the table`);
    }
    chunks.push({ tag, flags, offset, length });
  }
  const sorted = chunks.filter((c) => c.length > 0n).sort((a, b) => (a.offset < b.offset ? -1 : 1));
  for (let i = 1; i < sorted.length; i++) {
    if (sorted[i - 1].offset + sorted[i - 1].length > sorted[i].offset) {
      throw new ReadError("chunk_table", "payloads overlap");
    }
  }
  return chunks;
}

/** チャンクのペイロード (圧縮されていれば展開したもの)。 */
function payload(bytes: Uint8Array, chunk: Chunk): Uint8Array {
  const raw = bytes.subarray(Number(chunk.offset), Number(chunk.offset + chunk.length));
  if ((chunk.flags & 1) === 0) return raw;
  try {
    const out = zstdDecompressSync(raw, { maxOutputLength: MAX_DECOMPRESSED });
    return new Uint8Array(out.buffer, out.byteOffset, out.byteLength);
  } catch (e) {
    throw new ReadError(
      "decode",
      `${chunk.tag}: zstd decompression failed: ${(e as Error).message}`,
    );
  }
}

function decodeMsgpack(bytes: Uint8Array, tag: string): MsgValue {
  try {
    return decode(bytes);
  } catch (e) {
    throw new ReadError("decode", `${tag}: ${(e as Error).message}`);
  }
}

/** 型の決まったデータにする。`DecodeError` と `UnsupportedSchemaVersion` を読み込みの誤りにする。 */
function typed<T>(f: () => T): T {
  try {
    return f();
  } catch (e) {
    if (e instanceof D.UnsupportedSchemaVersion) throw new ReadError("schema_version", e.message);
    if (e instanceof D.DecodeError) throw new ReadError("decode", e.message);
    throw e;
  }
}

/** 手順 3: 機能の宣言 (spec/bundle.md#reading-feature-declarations)。 */
function checkFeatures(bytes: Uint8Array, chunks: Chunk[], warnings: string[]): void {
  const feats = chunks.filter((c) => c.tag === "FEAT");
  if (feats.length > 1) throw new ReadError("features", "more than one FEAT chunk");
  if (feats.length === 0) return;
  const value = decodeMsgpack(payload(bytes, feats[0]), "FEAT");
  const decls = typed(() => {
    const c = new D.Ctx([], "FEAT");
    const o = D.object(c, value);
    return D.opt(
      c,
      o,
      "features",
      D.array((ic, iv) => {
        const io = D.object(ic, iv);
        return {
          id: D.req(ic, io, "id", D.str),
          required: D.opt(ic, io, "required", D.bool, false),
        };
      }),
      [],
    );
  });
  const unknown = decls.filter((d) => !KNOWN_FEATURES.has(d.id));
  const required = [...new Set(unknown.filter((d) => d.required).map((d) => d.id))];
  if (required.length > 0) {
    throw new ReadError("unsupported_features", "unknown required features", required);
  }
  for (const id of new Set(unknown.map((d) => d.id))) {
    warnings.push(`unknown optional feature ${JSON.stringify(id)}`);
  }
}

/** 手順 4: 本体の `schema_version` を、ほかを復号する前に読む。 */
function checkSchemaVersion(bytes: Uint8Array, chunks: Chunk[]): void {
  const main = chunks.find((c) => c.tag === "MODL" || c.tag === "MODU");
  if (!main) return;
  const version = peekKey(payload(bytes, main), "schema_version");
  if (typeof version === "bigint" && version > BigInt(D.SUPPORTED_SCHEMA_VERSION)) {
    throw new ReadError("schema_version", `schema_version ${version} is newer than supported`);
  }
}

type Kind = "bundle" | "module";

/** 手順 5・6: チャンクを歩き、許されない・重複したものを拒否し、画像を集める。 */
function walk(bytes: Uint8Array, chunks: Chunk[], kind: Kind) {
  const counts = new Map<string, number>();
  const payloads = new Map<string, Uint8Array[]>();
  for (const chunk of chunks) {
    const rule = ALLOWED[chunk.tag];
    if (!rule) continue; // 知らないチャンクは展開も復号もしない
    const allowed = rule[kind];
    if (allowed === null) throw new ReadError("chunks", `${chunk.tag} is not allowed in a ${kind}`);
    const n = (counts.get(chunk.tag) ?? 0) + 1;
    counts.set(chunk.tag, n);
    if ((allowed === "one" || allowed === "at_most_one") && n > 1) {
      throw new ReadError("chunks", `more than one ${chunk.tag}`);
    }
    if (chunk.tag === "FEAT") continue; // 手順 3 で読んだ
    const list = payloads.get(chunk.tag) ?? [];
    list.push(payload(bytes, chunk));
    payloads.set(chunk.tag, list);
  }
  const main = kind === "bundle" ? "MODL" : "MODU";
  if (!payloads.has(main)) throw new ReadError("counts", `missing ${main} chunk`);
  const images = (tag: string): Image[] =>
    (payloads.get(tag) ?? []).map((b, i) => {
      if (startsWith(b, PNG)) return { format: "png", bytes: b };
      if (startsWith(b, KTX2)) return { format: "ktx2", bytes: b };
      throw new ReadError("counts", `${tag} ${i} is neither PNG nor KTX2`);
    });
  const meta = payloads.get("META")?.[0];
  let metaValue: unknown = null;
  if (meta) {
    try {
      metaValue = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(meta));
    } catch (e) {
      throw new ReadError("decode", `META: ${(e as Error).message}`);
    }
  }
  return { payloads, textures: images("TEXR"), normalMaps: images("NRML"), meta: metaValue };
}

function checkImageCounts(
  textures: M.TextureRef[],
  images: { textures: Image[]; normalMaps: Image[] },
) {
  if (images.textures.length !== textures.length) {
    throw new ReadError(
      "counts",
      `${images.textures.length} TEXR chunks for ${textures.length} textures`,
    );
  }
  const withNormal = textures.filter((t) => t.normal_map !== null).length;
  if (images.normalMaps.length !== withNormal) {
    throw new ReadError(
      "counts",
      `${images.normalMaps.length} NRML chunks for ${withNormal} normal maps`,
    );
  }
}

/** `.ugoe` を読む。 */
export function readBundle(bytes: Uint8Array): Bundle {
  const chunks = readChunkTable(bytes);
  const warnings: string[] = [];
  checkFeatures(bytes, chunks, warnings);
  checkSchemaVersion(bytes, chunks);
  const walked = walk(bytes, chunks, "bundle");
  const p = walked.payloads;
  const [modl] = p.get("MODL") ?? [];
  const model = typed(() => D.model(new D.Ctx(warnings, "MODL"), decodeMsgpack(modl, "MODL")));
  const phys = p.get("PHYS")?.[0];
  const physics = phys
    ? typed(() => D.physicsSettings(new D.Ctx(warnings, "PHYS"), decodeMsgpack(phys, "PHYS")))
    : null;
  const motions = (p.get("MOTN") ?? []).map((b, i) =>
    typed(() => D.motion(new D.Ctx(warnings, `MOTN[${i}]`), decodeMsgpack(b, "MOTN"))),
  );
  const expressions = (p.get("EXPR") ?? []).map((b, i) =>
    typed(() => D.expression(new D.Ctx(warnings, `EXPR[${i}]`), decodeMsgpack(b, "EXPR"))),
  );
  checkImageCounts(model.textures, walked);
  const errors = validateModel(model);
  if (physics) errors.push(...validatePhysics(model, physics));
  if (errors.length > 0) throw new ReadError("validation", "invalid model", errors);
  return {
    model,
    physics,
    motions,
    expressions,
    meta: walked.meta,
    textures: walked.textures,
    normalMaps: walked.normalMaps,
    warnings,
  };
}

/** `.ugoemod` を読む。 */
export function readModuleFile(bytes: Uint8Array): ModuleFile {
  const chunks = readChunkTable(bytes);
  const warnings: string[] = [];
  checkFeatures(bytes, chunks, warnings);
  checkSchemaVersion(bytes, chunks);
  const walked = walk(bytes, chunks, "module");
  const [modu] = walked.payloads.get("MODU") ?? [];
  const module = typed(() => D.module(new D.Ctx(warnings, "MODU"), decodeMsgpack(modu, "MODU")));
  checkImageCounts(module.textures, walked);
  const errors = validateModule(module);
  if (errors.length > 0) throw new ReadError("validation", "invalid module", errors);
  return {
    module,
    meta: walked.meta,
    textures: walked.textures,
    normalMaps: walked.normalMaps,
    warnings,
  };
}

/** JSON のモデルの文書を読み、検証する。 */
export function readModelJson(text: string): { model: M.Model; warnings: string[] } {
  const warnings: string[] = [];
  const model = typed(() => D.model(new D.Ctx(warnings), D.parseJson(text)));
  const errors = validateModel(model);
  if (errors.length > 0) throw new ReadError("validation", "invalid model", errors);
  return { model, warnings };
}
