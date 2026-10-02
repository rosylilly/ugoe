// Toast の列 (アプリに 1 つ)。`showToast()` で足し、`<Toaster />` が描く (docs/ui.md#部品の一覧)。
// すぐ消えてよい知らせだけに使う。操作を含む知らせや、残しておく知らせは Notice にする。

import { untrack } from "svelte";
import type { Tone } from "./types";

/** 出している知らせ。 */
export interface ToastItem {
  id: number;
  /** 翻訳を通した文言。 */
  message: string;
  tone: Tone;
}

/** `showToast()` の指定。 */
export interface ToastOptions {
  tone?: Tone;
  /** 出しておく時間 (ミリ秒)。既定は 5000。0 なら閉じるまで出しておく。 */
  duration?: number;
}

const DEFAULT_DURATION = 5000;

const items = $state<ToastItem[]>([]);
const timers = new Map<
  number,
  { timeout?: ReturnType<typeof setTimeout>; start: number; remaining: number }
>();
let nextId = 1;
let paused = false;

function schedule(id: number, ms: number) {
  const timer = {
    start: Date.now(),
    remaining: ms,
    timeout: undefined as ReturnType<typeof setTimeout> | undefined,
  };
  if (!paused) timer.timeout = setTimeout(() => dismissToast(id), ms);
  timers.set(id, timer);
}

/**
 * 知らせを出す。返す id で `dismissToast()` できる。
 * `$effect` の中から呼んでも、その effect が列に依存しないようにする (依存すると、足すたびに effect が走り直して止まらない)。
 */
export function showToast(message: string, options: ToastOptions = {}): number {
  const id = nextId++;
  untrack(() => items.push({ id, message, tone: options.tone ?? "info" }));
  const duration = options.duration ?? DEFAULT_DURATION;
  if (duration > 0) schedule(id, duration);
  return id;
}

/** 知らせを閉じる。 */
export function dismissToast(id: number): void {
  const timer = timers.get(id);
  if (timer?.timeout) clearTimeout(timer.timeout);
  timers.delete(id);
  untrack(() => {
    const index = items.findIndex((t) => t.id === id);
    if (index >= 0) items.splice(index, 1);
  });
}

/** 出している知らせ (古い順)。 */
export function toasts(): readonly ToastItem[] {
  return items;
}

/** ポインタやフォーカスが知らせの上にある間、消えるまでの時間を止める (WCAG 2.2.1)。 */
export function pauseToasts(): void {
  if (paused) return;
  paused = true;
  const now = Date.now();
  for (const timer of timers.values()) {
    if (timer.timeout) clearTimeout(timer.timeout);
    timer.timeout = undefined;
    timer.remaining = Math.max(0, timer.remaining - (now - timer.start));
  }
}

/** 止めていた時間を再び進める。 */
export function resumeToasts(): void {
  if (!paused) return;
  paused = false;
  for (const [id, timer] of timers) schedule(id, timer.remaining);
}
