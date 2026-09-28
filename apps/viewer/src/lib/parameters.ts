// パラメータの定義 (`WebPuppet.parameters_json()`) と、スライダーの一覧のためのグループ分け。
// 定義の形はモデルの JSON 形式と同じ (docs/spec/model.md)。

export interface ParameterDef {
  id: string;
  name: string;
  min: number;
  max: number;
  default: number;
  kind?: string;
  mirror?: string | null;
  group?: string | null;
  /** 中継用 (内部) のパラメータ。スライダーの一覧では既定で隠す (docs/spec/model.md#パラメータ)。 */
  internal?: boolean;
}

export interface ParameterGroupDef {
  id: string;
  name: string;
}

export interface ParameterList {
  parameters: ParameterDef[];
  groups: ParameterGroupDef[];
}

/** スライダーの一覧の 1 区切り。`id` が `null` のものはグループに属さないパラメータ。 */
export interface ParameterSection {
  id: string | null;
  name: string;
  parameters: ParameterDef[];
}

export function parseParameterList(json: string): ParameterList {
  const value = JSON.parse(json) as Partial<ParameterList>;
  return { parameters: value.parameters ?? [], groups: value.groups ?? [] };
}

/**
 * グループの定義順に区切り、各区切りの中はモデルの定義順に並べる。空のグループは出さない。
 * グループの無いパラメータ (と、定義に無いグループを指すもの) は最後の区切りにまとめる。
 */
export function groupParameters(list: ParameterList, ungroupedName: string): ParameterSection[] {
  const sections = new Map<string, ParameterSection>(
    list.groups.map((g) => [g.id, { id: g.id, name: g.name || g.id, parameters: [] }]),
  );
  const rest: ParameterSection = { id: null, name: ungroupedName, parameters: [] };
  for (const p of list.parameters) {
    const section = p.group == null ? undefined : sections.get(p.group);
    (section ?? rest).parameters.push(p);
  }
  return [...sections.values(), rest].filter((s) => s.parameters.length > 0);
}

/**
 * スライダーの一覧に出す区切り。`query` (前後の空白を除いて空でなければ) を ID か名前に含むパラメータだけを残し、
 * `showInternal` が偽なら中継用のパラメータを隠す。空になった区切りは出さない。
 */
export function visibleSections(
  sections: readonly ParameterSection[],
  query: string,
  showInternal: boolean,
): ParameterSection[] {
  const q = query.trim().toLowerCase();
  return sections
    .map((s) => ({
      ...s,
      parameters: s.parameters.filter(
        (p) =>
          (showInternal || !p.internal) &&
          (q === "" || p.id.toLowerCase().includes(q) || p.name.toLowerCase().includes(q)),
      ),
    }))
    .filter((s) => s.parameters.length > 0);
}

/** 中継用のパラメータの数。 */
export function internalCount(sections: readonly ParameterSection[]): number {
  return sections.reduce((n, s) => n + s.parameters.filter((p) => p.internal).length, 0);
}

/** スライダーの刻み。値域を 1000 等分した程度の、きりのよい大きさ。 */
export function sliderStep(p: Pick<ParameterDef, "min" | "max">): number {
  const range = p.max - p.min;
  if (!(range > 0)) return 0.001;
  return 10 ** Math.floor(Math.log10(range / 1000));
}

/** 表示用の値 (小数 3 桁まで、-0 は 0)。 */
export function formatValue(value: number): string {
  const rounded = Math.round(value * 1000) / 1000;
  return (Object.is(rounded, -0) ? 0 : rounded).toString();
}
