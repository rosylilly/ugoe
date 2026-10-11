// UI の監査の時間の上限 (組ごとの上限と全体の締め切り。T0884。条件の待ち。T0985) のテスト (`just ui-audit-check`)
import assert from "node:assert/strict";
import { test } from "node:test";
import {
  deadline,
  secondsOption,
  TimeLimitError,
  WAIT_TIMEOUT_MS,
  WaitTimeoutError,
  waitUntil,
  withTimeLimit,
} from "../lib/limits.mjs";

const never = () => new Promise(() => {});

test("withTimeLimit returns the result when the body finishes in time", async () => {
  let aborted = false;
  const r = await withTimeLimit(
    async () => "done",
    1000,
    () => {
      aborted = true;
    },
  );
  assert.equal(r, "done");
  assert.equal(aborted, false);
});

test("withTimeLimit passes the body's own failure through", async () => {
  await assert.rejects(
    withTimeLimit(async () => {
      throw new Error("no notice");
    }, 1000),
    /no notice/,
  );
});

test("withTimeLimit aborts and rejects a body that does not finish", async () => {
  let aborted = false;
  const started = performance.now();
  await assert.rejects(
    withTimeLimit(never, 20, () => {
      aborted = true;
    }),
    (e) => e instanceof TimeLimitError && e.ms === 20,
  );
  assert.equal(aborted, true);
  assert.ok(performance.now() - started < 1000);
});

test("withTimeLimit ignores the body's failure after the limit", async () => {
  // コンテキストを閉じると、待っていた操作は後から失敗する。それが未処理の reject にならない
  let fail;
  const body = () =>
    new Promise((_, reject) => {
      fail = reject;
    });
  await assert.rejects(
    withTimeLimit(body, 10, () => fail(new Error("Target closed"))),
    TimeLimitError,
  );
});

test("withTimeLimit without a finite limit waits for the body", async () => {
  assert.equal(await withTimeLimit(async () => 1, Number.POSITIVE_INFINITY), 1);
});

test("withTimeLimit with a passed deadline aborts at once", async () => {
  let aborted = false;
  await assert.rejects(
    withTimeLimit(never, -5, () => {
      aborted = true;
    }),
    TimeLimitError,
  );
  assert.equal(aborted, true);
});

test("deadline counts from the start", () => {
  let t = 1000;
  const d = deadline(2, { started: 1000, now: () => t });
  assert.equal(d.remaining(), 2000);
  assert.equal(d.passed(), false);
  t = 3000;
  assert.equal(d.passed(), true);
  assert.equal(deadline(null).passed(), false);
  assert.equal(deadline(null).remaining(), Number.POSITIVE_INFINITY);
});

test("secondsOption reads seconds, 0 as no limit and rejects bad values", () => {
  assert.equal(secondsOption(undefined, "--deadline"), null);
  assert.equal(secondsOption(undefined, "--variant-timeout", 480), 480);
  assert.equal(secondsOption("540", "--deadline"), 540);
  assert.equal(secondsOption("0", "--deadline", 480), null);
  for (const bad of ["-1", "abc", "NaN", "Infinity"]) {
    assert.throws(() => secondsOption(bad, "--deadline"), /--deadline/);
  }
});

/** Playwright の時間切れ (`TimeoutError`) を投げる待ち。 */
const timesOut = async () => {
  const e = new Error("locator.waitFor: Timeout 20000ms exceeded.");
  e.name = "TimeoutError";
  throw e;
};

test("waitUntil passes the shared limit to the wait and returns its result", async () => {
  let given = null;
  const r = await waitUntil("the menu", async (timeout) => {
    given = timeout;
    return "shown";
  });
  assert.equal(r, "shown");
  assert.equal(given, WAIT_TIMEOUT_MS);
  assert.equal(await waitUntil("the menu", async (timeout) => timeout, { timeout: 50 }), 50);
});

test("waitUntil names what it waited for, how long and the hint when the wait times out", async () => {
  let clock = 1000;
  const now = () => clock;
  await assert.rejects(
    waitUntil(
      "the File menu to open",
      async () => {
        clock += 20000;
        return timesOut();
      },
      { now, hint: async () => "notices: none" },
    ),
    (e) =>
      e instanceof WaitTimeoutError &&
      e.message === "timed out after 20.0s waiting for the File menu to open (notices: none)" &&
      e.what === "the File menu to open",
  );
  await assert.rejects(
    waitUntil("the dialog", timesOut, { now }),
    /^WaitTimeoutError: timed out after 0\.0s waiting for the dialog$/,
  );
});

test("waitUntil keeps the timeout message when the hint itself fails", async () => {
  await assert.rejects(
    waitUntil("the dialog", timesOut, {
      hint: async () => {
        throw new Error("page closed");
      },
    }),
    (e) => e instanceof WaitTimeoutError && !e.message.includes("("),
  );
});

test("waitUntil passes other failures through", async () => {
  await assert.rejects(
    waitUntil("the dialog", async () => {
      throw new Error("strict mode violation");
    }),
    (e) => !(e instanceof WaitTimeoutError) && /strict mode/.test(e.message),
  );
});
