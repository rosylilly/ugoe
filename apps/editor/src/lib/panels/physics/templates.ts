// 物理のテンプレート (docs/editor.md#物理のテンプレート) の、DOM に依存しない補助。templates.test.ts で検査する。
// リグの中身は Rust の ugoe_editor_core::physics_template が作る。ここでは画面に出す説明と、
// 足りないパラメータ・首のデフォーマの候補を求めるだけ。
import { t } from "../../i18n";
import type { Command, Model, PhysicsBlend, PhysicsTemplate } from "../../ipc";

export interface TemplateInfo {
  template: PhysicsTemplate;
  /** 名前と説明 (読むたびに t() を通す)。 */
  readonly label: string;
  readonly title: string;
  /** 使う標準パラメータ (なければ作るか、それを使うリグを作らない)。 */
  parameters: string[];
  /** 作るリグの ID。 */
  rigs: string[];
  /** なければ `create_missing` によらず作る中継パラメータ (モジュールのプロジェクトでは作れない)。 */
  relays: string[];
}

export const TEMPLATES: TemplateInfo[] = [
  {
    template: "body_follow",
    get label() {
      return t("physics.templates.items.bodyFollow.label");
    },
    get title() {
      return t("physics.templates.items.bodyFollow.tooltip");
    },
    parameters: ["AngleX", "BodyAngleX", "AngleY", "BodyAngleY", "AngleZ", "BodyAngleZ"],
    rigs: ["BodyFollowX", "BodyFollowY", "BodyFollowZ"],
    relays: [],
  },
  {
    template: "bounce",
    get label() {
      return t("physics.templates.items.bounce.label");
    },
    get title() {
      return t("physics.templates.items.bounce.tooltip");
    },
    parameters: ["AngleX", "BodyAngleX"],
    rigs: ["BodyBounce", "BodyBounceAngle"],
    relays: ["BodyBounce"],
  },
  {
    template: "neck_follow",
    get label() {
      return t("physics.templates.items.neckFollow.label");
    },
    get title() {
      return t("physics.templates.items.neckFollow.tooltip");
    },
    parameters: ["AngleZ", "AngleX"],
    rigs: ["NeckFollow"],
    relays: [],
  },
];

/** テンプレートが使う標準パラメータのうち、モデルにないもの。 */
export function missingParameters(model: Model, template: PhysicsTemplate): string[] {
  const have = new Set(model.parameters.map((p) => p.id));
  const info = TEMPLATES.find((t) => t.template === template);
  return info?.parameters.filter((id) => !have.has(id)) ?? [];
}

/** テンプレートが必ず作る中継パラメータのうち、モデルにないもの (モジュールのプロジェクトでは作れないので使えない)。 */
export function missingRelays(model: Model, template: PhysicsTemplate): string[] {
  const have = new Set(model.parameters.map((p) => p.id));
  const info = TEMPLATES.find((t) => t.template === template);
  return info?.relays.filter((id) => !have.has(id)) ?? [];
}

/** 首の追従で傾けられる回転デフォーマ。 */
export function neckCandidates(model: Model): { id: string; name: string }[] {
  return model.deformers.filter((d) => d.kind.type === "rotation");
}

function isNeckName(name: string): boolean {
  // i18n-ignore: 表示しない。名前から首を探す語 (physics_template::default_neck と同じ)
  return name.toLowerCase().includes("neck") || name.includes("首");
}

/**
 * 既定の首のデフォーマ (ID か名前に neck か「首」を含む最初の回転デフォーマ。physics_template::default_neck と同じ)。
 * `candidates` を渡すとその中から選ぶ (モジュールのプロジェクトで本体のデフォーマを除いたもの)。
 */
export function defaultNeck(
  model: Model,
  candidates: readonly { id: string; name: string }[] = neckCandidates(model),
): string | null {
  return candidates.find((d) => isNeckName(d.id) || isNeckName(d.name))?.id ?? null;
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
