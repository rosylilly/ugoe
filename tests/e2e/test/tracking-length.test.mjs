// 重い検出のトラッキングを流す長さ (tests/e2e/lib/tracking-length.mjs) の単体テスト。`just script-check` で流す。

import assert from "node:assert/strict";
import { test } from "node:test";
import {
  DEFAULT_TRACKING_MS,
  MAX_TRACKING_MS,
  minTrackingDone,
  TRACKING_MS_ENV,
  trackingMs,
} from "../lib/tracking-length.mjs";

const env = (value) => ({ [TRACKING_MS_ENV]: value });

test("defaults to 13 seconds when the variable is unset or empty", () => {
  assert.equal(trackingMs({}), DEFAULT_TRACKING_MS);
  assert.equal(trackingMs(env("")), DEFAULT_TRACKING_MS);
  assert.equal(trackingMs(env("  ")), DEFAULT_TRACKING_MS);
  assert.equal(DEFAULT_TRACKING_MS, 13_000);
});

test("reads whole milliseconds within the range", () => {
  assert.equal(trackingMs(env("60000")), 60_000);
  assert.equal(trackingMs(env(" 13000 ")), 13_000);
  assert.equal(trackingMs(env(String(MAX_TRACKING_MS))), MAX_TRACKING_MS);
});

test("rejects values out of range or not whole milliseconds", () => {
  for (const bad of [
    "12999",
    "120001",
    "0",
    "-60000",
    "60s",
    "6e4",
    "60000.5",
    "NaN",
    "Infinity",
    "0x1000",
  ]) {
    assert.throws(
      () => trackingMs(env(bad)),
      (e) => e.message.includes(TRACKING_MS_ENV) && e.message.includes(JSON.stringify(bad)),
      bad,
    );
  }
});

test("scales the minimum deliveries with the length", () => {
  assert.equal(minTrackingDone(DEFAULT_TRACKING_MS), 10);
  assert.equal(minTrackingDone(60_000), 47);
  assert.equal(minTrackingDone(MAX_TRACKING_MS), 93);
});
