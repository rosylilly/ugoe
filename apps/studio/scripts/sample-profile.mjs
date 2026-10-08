#!/usr/bin/env node
// 見本のプロファイル (assets/samples の ugoe-sample-profile.json・ugoe-sample-profile.en.json) を書き出す
// (`pnpm --filter @ugoe/studio sample-profile [--check]`。T0806・T0865)。
// 見本のモデル (assets/samples) を使うシーン・ホットキーを持つ。プロファイルは見本のモデルと同じフォルダに置き、モデルを相対パスで持つ
// (docs/studio.md#プロファイルのパス) ので、コミットしたものをどの機械でもそのまま開ける。手順は docs/manual/studio.md#見本のプロファイル。
// 見本の印 (`template`) を付けるので、スタジオは開いて変えても自動の保存で書き換えず、写しを保存させる (docs/studio.md#書き換えないプロファイル、T0866)。
// `--check` は書き出さずに、コミットしたものがここで作るものと同じかを確かめる (違えば 1 で終わる)。

import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

/** リポジトリの assets/samples。 */
export const SAMPLES_DIR = join(import.meta.dirname, "..", "..", "..", "assets", "samples");

/** 見本のプロファイルのファイル名 (`SAMPLES_DIR` の下。シーンの名前の言語ごと)。 */
export const PROFILE_FILES = { ja: "ugoe-sample-profile.json", en: "ugoe-sample-profile.en.json" };

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
 * 見本のプロファイル (`apps/studio/src-tauri/src/profile.rs` の形)。ファイルのパスは `SAMPLES_DIR` からの相対パス (区切りは `/`)。
 * @param {"ja" | "en"} lang シーンの名前の言語
 */
export function sampleProfile(lang = "ja") {
  const names = NAMES[lang] ?? NAMES.ja;
  const file = (...parts) => parts.join("/");
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
    template: true,
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
      // 着せ替えのシーンの本体 (dress-up.ugoe) のポーズの要素 (手を振る差分・手を下ろした姿)。このシーンでだけ効く
      key("Ctrl+Alt+KeyW", { kind: "pose", part: "Wave" }),
      key("Ctrl+Alt+KeyD", { kind: "pose", part: "HandsDown" }),
      key("Ctrl+Alt+KeyP", { kind: "trackingPause" }),
      key("Ctrl+Alt+KeyK", { kind: "trackingCalibrate" }),
      key("Ctrl+Alt+KeyL", { kind: "lipSyncPause" }),
    ],
  };
}

/**
 * プロファイルの中のファイルの相対パス (`sampleProfile`) を、`dir` の下の絶対パスに置き換えた写し
 * (見本のモデルのフォルダの外にプロファイルを書くとき。e2e と ui-audit の偽のホスト)。
 * 写しなので見本の印 (`template`) は外す (スタジオが写しを保存したときと同じ。自動の保存で書き換えてよい)。
 * @param {ReturnType<typeof sampleProfile>} profile
 * @param {string} dir
 */
export function resolveSampleProfile(profile, dir = SAMPLES_DIR) {
  const copy = structuredClone(profile);
  delete copy.template;
  const file = (path) => join(dir, ...path.split("/"));
  for (const scene of copy.scenes) {
    scene.model = file(scene.model);
    scene.modules = scene.modules.map(file);
  }
  for (const hotkey of copy.hotkeys) {
    if (hotkey.action.kind === "dressUp") hotkey.action.module = file(hotkey.action.module);
  }
  return copy;
}

/** 見本のプロファイルのファイルの中身。 */
export function sampleProfileText(lang) {
  return `${JSON.stringify(sampleProfile(lang), null, 2)}\n`;
}

function main(argv) {
  const check = argv.includes("--check");
  if (argv.some((a) => a !== "--check")) {
    console.error("usage: pnpm --filter @ugoe/studio sample-profile [--check]");
    process.exit(2);
  }
  let stale = false;
  for (const [lang, name] of Object.entries(PROFILE_FILES)) {
    const path = join(SAMPLES_DIR, name);
    const text = sampleProfileText(lang);
    if (check) {
      if (!existsSync(path) || readFileSync(path, "utf8") !== text) {
        console.error(`${path} is out of date (run \`pnpm --filter @ugoe/studio sample-profile\`)`);
        stale = true;
      }
    } else {
      writeFileSync(path, text);
      console.log(`wrote ${path}`);
    }
  }
  if (stale) process.exit(1);
}

if (process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2));
}
