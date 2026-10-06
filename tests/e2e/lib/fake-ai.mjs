// 偽の AI のプロバイダ: この計算機 (127.0.0.1) で OpenAI 互換の Chat Completions (`POST <base>/chat/completions`) に答える
// 小さなサーバ。ネットワークにも本物のモデルにも頼らずに、エディタの AI の提案 (分類・リグ) の経路を通すために使う (T0029)。
//
// エディタは送り先がループバックなら同意を求めず、API キーも要らない (apps/editor/src-tauri/src/ai_provider.rs の `is_local`)。
// 答えは要求の JSON Schema (`response_format.json_schema.schema`) から決める。
// - 分類 (`layers` を持つスキーマ。crates/ugoe-ai/src/classify.rs の `output_schema`): プロンプトの各レイヤーの行
//   (`<番号>. name: "<グループ> / <レイヤー>"; ...`) に、レイヤーの名前から決めたラベルを答える (`LABELS`。無ければ `other`)。
// - それ以外 (リグの値。crates/ugoe-ai/src/rig.rs の `values_schema`): 数の項目には説明の範囲 (`From <min> to <max>.`) の
//   真ん中を、`reason` には決まった文を答える。
// - 利用者の注文 (プロンプトの `<user_instructions>`。crates/ugoe-ai/src/instructions.rs) があれば、リグの値の `reason` に
//   注文の文ごとの答えを足す: 髪・揺れ・帽子のような、リグの値では変えられない文は `Not applied:` の行 (画面の「Not applied」)、
//   ほかの文は従う。従う文が値の言葉 (`STEERS`。振り向き・目・笑い・口) と向きの言葉 (`LESS`・`MORE`。less・narrower・控えめ・細く
//   など / more・wider・大きく など) を持てば、その値を範囲の 6 分の 1 だけ下げるか上げる (笑いは `eye_close_bend` を下げる)。
//   文ごとに足し、範囲の端で止める。同じ注文なら同じ値になる。値の言葉か向きの言葉が無い文は、値を変えずに従ったことにする。
//   動かした値は理由の続き (`(Moved by a sixth of the range: <名前> down, ...)`) に書く。

import { createServer } from "node:http";

/** レイヤーの名前 (パスの最後) → [ラベル, 左右]。`assets/fixtures/character.psd` のレイヤー。 */
export const LABELS = {
  背景: ["background", "center"],
  後ろ髪: ["hair_back", "center"],
  首: ["neck", "center"],
  服: ["topwear", "center"],
  顔: ["face", "center"],
  右目: ["eye", "right"],
  左目: ["eye", "left"],
  口: ["mouth", "center"],
  前髪: ["hair_front", "center"],
};

/** 偽のリグの答えの理由。提案の理由の欄に出る (マニュアルの画像にも写るので、偽の答えの決め方をそのまま書く)。 */
export const FAKE_REASON = "Each value is the middle of its allowed range.";

/** リグの値 (振り向き・目・口) では変えられない注文の文。`Not applied:` で答える。 */
const NOT_A_RIG_VALUE = /hair|sway|hat|headwear|physics|髪|揺|帽子/i;

/**
 * 注文の文で動かす値: 文が `words` を持てば、名前が `value` に合う数の項目を動かす (`invert` なら向きを逆に)。
 * 名前は crates/ugoe-ai/src/rig.rs の `RigParams::RANGES`。
 */
const STEERS = [
  { words: /\b(turn|turns|head)\b|振り向|向き/i, value: /^turn_angle_scale$/ },
  { words: /\beyes?\b|目/i, value: /^eye_close_line$/ },
  { words: /\bsmil(e|es|ing)\b|笑/i, value: /^eye_close_bend$/, invert: true },
  { words: /\bmouth\b|口/i, value: /^mouth_open_height$/ },
];

/** 値を下げる言葉と上げる言葉。 */
const LESS = /\b(less|smaller|narrower|lower|weaker)\b|控えめ|小さ|細|弱/i;
const MORE = /\b(more|larger|bigger|wider|higher|stronger)\b|大き|太|広|強/i;

/** 値の 1 歩 (範囲の何分の 1 か)。真ん中から両端まで 3 歩。 */
const STEPS = 6;

/** 注文の文。`。`・`.`・`!`・`?` で区切る。 */
function sentencesOf(instructions) {
  return instructions
    .split(/(?<=[.!?。])\s*/)
    .map((s) => s.trim())
    .filter((s) => s !== "");
}

/** 従う文が動かす値の歩数: 名前 → 歩数 (負なら下げる)。 */
function steering(sentences, names) {
  const steps = {};
  for (const s of sentences) {
    const direction = LESS.test(s) ? -1 : MORE.test(s) ? 1 : 0;
    if (direction === 0) continue;
    for (const { words, value, invert } of STEERS) {
      if (!words.test(s)) continue;
      for (const name of names.filter((n) => value.test(n))) {
        steps[name] = (steps[name] ?? 0) + (invert ? -direction : direction);
      }
    }
  }
  return steps;
}

/** 要求のプロンプトの文 (user の content の text をつなげたもの)。 */
function promptText(body) {
  const user = body.messages?.find((m) => m.role === "user");
  if (!user) return "";
  if (typeof user.content === "string") return user.content;
  return user.content
    .filter((p) => p.type === "text")
    .map((p) => p.text)
    .join("\n");
}

