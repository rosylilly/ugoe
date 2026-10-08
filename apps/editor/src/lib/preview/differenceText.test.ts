// 着せ替えの違いの種類と引数を、今の言語の文にする (T1677)。すべての種類を日英で見る。
import { setLocale } from "@ugoe/i18n";
import { afterEach, describe, expect, it } from "vitest";
import type { DifferenceLine, ElementChange } from "../ipc";
import { describeChange, describeItem, describeLine } from "./differenceText";

afterEach(() => setLocale("en"));

/** 行の種類と引数・英語の文・日本語の文。 */
const LINES: [DifferenceLine, string, string][] = [
  [{ kind: "name", before: "Hat", after: "Cap" }, "Name: Hat → Cap", "名前: Hat → Cap"],
  [{ kind: "slot", before: "Hair", after: "Hat" }, "Slot: Hair → Hat", "スロット: Hair → Hat"],
  [
    {
      kind: "elements",
      what: "mesh",
      before: 2,
      after: 1,
      added: [],
      removed: ["Ears"],
      changed: ["Band"],
    },
    "Meshes: 2 → 1 (removed: Ears; changed: Band)",
    "メッシュ: 2 → 1 (除いた: Ears; 変えた: Band)",
  ],
  [
    {
      kind: "elements",
      what: "pose_group",
      before: 1,
      after: 2,
      added: ["A / B", "C / D"],
      removed: [],
      changed: [],
    },
    "Pose groups: 1 → 2 (added: A / B and C / D)",
    "ポーズのグループ: 1 → 2 (足した: A / B、C / D)",
  ],
  [
    { kind: "textures", before: 2, after: 3, added: ["a", "b", "c"], removed: [], changed: ["d"] },
    "Textures: 2 → 3 (added: a, b, and c; different image: d)",
    "テクスチャ: 2 → 3 (足した: a、b、c; 画像が違う: d)",
  ],
  [
    { kind: "avatar_level", before: null, after: 2 },
    "Required avatar level: none → 2",
    "条件のアバタープロファイルのレベル: なし → 2",
  ],
  [
    { kind: "names", what: "required_slots", added: ["Outfit"], removed: ["Hat"] },
    "Required slots: added: Outfit; removed: Hat",
    "条件のスロット: 足した: Outfit; 除いた: Hat",
  ],
  [
    { kind: "names", what: "required_parameters", added: ["ParamA"], removed: [] },
    "Required parameters: added: ParamA",
    "条件のパラメータ: 足した: ParamA",
  ],
  [
    { kind: "names", what: "parameters_used", added: [], removed: ["ParamB"] },
    "Parameters used: removed: ParamB",
    "使うパラメータ: 除いた: ParamB",
  ],
  [{ kind: "pose_order" }, "Pose: order of groups changed", "ポーズ: グループの並びが違います"],
  [
    { kind: "unmatched_atlas" },
    "Some meshes in the file's texture atlases could not be matched to your current textures, so they count as changed",
    "ファイルのテクスチャのアトラスに、今の編集のテクスチャと対応づけられないメッシュがあるので、変わったものとして数えます",
  ],
];

