// 着せ替えの違い (ugoe_editor_core::compare_with_module_file の種類と引数) を、今の言語の文にする
// (docs/editor.md#違いの種類と引数。T1677。differenceText.test.ts)。
import { formatList, formatNumber } from "@ugoe/i18n";
import { t } from "../i18n";
import type { DifferenceItem, DifferenceLine, ElementChange } from "../ipc";

/** 値の無いもの (親なし・テクスチャなし・レベルなし) は「none」。 */
function orNone(value: string | number | null): string {
  return value === null ? t("preview.difference.change.none") : String(value);
}

/**
 * 表示用の数 (小数 2 桁まで。バックエンドも 2 桁に丸めて送る)。桁の区切りは付けない (ツリーやインスペクタの描画順と
 * 同じ書き方にする)。
 */
function number(value: number): string {
  return formatNumber(value, { maximumFractionDigits: 2, useGrouping: false });
}

/** 足した・除いた・変えた (`changed` の見出しは `changedKey`) ID の並び。空のものは書かない。 */
function details(
  added: readonly string[],
  removed: readonly string[],
  changed: readonly string[] = [],
  changedKey: "changed" | "differentImage" = "changed",
): string {
  const parts: string[] = [];
  if (added.length > 0)
    parts.push(t("preview.difference.details.added", { ids: formatList(added) }));
  if (removed.length > 0) {
    parts.push(t("preview.difference.details.removed", { ids: formatList(removed) }));
  }
  if (changed.length > 0) {
    parts.push(t(`preview.difference.details.${changedKey}`, { ids: formatList(changed) }));
  }
  return parts.join(t("preview.difference.details.separator"));
}

/** 違いの 1 行の文。 */
export function describeLine(line: DifferenceLine): string {
  switch (line.kind) {
    case "name":
      return t("preview.difference.line.name", { before: line.before, after: line.after });
    case "slot":
      return t("preview.difference.line.slot", { before: line.before, after: line.after });
    case "elements":
      return t("preview.difference.line.elements", {
        what: t(`preview.difference.elements.${line.what}`),
        before: line.before,
        after: line.after,
        details: details(line.added, line.removed, line.changed),
      });
    case "textures":
      return t("preview.difference.line.textures", {
        before: line.before,
        after: line.after,
        details: details(line.added, line.removed, line.changed, "differentImage"),
      });
    case "avatar_level":
      return t("preview.difference.line.avatarLevel", {
        before: orNone(line.before),
        after: orNone(line.after),
      });
    case "names":
      return t("preview.difference.line.names", {
        what: t(`preview.difference.names.${line.what}`),
        details: details(line.added, line.removed),
      });
    case "pose_order":
      return t("preview.difference.line.poseOrder");
    case "unmatched_atlas":
      return t("preview.difference.line.unmatchedAtlas");
  }
}

/** 要素の中身の違い 1 つの文。 */
export function describeChange(change: ElementChange): string {
  switch (change.kind) {
    case "name":
    case "part":
    case "entries":
      return t(`preview.difference.change.${change.kind}`, {
        before: change.before,
        after: change.after,
      });
    case "parent":
    case "texture":
    case "frame":
    case "attach":
      return t(`preview.difference.change.${change.kind}`, {
        before: orNone(change.before),
        after: orNone(change.after),
      });
    case "type":
      return t("preview.difference.change.type", {
        before: t(`preview.difference.change.deformerType.${change.before}`),
        after: t(`preview.difference.change.deformerType.${change.after}`),
      });
    case "shape":
      return t("preview.difference.change.shape", {
        before: t(`preview.difference.change.shapeType.${change.before}`),
        after: t(`preview.difference.change.shapeType.${change.after}`),
      });
    case "grid":
      return t("preview.difference.change.grid", {
        beforeCols: change.before.cols,
        beforeRows: change.before.rows,
        afterCols: change.after.cols,
        afterRows: change.after.rows,
      });
    case "count":
      return t("preview.difference.change.count", {
        what: t(`preview.difference.change.counted.${change.what}`),
        before: change.before,
        after: change.after,
      });
    case "changed":
      return t(`preview.difference.change.changed.${change.what}`, { count: change.count });
    case "in_keyforms":
      return t("preview.difference.change.inKeyforms", {
        what: t(`preview.difference.change.keyformValue.${change.what}`),
        count: change.count,
      });
    case "draw_order":
    case "radius":
      return t(
        `preview.difference.change.${change.kind === "draw_order" ? "drawOrder" : "radius"}`,
        {
          before: number(change.before),
          after: number(change.after),
        },
      );
    case "glue_meshes":
      return t("preview.difference.change.glueMeshes", {
        beforeA: change.before_a,
        beforeB: change.before_b,
        afterA: change.after_a,
        afterB: change.after_b,
      });
    case "colliders":
      return t("preview.difference.change.colliders", {
        details: details(change.added, change.removed),
      });
    case "links": {
      const text = details(change.added, change.removed);
      return change.entry === null
        ? t("preview.difference.change.links", { details: text })
        : t("preview.difference.change.linksOf", { entry: change.entry, details: text });
    }
    case "links_reordered":
      return change.entry === null
        ? t("preview.difference.change.linksReordered")
        : t("preview.difference.change.linksOfReordered", { entry: change.entry });
    case "keyform_parameters":
    case "blend_shapes":
    case "keyform_settings":
    case "triangles":
    case "normalization":
    case "position":
    case "blending_or_masks":
    case "other_settings":
      return t(`preview.difference.change.${change.kind}`);
  }
}

/** 要素の行の「何が違うか」の文 (足した・除いた要素は「added」・「removed」)。 */
export function describeItem(item: Pick<DifferenceItem, "state" | "changes">): string {
  switch (item.state) {
    case "added":
      return t("preview.difference.state.added");
    case "removed":
      return t("preview.difference.state.removed");
    case "changed":
      return item.changes.map(describeChange).join(t("preview.difference.change.separator"));
  }
}
