// エディタの偽のホストが返す、見本のプロジェクトに無い状態 (T0676): AI の提案・分類・送る前の確認・着せ替えの
// モジュール・モジュールのプロジェクトと違いの一覧。形は apps/editor/src/lib/generated/types.ts に合わせる。
// 中身は画面を撮るための作りもので、バックエンドの答えと同じである必要はない (行の長さ・数・種類が混ざることを優先)。
// ただし、コマンドの種類 (`op`) と分類のラベルは本物にあるものだけを使う (無い種類は画面が利用者の言葉で出せず、本物では
// 出ない出し方を撮ることになる。T0047)。`hosts.test.mjs` が crates/ugoe-editor-core/src/command.rs と
// crates/ugoe-ai/src/classify.rs に照らして確かめる。

/** 提案の元にした PSD (見本のプロジェクトの中にあることにする)。 */
export const FIXTURE_PSD =
  "/home/audit/ugoe-dress-up-sample/sources/bustup-with-a-long-file-name.psd";

/** 分類の結果 (確かめたほうがよいレイヤー・直したレイヤー・名前との食い違い・差分の組を含む)。 */
export function classificationFixture() {
  const layer = (path, label, side, confidence, extra = {}) => ({
    path,
    label,
    side,
    confidence,
    needs_review: false,
    variant: null,
    name_disagreement: null,
    corrected: false,
    ...extra,
  });
  return {
    psd: FIXTURE_PSD,
    by_ai: true,
    instructions: "The ribbon is neckwear.",
    layers: [
      layer(["Head", "Face"], "face", "center", 0.98),
      layer(["Head", "Eye R", "White"], "eye_white", "right", 0.52, {
        needs_review: true,
        name_disagreement: { label: null, side: "left" },
      }),
      layer(["Head", "Eye L", "White"], "eye_white", "left", 0.95),
      layer(["Head", "Mouth"], "mouth", "center", 0.61, { needs_review: true }),
      layer(["Head", "Hair", "Front"], "hair_front", "center", 1, { corrected: true }),
      layer(["Body", "Sleeve A"], "topwear", "center", 0.9, { variant: 0 }),
      layer(["Body", "Sleeve B"], "topwear", "center", 0.9, { variant: 0 }),
    ],
    // crates/ugoe-ai/src/classify.rs の `Label::ALL` (宣言順)
    label_choices: [
      "face",
      "ear",
      "nose",
      "cheek",
      "neck",
      "eye",
      "eye_white",
      "iris",
      "eye_highlight",
      "eyelash_upper",
      "eyelash_lower",
      "eye_closed",
      "eyebrow",
      "mouth",
      "mouth_open",
      "mouth_closed",
      "mouth_inner",
      "teeth",
      "tongue",
      "hair_front",
      "hair_side",
      "hair_back",
      "body",
      "arm",
      "hand",
      "leg",
      "foot",
      "tail",
      "topwear",
      "bottomwear",
      "legwear",
      "footwear",
      "handwear",
      "neckwear",
      "headwear",
      "earwear",
      "eyewear",
      "accessory",
      "background",
      "other",
    ],
  };
}

const command = (op, subject = null, requires = [], extra = {}) => ({
  op,
  subject,
  requires,
  ...extra,
});

