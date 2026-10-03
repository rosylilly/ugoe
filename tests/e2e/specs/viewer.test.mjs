// ビューア (apps/viewer) の主な流れ: 見本を開く・トラッキングの記録を流してパラメータが動く・設定を開く。
// 流し方は tests/e2e/README.md。

import assert from "node:assert/strict";
import { describe } from "node:test";
import { launch, shot, step } from "../lib/harness.mjs";
import { installPageHelpers } from "../lib/page.mjs";
import { startFakeTracking, trackerState } from "../lib/tracking.mjs";
import { Keys, waitFor } from "../lib/webdriver.mjs";

describe("viewer", () => {
  const app = launch({ name: "viewer" });

  step(app, "starts", async () => {
    const s = app.session;
    await waitFor(() => s.execute("return !!window.__UGOE_VIEWER__"), {
      message: "the viewer to start",
    });
    await installPageHelpers(s);
  });

  step(app, "opens a sample from the Samples menu", async () => {
    const s = app.session;
    const menu = await s.waitFindText(".toolbar button", "Samples");
    await menu.click();
    const item = await s.waitFindText("[role=menuitem]", "Avatar");
    await item.click();
    await waitFor(
      () =>
        s.execute(`const p = window.__UGOE_VIEWER__; return p.status === "ready" && p.modelName`),
      { timeout: 20_000, message: "the sample to load" },
    );
    const error = await s.execute("return window.__UGOE_VIEWER__.error");
    assert.equal(error, null);
    await waitFor(() => s.execute(`return __e2e.slider("^Angle ?X( |$)") !== null`), {
      message: "the AngleX slider",
    });
    await shot(s, "viewer-sample");
  });

  step(app, "plays a tracking recording and the parameters follow", async () => {
    const s = app.session;
    await startFakeTracking(s, { seconds: 6 });
    const samples = [];
    const started = Date.now();
    while (Date.now() - started < 4000) {
      samples.push(await s.execute(`return window.__UGOE_VIEWER__.values.AngleX ?? null`));
      await new Promise((r) => setTimeout(r, 200));
    }
    const tracker = await trackerState(s);
    assert.equal(tracker.status, "running", JSON.stringify(tracker));
    const seen = samples.filter((v) => v !== null);
    assert.ok(seen.length > 0, "the model has no AngleX");
    const spread = Math.max(...seen) - Math.min(...seen);
    assert.ok(spread > 5, `AngleX moved only ${spread} (${JSON.stringify(samples)})`);
    // パネルのスライダーも同じ値を見せる
    const slider = await s.execute(`return __e2e.sliderValue("^Angle ?X( |$)")`);
    assert.ok(Number.isFinite(slider));
    await shot(s, "viewer-tracking");
    await s.execute("window.__UGOE_TRACKER__.stop()");
  });

  step(app, "opens and closes the settings", async () => {
    const s = app.session;
    const button = await s.waitFind(".toolbar button[aria-label=Settings]");
    await button.click();
    const title = await s.waitFind("[role=dialog] .ugoe-dialog__title");
    assert.equal(await title.text(), "Settings");
    await s.waitFindText("[role=dialog] .ugoe-select__label", "Language");
    await s.keys(Keys.Escape);
    await waitFor(async () => (await s.findAll("[role=dialog]")).length === 0, {
      message: "the settings to close",
    });
  });
});
