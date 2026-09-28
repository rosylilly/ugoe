// 物理のテンプレート (docs/editor.md#物理のテンプレート) の、DOM に依存しない補助。templates.test.ts で検査する。
// リグの中身は Rust の ugoe_editor_core::physics_template が作る。ここでは画面に出す説明と、
// 足りないパラメータ・首のデフォーマの候補を求めるだけ。
import type { Command, Model, PhysicsBlend, PhysicsTemplate } from "../../ipc";

export interface TemplateInfo {
  template: PhysicsTemplate;
  label: string;
  title: string;
  /** 使う標準パラメータ (なければ作るか、それを使うリグを作らない)。 */
  parameters: string[];
  /** 作るリグの ID。 */
  rigs: string[];
}

export const TEMPLATES: TemplateInfo[] = [
  {
    template: "body_follow",
    label: "Body follow",
    title:
      "The body (BodyAngleX/Y/Z) follows the head (AngleX/Y/Z) late, overshoots a little and settles",
    parameters: ["AngleX", "BodyAngleX", "AngleY", "BodyAngleY", "AngleZ", "BodyAngleZ"],
    rigs: ["BodyFollowX", "BodyFollowY", "BodyFollowZ"],
  },
  {
    template: "bounce",
    label: "Bounce",
    title:
      "Turning the head swings the relay parameter BodyBounce, and a second pendulum adds the bounce to BodyAngleX",
    parameters: ["AngleX", "BodyAngleX"],
    rigs: ["BodyBounce", "BodyBounceAngle"],
  },
  {
    template: "neck_follow",
    label: "Neck follow",
    title: "A neck rotation deformer tilts with the head (AngleZ, AngleX) late",
    parameters: ["AngleZ", "AngleX"],
    rigs: ["NeckFollow"],
  },
];

/** テンプレートが使う標準パラメータのうち、モデルにないもの。 */
export function missingParameters(model: Model, template: PhysicsTemplate): string[] {
  const have = new Set(model.parameters.map((p) => p.id));
  const info = TEMPLATES.find((t) => t.template === template);
  return info?.parameters.filter((id) => !have.has(id)) ?? [];
}

/** 首の追従で傾けられる回転デフォーマ。 */
export function neckCandidates(model: Model): { id: string; name: string }[] {
  return model.deformers.filter((d) => d.kind.type === "rotation");
}

function isNeckName(name: string): boolean {
  return name.toLowerCase().includes("neck") || name.includes("首");
}

/** 既定の首のデフォーマ (ID か名前に neck か「首」を含む最初の回転デフォーマ。physics_template::default_neck と同じ)。 */
export function defaultNeck(model: Model): string | null {
  return neckCandidates(model).find((d) => isNeckName(d.id) || isNeckName(d.name))?.id ?? null;
}

export function templateCommand(
  template: PhysicsTemplate,
  createMissing: boolean,
  blend: PhysicsBlend,
  neck: string | null,
): Command {
  return {
    op: "apply_physics_template",
    template,
    options: {
      create_missing: createMissing,
      blend,
      ...(template === "neck_follow" && neck ? { neck } : {}),
    },
  };
}