/** 提案の一覧 (エディタのリグ・エージェントのリグと添えた分類の直し・古くなった法線マップ)。 */
export function proposalsFixture() {
  // crates/ugoe-ai/src/rig.rs の形: ワープを作り、格子の軸にキーを足して (既定値のキーを先に)、セルを置く
  const face = "deformer `FaceWarp`";
  const rig = [
    command("add_parameter", "AngleX"),
    command("add_parameter", "AngleY"),
    command("add_parameter", "EyeLOpen"),
    command("add_parameter", "EyeROpen"),
    command("add_parameter", "MouthOpenY"),
    command("create_warp_deformer", "FaceWarp"),
    command("add_keyform_key", face, [0, 5]),
    command("add_keyform_key", face, [0, 5, 6]),
    command("add_keyform_key", face, [1, 5, 7]),
    command("add_keyform_key", face, [1, 5, 8]),
    command("set_keyform_cell", face, [9]),
    command("set_keyform_cell", face, [10]),
    command("add_keyform_key", "mesh `EyeL`", [2]),
    command("set_keyform_cell", "mesh `EyeL`", [12]),
    command("add_keyform_key", "mesh `EyeR`", [3]),
    command("set_keyform_cell", "mesh `EyeR`", [14]),
    command("add_keyform_key", "mesh `Mouth`", [4]),
    command("set_keyform_cell", "mesh `Mouth`", [16]),
  ];
  // エージェントが残したラベル (分類の直し。一覧に `Head / Mouth → mouth (center)` と出す)
  const keep = (subject, label, side) =>
    command("set_layer_correction", subject, [], { correction: { label, side } });
  const agent = [
    keep("Head / Eye R / White", "eye_white", "right"),
    keep("Head / Mouth", "mouth", "center"),
    keep("Body / Sleeve A", "topwear", "center"),
    ...rig.map((c) => ({ ...c, requires: c.requires.map((i) => i + 3) })),
  ];
  return [
    {
      id: 1,
      title: "Rig the face (head turn, eye blinks, mouth)",
      rationale:
        "Head turn: the face is a warp with 3 × 3 keys over AngleX and AngleY (±30°).\nEye blinks: each eye closes along its lid (EyeLOpen, EyeROpen).\nMouth: one mouth layer opens downward (MouthOpenY).\nTo review: Head / Eye R / White (the AI says right, the name says left).",
      commands: rig,
      stale: false,
      source: null,
      revises: null,
      instructions: "Keep head turns subtle. Close the eyes a little narrower.",
      unapplied: ["Make the hair bounce when the head turns"],
    },
    {
      id: 2,
      title: "Rig from the agent's classification",
      rationale:
        "The agent classified the layers, kept its labels for three layers, and built the same face rig from them.",
      commands: agent,
      stale: false,
      source: "Claude Code",
      revises: null,
      instructions: null,
      unapplied: [],
    },
    {
      id: 3,
      title: "Normal maps from the shapes",
      rationale: "Face (3 textures) and body (2 textures) bulge as one surface each.",
      commands: [
        command("set_normal_map", "Face"),
        command("set_normal_map", "EyeL"),
        command("set_normal_map", "EyeR"),
        command("set_normal_map", "Body"),
        command("set_normal_map", "Shirt"),
        command("set_lighting", null),
      ],
      stale: true,
      source: null,
      revises: null,
      instructions: null,
      unapplied: [],
    },
  ];
}

/** `get_proposals` の答え。`restoring` は分類のキャッシュを戻している最中か (T0676)。 */
export function proposalStateFixture({ restoring = false } = {}) {
  return {
    proposals: proposalsFixture(),
    psd: FIXTURE_PSD,
    classification: classificationFixture(),
    regenerable: [1],
    sections: [
      {
        id: 3,
        sections: [
          { label: "face", indices: [0, 1, 2] },
          { label: "body", indices: [3, 4] },
          { label: "lighting", indices: [5] },
        ],
      },
    ],
    stale_rigs: [],
    restoring,
  };
}

/** 送る前の確認に示す内容 (`needs_consent` の答え)。 */
export function disclosureFixture() {
  return {
    service: "Anthropic Claude API",
    endpoint: "https://api.anthropic.com/v1/messages",
    model: "claude-opus-5-5",
    image_count: 24,
    image_bytes: 1_843_200,
    text_bytes: 6_480,
    instruction_chars: 38,
    retries: 0,
  };
}

/** AI の設定 (`ai_settings`)。`ready` なら外部に送れる (キーを保存し、プロジェクトが送ってよい)。 */
export function aiSettingsFixture({ ready = false } = {}) {
  return {
    provider: {
      provider: "claude",
      claude: { model: "claude-opus-5-5" },
      openai: { base_url: "http://localhost:11434/v1", model: "", output: "json_schema" },
      time_limit_minutes: 10,
    },
    service: "Anthropic Claude",
    local: false,
    key: ready ? "stored" : "not_stored",
    key_required: true,
    key_env: "ANTHROPIC_API_KEY",
    env_key: false,
    send_externally: ready,
    consent_remembered: false,
  };
}

