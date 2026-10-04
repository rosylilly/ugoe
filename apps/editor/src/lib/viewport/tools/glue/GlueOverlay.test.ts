// @vitest-environment happy-dom
// グルーの頂点の組の表示: 組を編集するモードを抜けるキー操作 (docs/testing.md#エディタの部品のテスト)。
import { describe, expect, it } from "vitest";
import { GlueView } from "../../../panels/glues/glueView.svelte";
import { ViewportScene } from "../scene.svelte";
import { dialogButton, FakeController, FakeDocument, press, render } from "../test/harness.svelte";
import GlueOverlay from "./GlueOverlay.svelte";

function setup() {
  const controller = new FakeController();
  const doc = new FakeDocument();
  const view = new GlueView();
  view.active = true;
  view.glue = "Seam";
  const scene = new ViewportScene(controller.asController, doc.asDocument);
  render(GlueOverlay, { doc: doc.asDocument, scene, view });
  view.setPairMode("add");
  return { view };
}

describe("GlueOverlay keys", () => {
  it("leaves the pair mode with Esc", () => {
    const { view } = setup();
    press(document.body, "Escape");
    expect(view.pairMode).toBe("off");
  });

  it("does not take Esc pressed inside a dialog", () => {
    const { view } = setup();
    press(dialogButton(), "Escape");
    expect(view.pairMode).toBe("add");
  });
});
