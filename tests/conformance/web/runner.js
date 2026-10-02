// 適合テストの wasm (npm パッケージ @ugoe/runtime) の実行器 (tests/conformance/README.md)。
//
// run.mjs のサーバーが配るページで動く。計画 (plan.json) のケースごとに、パッケージの `createPuppet` と
// `loadModel` で WebPuppet を作り、評価の結果 (<ケース>.json) と描いた画像 (<ケース>.png) をサーバーへ送る。
// 画像は `read_pixels` (canvas とは別のテクスチャに描いて読み戻す) で取る。canvas の読み戻しは WebGPU のヘッドレスの
// Chrome で空になることがあったため (T0418)。
//
// - パッケージは /pkg/ (bindings/web/dist)、モデルは /file/<計画の番号>/<モジュールの番号 or "model"> で取る。
// - 描画のバックエンドは URL の ?backend= (webgl2 / webgpu)。
// - 時間を進めないケースは、すべての段を止めて評価する (C の実行器と同じ)。
// - `follow` のあるケースは、`update` の代わりに `follow(値, dt)` で進める (T0133)。

import { createPuppet, loadModel } from "/pkg/index.js";

const STAGES = ["physics", "eye_blink", "breath", "lip_sync", "pose"];
const backend = new URLSearchParams(location.search).get("backend") ?? "webgl2";

async function post(path, body) {
  const response = await fetch(path, { method: "POST", body });
  if (!response.ok) throw new Error(`POST ${path}: ${response.status}`);
}

function log(message) {
  return post("/log", message);
}

async function bytes(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`GET ${url}: ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}

/** WebPuppet から評価の結果を取り出す (形は tests/conformance/src/frame.rs)。 */
function frame(puppet) {
  const ids = puppet.drawable_ids();
  const states = JSON.parse(puppet.drawable_states_json());
  const color = ([r, g, b]) => ({ r, g, b });
  const meshes = {};
  ids.forEach((id, i) => {
    const xy = puppet.drawable_positions(i);
    const positions = [];
    for (let v = 0; v + 1 < xy.length; v += 2) positions.push([xy[v], xy[v + 1]]);
    const s = states[i];
    meshes[id] = {
      positions,
      opacity: s.opacity,
      draw_order: s.draw_order,
      multiply_color: color(s.multiply_color),
      screen_color: color(s.screen_color),
      mask_strength: s.mask_strength,
      visible: s.visible,
    };
  });
  const render_order = Array.from(puppet.render_order(), (i) => ids[i]);
  return { meshes, render_order };
}

/** 読み戻した画素 (ストレートアルファの RGBA) を PNG にする。 */
async function png(puppet) {
  const pixels = await puppet.read_pixels();
  const [width, height] = [puppet.width(), puppet.height()];
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d");
  context.putImageData(new ImageData(new Uint8ClampedArray(pixels.buffer), width, height), 0, 0);
  return new Promise((resolve, reject) =>
    canvas.toBlob(
      (blob) => (blob ? resolve(blob) : reject(new Error("toBlob failed"))),
      "image/png",
    ),
  );
}

async function runCase(plan, index) {
  const c = plan.cases[index];
  await post("/start", c.name);
  const canvas = document.createElement("canvas");
  canvas.width = 1;
  canvas.height = 1;
  document.body.append(canvas);
  const puppet = await createPuppet(canvas, { backend });
  try {
    if (puppet.backend() !== backend)
      throw new Error(`drawing with ${puppet.backend()}, not ${backend}`);
    loadModel(puppet, await bytes(`/file/${index}/model`));
    for (let m = 0; m < c.modules.length; m++) {
      const name = puppet.load_module(await bytes(`/file/${index}/${m}`));
      puppet.attach_module(name);
    }
    for (const [id, value] of Object.entries(c.parameters)) {
      if (!puppet.set_parameter(id, value)) throw new Error(`unknown parameter ${id}`);
    }
    if (c.steps.length === 0) {
      for (const stage of STAGES) puppet.set_enabled(stage, false);
    } else {
      puppet.set_enabled("eye_blink", c.effects);
      puppet.set_enabled("breath", c.effects);
      if (c.follow?.length) {
        // 受け手として進める (T0133): 各フレームの値を基準値として書き、物理とポーズだけを進める。
        c.steps.forEach((dt, i) => {
          puppet.follow(new Float32Array(c.follow[i]), dt);
        });
      } else {
        for (const dt of c.steps) puppet.update(dt);
      }
    }
    await post(`/result/${c.name}.json`, JSON.stringify(frame(puppet)));
    if (c.image) {
      const [w, h] = puppet.canvas_size();
      puppet.resize(Math.round(w), Math.round(h));
      puppet.fit();
      const [r, g, b] = plan.background;
      puppet.set_background(r / 255, g / 255, b / 255, 1);
      if (c.lighting) puppet.set_lighting_json(JSON.stringify(c.lighting));
      await post(`/result/${c.name}.png`, await png(puppet));
    }
  } finally {
    puppet.free();
    canvas.remove();
  }
}

async function main() {
  const plan = await (await fetch("/plan.json")).json();
  let failed = 0;
  for (let i = 0; i < plan.cases.length; i++) {
    try {
      await runCase(plan, i);
    } catch (e) {
      failed += 1;
      await log(`${plan.cases[i].name}: ${e?.stack ?? e}`);
    }
  }
  await post("/done", JSON.stringify({ total: plan.cases.length, failed }));
}

main().catch((e) => post("/done", JSON.stringify({ error: String(e?.stack ?? e) })));