/** モジュールのプロジェクトの状態 (`module_status`)。本体は見本、スロットは `Outfit`。 */
export function moduleStatusFixture({ name, base, model }) {
  const ids = (list) => (list ?? []).map((e) => e.id);
  const slots = (model.slots ?? []).map((s) => ({
    id: s.id,
    anchor: s.anchor ?? null,
    part: s.part ?? null,
    draw_order_min: s.draw_order?.min ?? 0,
    draw_order_max: s.draw_order?.max ?? 1000,
    replaces: s.replaces ?? [],
    parameters: s.parameters ?? [],
    pose_parts: s.pose_parts ?? [],
  }));
  return {
    name,
    base,
    slot: slots.find((s) => s.id === "Outfit") ?? slots[0],
    base_parts: ids(model.parts),
    base_deformers: ids(model.deformers),
    base_meshes: ids(model.meshes),
    base_textures: (model.textures ?? []).map((t) => t.name ?? t.id ?? ""),
    base_rigs: [],
    base_colliders: [],
    unavailable_colliders: [],
    requires: { avatar_level: null, slots: [], parameters: [] },
    base_slots: slots,
    base_parameters: ids(model.parameters),
    base_avatar_level: model.avatar?.level ?? null,
    base_revision: 1,
    base_pose_groups: model.pose?.groups?.length ?? 0,
  };
}

/**
 * 今の編集と書き出したファイルの違い (`compare_preview_module`。種類と引数で、画面が文にする。T1677)。長い一覧 (要素の行が多い) で、
 * 送りと絞り込みを撮る。
 */
export function differencesFixture() {
  const item = (kind, id, changes, element = `Sailor/${id}`) => ({
    kind,
    id,
    state: changes === "added" || changes === "removed" ? changes : "changed",
    changes: Array.isArray(changes) ? changes : [],
    element,
    meshes: kind === "mesh" ? [id] : [],
    parts: [],
  });
  const elements = (what, before, after, ids) => ({
    kind: "elements",
    what,
    before,
    after,
    added: ids.added ?? [],
    removed: ids.removed ?? [],
    changed: ids.changed ?? [],
  });
  const meshes = Array.from({ length: 14 }, (_, i) =>
    item(
      "mesh",
      `Ribbon${String(i + 1).padStart(2, "0")}`,
      i % 3 === 0
        ? [
            { kind: "count", what: "vertices", before: 12, after: 14 },
            { kind: "in_keyforms", what: "vertex_positions", count: 2 },
            { kind: "draw_order", before: 300, after: 310 },
          ]
        : i % 3 === 1
          ? "added"
          : [
              { kind: "texture", before: "Sailor", after: "SailorRibbon" },
              { kind: "in_keyforms", what: "opacity_or_colors", count: 3 },
            ],
    ),
  );
  const ids = (from) => meshes.filter((_, i) => i % 3 === from).map((m) => m.id);
  return [
    {
      line: elements("mesh", 14, 25, { added: ids(1), changed: [...ids(0), ...ids(2)] }),
      items: meshes,
    },
    {
      line: elements("deformer", 3, 4, { added: ["CollarWarp"], changed: ["WaveRotation"] }),
      items: [
        item("deformer", "CollarWarp", "added"),
        item("deformer", "WaveRotation", [
          { kind: "count", what: "keyforms", before: 1, after: 2 },
          { kind: "keyform_settings" },
        ]),
      ],
    },
    {
      line: elements("part", 2, 2, { changed: ["WaveSleeve"] }),
      items: [item("part", "WaveSleeve", [{ kind: "in_keyforms", what: "opacity", count: 1 }])],
    },
    {
      line: elements("physics_rig", 0, 1, { added: ["RibbonSway"] }),
      items: [item("physics_rig", "RibbonSway", "added", "Sailor/RibbonSway")],
    },
    {
      line: elements("pose_group", 1, 1, { changed: ["Sleeve A / Sleeve B"] }),
      items: [
        item(
          "pose_group",
          "Sleeve A / Sleeve B",
          [{ kind: "entries", before: "Sleeve A / Sleeve B", after: "Sleeve B / Sleeve A" }],
          null,
        ),
      ],
    },
    {
      line: {
        kind: "textures",
        before: 2,
        after: 3,
        added: ["SailorRibbon"],
        removed: [],
        changed: ["Sailor"],
      },
      items: [],
    },
    { line: { kind: "avatar_level", before: null, after: 1 }, items: [] },
  ];
}