/** プロンプトの利用者の注文 (`<user_instructions>` の中の JSON の文字列)。無ければ `null`。 */
function instructionsIn(prompt) {
  const match = /<user_instructions>\s*("(?:[^"\\]|\\.)*")\s*<\/user_instructions>/.exec(prompt);
  return match ? JSON.parse(match[1]) : null;
}

/** 従う文 (リグの値で変えられる文)。 */
function followedOf(sentences) {
  return sentences.filter((s) => !NOT_A_RIG_VALUE.test(s));
}

/**
 * 注文に答える理由の続き: 従った文と動かした値 (`moved`。名前 → 歩数)、リグの値では変えられない文 (`Not applied:` の行)。
 */
function answerInstructions(sentences, moved) {
  const followed = followedOf(sentences);
  const lines = followed.length > 0 ? [` As asked: ${followed.join(" ")}`] : [];
  const changes = Object.entries(moved)
    .filter(([, n]) => n !== 0)
    .map(([name, n]) => `${name} ${n < 0 ? "down" : "up"}`);
  if (changes.length > 0) lines.push(` (Moved by a sixth of the range: ${changes.join(", ")}.)`);
  for (const s of sentences.filter((s) => NOT_A_RIG_VALUE.test(s))) {
    lines.push(`\nNot applied: ${s} This is not one of the rig values.`);
  }
  return lines.join("");
}

function classify(prompt) {
  const layers = [];
  for (const match of prompt.matchAll(/^(\d+)\. name: ("(?:[^"\\]|\\.)*")/gm)) {
    const path = JSON.parse(match[2]).split(" / ");
    const [label, side] = LABELS[path.at(-1)] ?? ["other", "center"];
    layers.push({ index: Number(match[1]), label, side, confidence: 0.9 });
  }
  return { layers, variant_sets: [] };
}

/**
 * リグの値の答え。数の項目は範囲の真ん中から、注文の従う文が動かす歩数 (範囲の 6 分の 1 ずつ、端で止める) だけ動かす。
 * 範囲の無い項目は 0。
 */
function values(schema, prompt) {
  const instructions = instructionsIn(prompt);
  const sentences = instructions ? sentencesOf(instructions) : [];
  const properties = Object.entries(schema.properties ?? {});
  const steps = steering(
    followedOf(sentences),
    properties.filter(([, p]) => p.type === "number").map(([name]) => name),
  );
  const moved = {};
  const answer = {};
  for (const [name, property] of properties) {
    if (property.type !== "number") continue;
    const range = /From (-?\d+(?:\.\d+)?) to (-?\d+(?:\.\d+)?)/.exec(property.description ?? "");
    if (!range) {
      answer[name] = 0;
      continue;
    }
    const [min, max] = [Number(range[1]), Number(range[2])];
    const half = STEPS / 2;
    const step = Math.min(half, Math.max(-half, steps[name] ?? 0));
    if (step !== 0) moved[name] = step;
    answer[name] = min + ((max - min) * (half + step)) / STEPS;
  }
  for (const [name, property] of properties) {
    if (property.type === "number") continue;
    answer[name] = instructions ? FAKE_REASON + answerInstructions(sentences, moved) : FAKE_REASON;
  }
  return answer;
}

/**
 * 偽のプロバイダを 127.0.0.1 の空いているポートで起動する。
 * 返す `requests` に、受けた要求の種類 (`classify` か `values`)・画像の数・モデルを溜める。
 * @returns {Promise<{ baseUrl: string, requests: { kind: string, images: number, model: string }[], close: () => Promise<void> }>}
 */
export async function startFakeAi() {
  const requests = [];
  const server = createServer((req, res) => {
    const chunks = [];
    req.on("data", (c) => chunks.push(c));
    req.on("end", () => {
      const reply = (status, value) => {
        res.writeHead(status, { "content-type": "application/json" });
        res.end(JSON.stringify(value));
      };
      if (req.method !== "POST" || !req.url.endsWith("/chat/completions")) {
        reply(404, { error: { message: `no such endpoint: ${req.method} ${req.url}` } });
        return;
      }
      let body;
      try {
        body = JSON.parse(Buffer.concat(chunks).toString("utf8"));
      } catch (e) {
        reply(400, { error: { message: `not JSON: ${e.message}` } });
        return;
      }
      const schema = body.response_format?.json_schema?.schema ?? {};
      const kind = schema.properties?.layers ? "classify" : "values";
      const images = (body.messages ?? [])
        .flatMap((m) => (Array.isArray(m.content) ? m.content : []))
        .filter((p) => p.type === "image_url").length;
      requests.push({ kind, images, model: body.model });
      const prompt = promptText(body);
      const answer = kind === "classify" ? classify(prompt) : values(schema, prompt);
      reply(200, {
        id: `e2e-${requests.length}`,
        object: "chat.completion",
        model: body.model,
        choices: [
          {
            index: 0,
            message: { role: "assistant", content: JSON.stringify(answer) },
            finish_reason: "stop",
          },
        ],
      });
    });
  });
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const { port } = server.address();
  return {
    baseUrl: `http://127.0.0.1:${port}/v1`,
    requests,
    close: () => new Promise((resolve) => server.close(() => resolve())),
  };
}
