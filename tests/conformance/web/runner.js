// 適合テストの wasm (npm パッケージ @ugoe/runtime) の実行器 (tests/conformance/README.md)。
//
// run.mjs のサーバーが配るページで動く。計画 (plan.json) のケースごとに、パッケージの `createPuppet` と
// `loadModel` で WebPuppet を作り、評価の結果 (<ケース>.json) と描いた画像 (<ケース>.png) をサーバーへ送る。
//
// - パッケージは /pkg/ (bindings/web/dist)、モデルは /file/<計画の番号>/<モジュールの番号 or "model"> で取る。
// - 描画のバックエンドは URL の ?backend= (webgl2 / webgpu)。
// - 時間を進めないケースは、すべての段を止めて評価する (C の実行器と同じ)。

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

/** WebPuppet から評価の結果を取り出す (形は tests/conformance/src/frame.rs。位置と描画順だけ)。 */
function frame(puppet) {
  const ids = puppet.drawable_ids();
  const meshes = {};
  ids.forEach((id, i) => {
    const xy = puppet.drawable_positions(i);
    const positions = [];
    for (let v = 0; v + 1 < xy.length; v += 2) positions.push([xy[v], xy[v + 1]]);
    meshes[id] = { positions };
  });
  const render_order = Array.from(puppet.render_order(), (i) => ids[i]);
  return { meshes, render_order };
}

/** canvas の中身を PNG にする。描いた直後 (同じタスクの中) に 2D の canvas へ写す。 */
function snapshot(canvas) {
  const copy = document.createElement("canvas");
  copy.width = canvas.width;
  copy.height = canvas.height;
  const context = copy.getContext("2d");
  context.drawImage(canvas, 0, 0);
  return new Promise((resolve, reject) =>
    copy.toBlob((blob) => (blob ? resolve(blob) : reject(new Error("toBlob failed"))), "image/png"),
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
      for (const dt of c.steps) puppet.update(dt);
    }
    await post(`/result/${c.name}.json`, JSON.stringify(frame(puppet)));
    if (c.image) {
      const [w, h] = puppet.canvas_size();
      puppet.resize(Math.round(w), Math.round(h));
      puppet.fit();
      const [r, g, b] = plan.background;
      puppet.set_background(r / 255, g / 255, b / 255, 1);
      if (c.lighting) puppet.set_lighting_json(JSON.stringify(c.lighting));
      puppet.render();
      await post(`/result/${c.name}.png`, await snapshot(canvas));
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
