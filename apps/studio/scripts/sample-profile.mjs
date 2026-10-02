#!/usr/bin/env node
// 見本のプロファイルを書き出す (`pnpm --filter @ugoe/studio sample-profile <出力の .json> [--lang en] [--force]`。T0806)。
// 見本のモデル (assets/samples) を使うシーン・ホットキーを持つ。プロファイルはモデルを絶対パスで持つ (docs/studio.md#プロファイルの形式)
// ので、ファイルをコミットせず、このリポジトリの場所に合わせてその場で作る。手順は docs/manual/studio.md#見本のプロファイル。

import { existsSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

/** リポジトリの assets/samples。 */
export const SAMPLES_DIR = join(import.meta.dirname, "..", "..", "..", "assets", "samples");

/** シーンの名前 (プロファイルの中身なので、画面の言語では変わらない。書き出すときに選ぶ)。 */
const NAMES = {
  ja: {
    chat: "雑談",
    evening: "夕方 (ライティング)",
    dressUp: "着せ替え",
    corner: "ゲーム (右下)",
  },
  en: { chat: "Chat", evening: "Evening (lighting)", dressUp: "Dress-up", corner: "Game (corner)" },
};

/** `@ugoe/player` の光源の組「夕日」と同じ値 (packages/player/src/lighting.ts の PRESETS)。 */
const SUNSET = {
  enabled: true,
  ambient: { color: "#8c9bd0", intensity: 0.45 },
  directional: { enabled: true, color: "#ffb070", intensity: 1, azimuth: 10, elevation: 20 },
  point: { enabled: false, color: "#ffffff", intensity: 1, x: 0.3, y: 0.3, z: 0.4, range: 1.5 },
};

const PLACEMENT = { x: 0, y: 0, scale: 1, rotation: 0 };

/**
 * 見本のプロファイル (`apps/studio/src-tauri/src/profile.rs` の形)。
 * @param {string} samples 見本のモデルのフォルダ (絶対パス)
 * @param {"ja" | "en"} lang シーンの名前の言語
 */
export function sampleProfile(samples, lang = "ja") {
  const names = NAMES[lang] ?? NAMES.ja;
  const file = (...parts) => join(samples, ...parts);
  const scene = (id, name, model, rest = {}) => ({
    id,
    name,
    model,
    modules: [],
    lighting: null,
    transform: PLACEMENT,
    background: { kind: "transparent" },
    ...rest,
  });
  const key = (keys, action) => ({ keys, action });
  return {
    format: "ugoe-studio-profile",
    version: 1,
    scenes: [
      scene("chat", names.chat, file("avatar.ugoe")),
      scene("evening", names.evening, file("avatar-lit.ugoe"), { lighting: SUNSET }),
      scene("dress-up", names.dressUp, file("dress-up", "dress-up.ugoe"), {
        modules: [file("dress-up", "long-hair.ugoemod"), file("dress-up", "sailor.ugoemod")],
        background: { kind: "chroma", color: "#00ff00" },
      }),
      // 雑談と同じモデルなので、切り替えてもトラッキングとモーションが続く (配置だけが変わる)
      scene("corner", names.corner, file("avatar.ugoe"), {
        transform: { x: 0.3, y: 0.22, scale: 0.55, rotation: 0 },
      }),
    ],
    activeScene: "chat",
    output: { size: "1920x1080" },
    hotkeys: [
      key("Ctrl+Shift+Digit1", { kind: "scene", scene: "chat" }),
      key("Ctrl+Shift+Digit2", { kind: "scene", scene: "evening" }),
      key("Ctrl+Shift+Digit3", { kind: "scene", scene: "dress-up" }),
      key("Ctrl+Shift+Digit4", { kind: "scene", scene: "corner" }),
      key("Ctrl+Alt+Digit1", { kind: "expression", expression: "smile", mode: "toggle" }),
      key("Ctrl+Alt+Digit2", { kind: "expression", expression: "surprised", mode: "toggle" }),
      key("Ctrl+Alt+Digit0", { kind: "clearExpressions" }),
      key("Ctrl+Alt+KeyN", { kind: "motion", motion: "nod", priority: "normal" }),
      key("Ctrl+Alt+KeyM", { kind: "motion", motion: "shake", priority: "normal" }),
      // 着せ替えのシーン (dress-up.ugoe のスロット) でだけ効く
      key("Ctrl+Alt+KeyH", {
        kind: "dressUp",
        slot: "Hair",
        module: file("dress-up", "twin-tails.ugoemod"),
      }),
      key("Ctrl+Alt+KeyC", {
        kind: "dressUp",
        slot: "Headwear",
        module: file("dress-up", "cat-ears.ugoemod"),
      }),
      key("Ctrl+Alt+KeyP", { kind: "trackingPause" }),
      key("Ctrl+Alt+KeyK", { kind: "trackingCalibrate" }),
      key("Ctrl+Alt+KeyL", { kind: "lipSyncPause" }),
    ],
  };
}

function main(argv) {
  const args = argv.filter((a) => !a.startsWith("--"));
  const langIndex = argv.indexOf("--lang");
  const lang = langIndex >= 0 ? argv[langIndex + 1] : "ja";
  const out = args.find((a) => a !== lang);
  if (out === undefined || !["ja", "en"].includes(lang)) {
    console.error(
      "usage: pnpm --filter @ugoe/studio sample-profile <out.json> [--lang ja|en] [--force]",
    );
    process.exit(2);
  }
  // pnpm --filter はパッケージのフォルダで動かすので、相対のパスは呼んだ場所から読む
  const path = resolve(process.env.INIT_CWD ?? process.cwd(), out);
  if (existsSync(path) && !argv.includes("--force")) {
    console.error(`${path} already exists (add --force to overwrite)`);
    process.exit(1);
  }
  writeFileSync(path, `${JSON.stringify(sampleProfile(SAMPLES_DIR, lang), null, 2)}\n`);
  console.log(`wrote ${path}`);
}

if (process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2));
}
