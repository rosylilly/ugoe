// 描画の例 (spec/examples/README.md#rendering-examples) の読み込みと、期待値との比較。

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { readModelJson } from "./bundle.ts";
import { evaluate, parameterValues } from "./evaluate.ts";
import { rgbaPng } from "./png.ts";
import { type Image, type Lights, render } from "./render.ts";
import type * as M from "./types.ts";
import { writeBundle } from "./write.ts";

export interface TextureSpec extends Image {
  normal_map?: Image;
}

export interface ExpectedPixel {
  at: [number, number];
  rgba: [number, number, number, number];
}

export interface RenderCase {
  summary: string;
  parameters?: Record<string, number>;
  target: [number, number];
  lights?: Lights;
  pixels: ExpectedPixel[];
}

export interface RenderDocument {
  model: string;
  summary: string;
  textures: TextureSpec[];
  cases: RenderCase[];
}

export interface RenderExample {
  model: M.Model;
  doc: RenderDocument;
  warnings: string[];
}

/** 許容誤差: 乗算済みの 0〜255 の尺度で各チャンネル 3 (spec/rendering.md#conformance)。 */
export const PIXEL_TOLERANCE = 3;

/** 調べる画素の中心は、三角形の辺から描画先の画素で少なくともこれだけ離れている。 */
export const EDGE_CLEARANCE = 1;

export function loadRenderExample(dir: string): RenderExample {
  const doc = JSON.parse(readFileSync(join(dir, "render.json"), "utf8")) as RenderDocument;
  const { model, warnings } = readModelJson(readFileSync(join(dir, doc.model), "utf8"));
  return { model, doc, warnings };
}

/** 1 つのケースを描いて、画素ごとの食い違いの説明の一覧を返す (空なら一致)。 */
export function compareRenderCase(example: RenderExample, c: RenderCase): string[] {
  const { model, doc } = example;
  const result = evaluate(model, parameterValues(model, c.parameters ?? {}));
  const [width, height] = c.target;
  const image = render({
    model,
    result,
    textures: doc.textures,
    normalMaps: doc.textures.map((t) => t.normal_map ?? null),
    width,
    height,
    lights: c.lights,
  });
  const failures: string[] = [];
  for (const p of c.pixels) {
    const at = 4 * (p.at[1] * width + p.at[0]);
    const got = [0, 1, 2, 3].map((q) => image[at + q] * 255);
    if (p.rgba.some((e, q) => !(Math.abs(e - got[q]) <= PIXEL_TOLERANCE))) {
      failures.push(`pixel ${p.at}: expected ${p.rgba}, got ${got.map((x) => x.toFixed(2))}`);
    }
  }
  return failures;
}

/**
 * 調べる画素の中心から、すべてのメッシュ (描くものもマスクだけのものも) の三角形の辺までの距離
 * (描画先の画素) が `EDGE_CLEARANCE` 未満のものを返す。例がラスタライズの境目に頼らないことの検査。
 */
export function pixelsNearEdges(example: RenderExample, c: RenderCase): string[] {
  const { model } = example;
  const result = evaluate(model, parameterValues(model, c.parameters ?? {}));
  const sx = c.target[0] / model.meta.canvas.width;
  const sy = c.target[1] / model.meta.canvas.height;
  const near: string[] = [];
  for (const p of c.pixels) {
    const center: M.Vec2 = [p.at[0] + 0.5, p.at[1] + 0.5];
    let best = Number.POSITIVE_INFINITY;
    model.meshes.forEach((m, i) => {
      const pos = result.meshes[i].positions.map(([x, y]): M.Vec2 => [x * sx, y * sy]);
      for (let t = 0; t + 2 < m.indices.length; t += 3) {
        const tri = [pos[m.indices[t]], pos[m.indices[t + 1]], pos[m.indices[t + 2]]];
        for (let e = 0; e < 3; e++)
          best = Math.min(best, segmentDistance(center, tri[e], tri[(e + 1) % 3]));
      }
    });
    if (best < EDGE_CLEARANCE) near.push(`pixel ${p.at} is ${best.toFixed(3)} px from an edge`);
  }
  return near;
}

function segmentDistance(p: M.Vec2, a: M.Vec2, b: M.Vec2): number {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const len2 = dx * dx + dy * dy;
  const t =
    len2 > 0 ? Math.min(Math.max(((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2, 0), 1) : 0;
  return Math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy);
}

/** 例のモデルとテクスチャ (と法線マップ) の `.ugoe` (参照実装に描かせるため)。 */
export function renderExampleBundle(example: RenderExample): Uint8Array {
  const png = (i: Image) => rgbaPng(i.width, i.height, i.rgba);
  return writeBundle({
    model: example.model,
    textures: example.doc.textures.map(png),
    normalMaps: example.doc.textures.flatMap((t) => (t.normal_map ? [png(t.normal_map)] : [])),
  });
}