/**
 * `.ugoe`・`.ugoemod` のコンテナ (docs/spec/file-formats.md の「コンテナ」) の、モジュールの名前 (`MODU` の最上位の `name`) を
 * `name` に書き換えたもの。モジュールのプロジェクトのプレビューは、今の編集を `<名前> (editing)` で読み込む
 * (apps/editor/src/lib/preview/preview.ts の `editingName`) ので、見本の `.ugoemod` からその名前のものを作る。
 */
export function renameModule(bytes, name) {
  const buf = Buffer.from(bytes);
  if (buf.toString("latin1", 0, 4) !== "UGOE") throw new Error("not a ugoe container");
  const count = buf.readUInt32LE(12);
  const chunks = [];
  for (let i = 0; i < count; i++) {
    const at = 16 + i * 24;
    const offset = Number(buf.readBigUInt64LE(at + 8));
    const length = Number(buf.readBigUInt64LE(at + 16));
    chunks.push({
      tag: buf.subarray(at, at + 4),
      flags: buf.readUInt32LE(at + 4),
      data: buf.subarray(offset, offset + length),
    });
  }
  const modu = chunks.find((c) => c.tag.toString("latin1") === "MODU");
  if (!modu || modu.flags & 1) throw new Error("no uncompressed MODU chunk");
  // 最上位のマップの `name` の値 (fixstr か str8)
  const key = Buffer.from([0xa4, ...Buffer.from("name")]);
  const at = modu.data.indexOf(key);
  if (at < 0) throw new Error("no name in MODU");
  const start = at + key.length;
  const head = modu.data[start];
  let end;
  if (head >= 0xa0 && head <= 0xbf) end = start + 1 + (head - 0xa0);
  else if (head === 0xd9) end = start + 2 + modu.data[start + 1];
  else throw new Error("unexpected name encoding");
  const text = Buffer.from(name);
  const value =
    text.length < 32
      ? Buffer.concat([Buffer.from([0xa0 + text.length]), text])
      : Buffer.concat([Buffer.from([0xd9, text.length]), text]);
  modu.data = Buffer.concat([modu.data.subarray(0, start), value, modu.data.subarray(end)]);

  // 並べ直す (チャンクは 16 バイト境界)
  const align = (n) => Math.ceil(n / 16) * 16;
  const header = Buffer.from(buf.subarray(0, 16 + count * 24));
  let offset = align(header.length);
  const parts = [header];
  let written = header.length;
  chunks.forEach((c, i) => {
    const at = 16 + i * 24;
    header.writeBigUInt64LE(BigInt(offset), at + 8);
    header.writeBigUInt64LE(BigInt(c.data.length), at + 16);
    parts.push(Buffer.alloc(offset - written));
    parts.push(c.data);
    written = offset + c.data.length;
    offset = align(written);
  });
  return Buffer.concat(parts);
}

/**
 * パラメータの多いモデル (T1630): 見本のモデルのパラメータを `count` 個まで、グループを `groups` 個まで足す。
 * 足したパラメータはグループに順に入れ、7 つに 1 つはグループに入れない (「Other」の区切りも出す)。名前は長いものを混ぜる。
 */
export function manyParametersModel(model, { count = 100, groups = 10 } = {}) {
  const parameters = [...(model.parameters ?? [])];
  const parameterGroups = [...(model.parameter_groups ?? [])];
  for (let i = parameterGroups.length; i < groups; i++) {
    parameterGroups.push({
      id: `Extra${i}`,
      name: i % 3 === 0 ? `Accessories and props ${i}` : `Extra ${i}`,
    });
  }
  for (let i = parameters.length; i < count; i++) {
    parameters.push({
      id: `Extra${i}`,
      name: i % 5 === 0 ? `Extra parameter with a long name ${i}` : `Extra ${i}`,
      min: -1,
      max: 1,
      default: 0,
      kind: "normal",
      mirror: null,
      group: i % 7 === 0 ? null : parameterGroups[i % parameterGroups.length].id,
    });
  }
  return { ...model, parameters, parameter_groups: parameterGroups };
}

