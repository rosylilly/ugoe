// アバタープロファイルの編集 (docs/editor.md#アバタープロファイルの編集) の補助。DOM や IPC に依存しない
// 純粋な関数だけを置く (avatar.test.ts で検査する)。値の検査はコマンド (ugoe_editor_core::avatar) と
// 準拠の検査 (check_avatar_profile) が行う。
import type {
  AvatarHitArea,
  AvatarMeta,
  AvatarPermission,
  Command,
  CommercialUsage,
  CreditNotation,
  Modification,
} from "../../../generated/types";

/** 準拠を主張できるレベルの上限 (ugoe_model::avatar::MAX_LEVEL)。 */
export const MAX_LEVEL = 3;

/** レベルの名前 (docs/spec/avatar-profile.md#準拠レベル)。 */
export const LEVEL_NAMES: Record<number, string> = {
  1: "Basic",
  2: "Standard",
  3: "Expressive",
};

/** 標準の当たり判定の名前 (ugoe_model::avatar::HIT_AREA_HEAD / HIT_AREA_BODY)。 */
export const STANDARD_HIT_AREAS = ["Head", "Body"] as const;

/** 選択肢。先頭が最も制限の強い値 (既定値)。 */
export const AVATAR_PERMISSIONS: { value: AvatarPermission; label: string }[] = [
  { value: "only_author", label: "Only the author" },
  { value: "only_licensed", label: "Only licensed people" },
  { value: "everyone", label: "Everyone" },
];

export const COMMERCIAL_USAGES: { value: CommercialUsage; label: string }[] = [
  { value: "personal_non_profit", label: "Personal, non-profit" },
  { value: "personal_profit", label: "Personal, for profit" },
  { value: "corporation", label: "Corporations too" },
];

export const CREDIT_NOTATIONS: { value: CreditNotation; label: string }[] = [
  { value: "required", label: "Required" },
  { value: "unnecessary", label: "Not needed" },
];

export const MODIFICATIONS: { value: Modification; label: string }[] = [
  { value: "prohibited", label: "Prohibited" },
  { value: "allow", label: "Allowed" },
  { value: "allow_redistribution", label: "Allowed, redistributable" },
];

/** 真偽値の許諾 (既定はどれも false = 許さない)。 */
export const USAGE_FLAGS: { key: UsageFlag; label: string }[] = [
  { key: "allow_violent_usage", label: "Excessively violent content" },
  { key: "allow_sexual_usage", label: "Excessively sexual content" },
  { key: "allow_political_or_religious_usage", label: "Political or religious purposes" },
  { key: "allow_antisocial_or_hate_usage", label: "Antisocial or hateful purposes" },
  { key: "allow_redistribution", label: "Redistribution" },
];

export type UsageFlag =
  | "allow_violent_usage"
  | "allow_sexual_usage"
  | "allow_political_or_religious_usage"
  | "allow_antisocial_or_hate_usage"
  | "allow_redistribution";

/** 省略できる文字列のフィールド (空にすると null)。 */
export type OptionalText =
  | "version"
  | "copyright"
  | "contact"
  | "license_url"
  | "third_party_licenses";

/** 1 行に 1 つの一覧 (作者・原作) を読む。前後の空白を除き、空行は捨てる。 */
export function parseLines(text: string): string[] {
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

/** 省略できる文字列を読む。空白だけなら null。 */
export function optionalText(text: string): string | null {
  const trimmed = text.trim();
  return trimmed.length > 0 ? trimmed : null;
}

/**
 * プロファイルを作るコマンド。許諾はコマンドの既定値 (最も制限の強い値) のままにし、
 * 名前だけモデルの名前で埋める (空なら埋めない)。
 */
export function createCommand(modelName: string): Command {
  const name = modelName.trim();
  if (name.length === 0) return { op: "update_avatar_profile" };
  return { op: "update_avatar_profile", meta: { ...defaultMeta(), name } };
}

/** 最も制限の強いメタデータ (ugoe_model::avatar::AvatarMeta::default と同じ)。 */
export function defaultMeta(): AvatarMeta {
  return {
    name: "",
    version: null,
    authors: [],
    copyright: null,
    contact: null,
    references: [],
    thumbnail: null,
    license_url: null,
    avatar_permission: "only_author",
    commercial_usage: "personal_non_profit",
    allow_violent_usage: false,
    allow_sexual_usage: false,
    allow_political_or_religious_usage: false,
    allow_antisocial_or_hate_usage: false,
    credit_notation: "required",
    allow_redistribution: false,
    modification: "prohibited",
    third_party_licenses: null,
  };
}

/** メタデータの一部を変えるコマンド。変わらなければ null (Undo の履歴を増やさない)。 */
export function metaCommand(meta: AvatarMeta, patch: Partial<AvatarMeta>): Command | null {
  const next = { ...meta, ...patch };
  const changed = (Object.keys(patch) as (keyof AvatarMeta)[]).some(
    (key) => JSON.stringify(meta[key]) !== JSON.stringify(next[key]),
  );
  return changed ? { op: "update_avatar_profile", meta: next } : null;
}

/**
 * 標準の当たり判定 `name` を当たり判定 `hitArea` に対応させるコマンド (`null` で外す)。
 * 並びは STANDARD_HIT_AREAS の順、それ以外の名前はその後に元の順で残す。
 */
export function hitAreaCommand(
  current: AvatarHitArea[],
  name: string,
  hitArea: string | null,
): Command | null {
  const before = current.find((a) => a.name === name)?.hit_area ?? null;
  if (before === hitArea) return null;
  const rest = current.filter((a) => a.name !== name);
  const next = hitArea === null ? rest : [...rest, { name, hit_area: hitArea }];
  const rank = (a: AvatarHitArea) => {
    const i = (STANDARD_HIT_AREAS as readonly string[]).indexOf(a.name);
    return i < 0 ? STANDARD_HIT_AREAS.length : i;
  };
  // Array.prototype.sort は安定なので、標準以外の名前は元の順のまま。
  next.sort((a, b) => rank(a) - rank(b));
  return { op: "update_avatar_profile", hit_areas: next };
}

/** 準拠の検査の結果の見出し (名乗るレベルと満たすレベル)。 */
export function checkSummary(check: {
  claimed: number | null;
  satisfied: number;
  errors: string[];
}): { ok: boolean; text: string } {
  const satisfied = `Satisfies ${levelLabel(check.satisfied)}.`;
  if (check.claimed === null) {
    return {
      ok: false,
      text: `No avatar profile: the model does not conform to any level. ${satisfied}`,
    };
  }
  const claimed = levelLabel(check.claimed);
  if (check.errors.length === 0) {
    return { ok: true, text: `Conforms to ${claimed}. ${satisfied}` };
  }
  const count = check.errors.length === 1 ? "1 error" : `${check.errors.length} errors`;
  return { ok: false, text: `Claims ${claimed} but has ${count}. ${satisfied}` };
}

function levelLabel(level: number): string {
  return level === 0 ? "no level" : `level ${level} (${LEVEL_NAMES[level] ?? "?"})`;
}