/** 要素の違いの種類と引数・英語の文・日本語の文。 */
const CHANGES: [ElementChange, string, string][] = [
  [{ kind: "name", before: "Sway", after: "Swing" }, "name Sway → Swing", "名前 Sway → Swing"],
  [{ kind: "part", before: "Hair", after: "Hat" }, "part Hair → Hat", "パーツ Hair → Hat"],
  [{ kind: "parent", before: null, after: "Root" }, "parent none → Root", "親 なし → Root"],
  [
    { kind: "type", before: "warp", after: "rotation" },
    "type warp → rotation",
    "種類 ワープ → 回転",
  ],
  [
    { kind: "grid", before: { cols: 3, rows: 2 }, after: { cols: 4, rows: 5 } },
    "grid 3×2 → 4×5",
    "格子 3×2 → 4×5",
  ],
  [
    { kind: "texture", before: "hair", after: null },
    "texture hair → none",
    "テクスチャ hair → なし",
  ],
  [
    { kind: "count", what: "vertices", before: 12, after: 14 },
    "vertices 12 → 14",
    "頂点の数 12 → 14",
  ],
  [
    { kind: "count", what: "keyforms", before: 1, after: 2 },
    "keyforms 1 → 2",
    "キーフォームの数 1 → 2",
  ],
  [
    { kind: "count", what: "triangles", before: 2, after: 4 },
    "triangles 2 → 4",
    "三角形の数 2 → 4",
  ],
  [
    { kind: "count", what: "vertex_pairs", before: 1, after: 3 },
    "vertex pairs 1 → 3",
    "頂点の組の数 1 → 3",
  ],
  [{ kind: "count", what: "particles", before: 3, after: 4 }, "particles 3 → 4", "粒子の数 3 → 4"],
  [{ kind: "count", what: "inputs", before: 1, after: 2 }, "inputs 1 → 2", "入力の数 1 → 2"],
  [{ kind: "count", what: "outputs", before: 2, after: 1 }, "outputs 2 → 1", "出力の数 2 → 1"],
  [{ kind: "changed", what: "uvs", count: 1 }, "UVs of 1 vertex", "1 個の頂点の UV"],
  [{ kind: "changed", what: "uvs", count: 4 }, "UVs of 4 vertices", "4 個の頂点の UV"],
  [{ kind: "changed", what: "vertex_pairs", count: 1 }, "1 pair of vertices", "1 組の頂点"],
  [{ kind: "changed", what: "vertex_pairs", count: 2 }, "2 pairs of vertices", "2 組の頂点"],
  [{ kind: "changed", what: "particles", count: 1 }, "settings of 1 particle", "1 個の粒子の設定"],
  [{ kind: "changed", what: "particles", count: 3 }, "settings of 3 particles", "3 個の粒子の設定"],
  [{ kind: "changed", what: "inputs", count: 1 }, "1 input", "1 個の入力"],
  [{ kind: "changed", what: "outputs", count: 2 }, "2 outputs", "2 個の出力"],
  [
    { kind: "in_keyforms", what: "opacity", count: 1 },
    "opacity in 1 keyform",
    "1 個のキーフォームの不透明度",
  ],
  [
    { kind: "in_keyforms", what: "control_points", count: 2 },
    "control points in 2 keyforms",
    "2 個のキーフォームの制御点",
  ],
  [
    { kind: "in_keyforms", what: "origin_or_angle", count: 1 },
    "origin or angle in 1 keyform",
    "1 個のキーフォームの原点か角度",
  ],
  [
    { kind: "in_keyforms", what: "intensity", count: 1 },
    "intensity in 1 keyform",
    "1 個のキーフォームの強さ",
  ],
  [
    { kind: "in_keyforms", what: "vertex_positions", count: 3 },
    "vertex positions in 3 keyforms",
    "3 個のキーフォームの頂点の位置",
  ],
  [
    { kind: "in_keyforms", what: "draw_order", count: 2 },
    "draw order in 2 keyforms",
    "2 個のキーフォームの描画順",
  ],
  [
    { kind: "in_keyforms", what: "opacity_or_colors", count: 1 },
    "opacity or colors in 1 keyform",
    "1 個のキーフォームの不透明度か色",
  ],
  [
    { kind: "draw_order", before: 1000, after: 520.25 },
    "draw order 1000 → 520.25",
    "描画順 1000 → 520.25",
  ],
  [
    { kind: "glue_meshes", before_a: "A", before_b: "B", after_a: "A", after_b: "C" },
    "meshes A + B → A + C",
    "両端のメッシュ A + B → A + C",
  ],
  [
    { kind: "colliders", added: [], removed: ["Collar"] },
    "colliders (removed: Collar)",
    "衝突体 (除いた: Collar)",
  ],
  [{ kind: "frame", before: "Head", after: null }, "frame Head → none", "座標系の基準 Head → なし"],
  [
    { kind: "attach", before: null, after: "Neck" },
    "attached to none → Neck",
    "取り付け先 なし → Neck",
  ],
  [
    { kind: "shape", before: "circle", after: "capsule" },
    "shape circle → capsule",
    "形 円 → カプセル",
  ],
  [{ kind: "radius", before: 8, after: 12.5 }, "radius 8 → 12.5", "半径 8 → 12.5"],
  [
    { kind: "entries", before: "HatA / HatB", after: "HatB / HatA" },
    "entries HatA / HatB → HatB / HatA",
    "要素 HatA / HatB → HatB / HatA",
  ],
  [
    { kind: "links", entry: null, added: ["HatA"], removed: ["Sleeve"] },
    "links (added: HatA; removed: Sleeve)",
    "リンク (足した: HatA; 除いた: Sleeve)",
  ],
  [
    { kind: "links", entry: "HatA", added: ["Bow"], removed: [] },
    "links of HatA (added: Bow)",
    "HatA のリンク (足した: Bow)",
  ],
  [{ kind: "links_reordered", entry: null }, "links reordered", "リンクの並び"],
  [{ kind: "links_reordered", entry: "HatA" }, "links of HatA reordered", "HatA のリンクの並び"],
  [{ kind: "keyform_parameters" }, "keyform parameters", "キーフォームのパラメータ"],
  [{ kind: "blend_shapes" }, "blend shapes", "ブレンドシェイプ"],
  [{ kind: "keyform_settings" }, "keyform settings", "キーフォームの設定"],
  [{ kind: "triangles" }, "triangles", "三角形"],
  [{ kind: "normalization" }, "normalization", "正規化"],
  [{ kind: "position" }, "position", "位置"],
  [{ kind: "blending_or_masks" }, "blending or masks", "合成かマスク"],
  [{ kind: "other_settings" }, "other settings", "ほかの設定"],
];

describe("difference text (T1677)", () => {
  it.each(LINES)("describes the line %j", (line, en, ja) => {
    expect(describeLine(line)).toBe(en);
    setLocale("ja");
    expect(describeLine(line)).toBe(ja);
  });

  it.each(CHANGES)("describes the change %j", (change, en, ja) => {
    expect(describeChange(change)).toBe(en);
    setLocale("ja");
    expect(describeChange(change)).toBe(ja);
  });

  it("covers every kind", () => {
    const lineKinds = new Set(LINES.map(([l]) => l.kind));
    expect(lineKinds.size).toBe(8);
    const changeKinds = new Set(CHANGES.map(([c]) => c.kind));
    expect(changeKinds.size).toBe(27);
  });

  it("joins the changes of an element, and names added and removed elements", () => {
    const changes: ElementChange[] = [
      { kind: "changed", what: "uvs", count: 1 },
      { kind: "draw_order", before: 500, after: 490 },
    ];
    expect(describeItem({ state: "changed", changes })).toBe(
      "UVs of 1 vertex; draw order 500 → 490",
    );
    expect(describeItem({ state: "added", changes: [] })).toBe("added");
    expect(describeItem({ state: "removed", changes: [] })).toBe("removed");
    setLocale("ja");
    expect(describeItem({ state: "changed", changes })).toBe("1 個の頂点の UV; 描画順 500 → 490");
    expect(describeItem({ state: "added", changes: [] })).toBe("足した");
    expect(describeItem({ state: "removed", changes: [] })).toBe("除いた");
  });
});