/**
 * マスク専用かつ非表示のメッシュを 1 つ足したモデル (T1670。見本の `ugoe sample dress-up` にはマスク専用のメッシュが無い)。
 * 非表示のパーツ (見本の `Wave`) の手 (`HandWave`) を写し、マスク専用にしてすぐ後に置く (ツリーにマスクと非表示の札を持つ行が出る)。
 * 写すメッシュが無ければそのまま返す。描くのは見本の .ugoe のまま (このメッシュは無い)。
 */
export function maskOnlyHiddenModel(model) {
  const meshes = model.meshes ?? [];
  const i = meshes.findIndex((m) => m.id === "HandWave");
  if (i < 0) return model;
  const mask = { ...meshes[i], id: "HandWaveMask", name: "HandWaveMask", mask_only: true };
  return { ...model, meshes: [...meshes.slice(0, i + 1), mask, ...meshes.slice(i + 1)] };
}

/**
 * グルーを 1 つ足したモデル (T0949。見本の `ugoe sample dress-up` はグルーを持たない)。袖 (`SleeveWave`) と手 (`HandWave`)
 * をつなぎ、強さは `ArmWave` の 3 つのキーで変える (グルーのパネルの強さの欄・軸のキー・軸を足す欄を出す)。見本に無いメッシュ・
 * パラメータなら先頭のものを使い、メッシュが 2 つ無ければそのまま返す。描くのは見本の .ugoe のまま (グルーは無い)。
 */
export function glueModel(model) {
  const meshes = model.meshes ?? [];
  const pick = (id, skip) =>
    meshes.find((m) => m.id === id) ?? meshes.find((m) => m.id !== skip?.id);
  const a = pick("SleeveWave");
  const b = pick("HandWave", a);
  if (!a || !b || a === b) return model;
  const parameter =
    (model.parameters ?? []).find((p) => p.id === "ArmWave") ?? model.parameters?.[0];
  const vertices = (m) => m.uvs?.length ?? 0;
  const count = Math.min(5, vertices(a), vertices(b));
  const pairs = Array.from({ length: count }, (_, i) => ({
    a: vertices(a) - count + i,
    b: i,
    weight: 0.5,
  }));
  // 値域の両端と既定値 (重なれば 1 つに)。強さは両端で半分
  const keys = parameter
    ? [...new Set([parameter.min ?? -1, parameter.default ?? 0, parameter.max ?? 1])].sort(
        (x, y) => x - y,
      )
    : [];
  const glue = {
    id: "SleeveHandGlue",
    mesh_a: a.id,
    mesh_b: b.id,
    pairs,
    keyforms: parameter
      ? {
          bindings: [{ parameter: parameter.id, keys }],
          keyforms: keys.map((_, i) => ({
            intensity: i === 0 || i === keys.length - 1 ? 0.5 : 1,
          })),
        }
      : { bindings: [], keyforms: [{ intensity: 1 }] },
  };
  return { ...model, glues: [...(model.glues ?? []), glue] };
}

/**
 * 外での書き換え (`check_external_change`) と本体の変化 (`check_base_change`) の答え (T0953)。ファイルは 3 つを超えて
 * 「ほか N 件」を出す。
 */
export function externalChangeFixture() {
  return {
    files: [
      "model.json",
      "physics.json",
      "motions/idle-with-a-long-name.motion.json",
      "expressions/smile.expression.json",
      "textures/body.png",
    ],
    missing: false,
    token: "ui-audit-external",
  };
}

/** モジュールのプロジェクトの本体の変化 (`check_base_change`)。 */
export function baseChangeFixture() {
  return {
    files: ["model.json", "textures/face.png"],
    missing: false,
    token: "ui-audit-base",
  };
}
