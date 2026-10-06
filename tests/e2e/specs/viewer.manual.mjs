// ビューア (apps/viewer) のマニュアルの画像 (docs/manual/images) を撮る (`just e2e --manual --only viewer`)。
// マニュアルのビューアの画面は日本語で書くので、日本語の画面で撮る。流し方は tests/e2e/README.md#マニュアルの画像。

import { describe } from "node:test";
import { launch, step } from "../lib/harness.mjs";
import { manualShot, resizeTo, useLocale } from "../lib/manual.mjs";
import { installPageHelpers } from "../lib/page.mjs";
import { waitFor } from "../lib/webdriver.mjs";

/** 右の列の機能のパネル `css` を、上の段の頭まで送る。 */
function scrollPanel(session, css) {
  return session.execute((css) => {
    const panel = document.querySelector(css);
    if (!panel) throw new Error(`no ${css}`);
    panel.scrollIntoView({ block: "start" });
  }, css);
}

describe("viewer manual images", () => {
  const app = launch({ name: "viewer" });

  step(app, "starts in Japanese", async () => {
    const s = app.session;
    await waitFor(() => s.execute("return !!window.__UGOE_VIEWER__"), {
      message: "the viewer to start",
    });
    await useLocale(s, "ugoe.viewer.locale", "ja");
    await waitFor(() => s.execute("return !!window.__UGOE_VIEWER__"), {
      message: "the viewer to start again",
    });
    await s.waitFindText(".toolbar button", "サンプル");
    await installPageHelpers(s);
  });

  step(app, "dress-up-viewer-sample", async () => {
    const s = app.session;
    await resizeTo(s, 1300, 820);
    await s.clickText(".toolbar button", "サンプル");
    await s.clickText("[role=menuitem]", "着せ替え");
    await waitFor(
      () =>
        s.execute(
          `const p = window.__UGOE_VIEWER__; return p.status === "ready" && p.modules.length > 0`,
        ),
      { timeout: 20_000, message: "the dress-up sample to load" },
    );
    // 待機モーションの途中の姿にならないように、少し流してから撮る
    await new Promise((r) => setTimeout(r, 1500));
    await manualShot(s, "dress-up-viewer-sample");
  });

  step(app, "motions-viewer", async () => {
    const s = app.session;
    await resizeTo(s, 1100, 760);
    // nod の終わりのイベントを出してから、tilt を再生し smile に surprised を重ねる
    // 再生を始められなかったとき (モデルの読み込み直後など) のために、イベントが出るまで始め直す
    await waitFor(
      async () => {
        await s.execute(() => {
          const p = window.__UGOE_VIEWER__;
          if (p.currentMotion !== "nod") p.playMotion("nod", "normal");
        });
        await new Promise((r) => setTimeout(r, 500));
        return s.execute(() => window.__UGOE_VIEWER__.motionEvents.length > 0);
      },
      { timeout: 15_000, message: "a motion event" },
    );
    await s.execute(() => {
      const p = window.__UGOE_VIEWER__;
      p.setExpression("smile");
      p.addExpression("surprised");
      p.playMotion("tilt", "normal");
    });
    // モーションの一覧・イベント・表情が見えるように、イベントの行を真ん中へ送る
    await s.execute(() =>
      document.querySelector(".panel.motions .events")?.scrollIntoView({ block: "center" }),
    );
    await new Promise((r) => setTimeout(r, 700));
    await manualShot(s, "motions-viewer");
    await s.execute(() => {
      const p = window.__UGOE_VIEWER__;
      p.stopMotions();
      p.stopExpressions();
    });
  });

  step(app, "viewer-pose", async () => {
    const s = app.session;
    await resizeTo(s, 1300, 820);
    await scrollPanel(s, ".panel.poses");
    await s.clickWhen(
      () =>
        s.execute(() =>
          [...document.querySelectorAll(".panel.poses button, .panel.poses [role=radio]")].find(
            (b) => b.textContent.trim() === "Wave",
          ),
        ),
      { message: "the Wave pose" },
    );
    // フェードで切り替わり、手を振る姿になるまで待つ
    await new Promise((r) => setTimeout(r, 1200));
    await manualShot(s, "viewer-pose");
  });
});
