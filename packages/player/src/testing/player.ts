// テスト用の部品 (配布物に入らない。src/index.ts からは出さない)。偽の wasm と DOM で Player を動かす。
// テストのファイルは `@ugoe/face-tracking` を `./testing/faceTracking` に差し替え、先頭で `setupPlayer()` を呼ぶ。

import { MemoryStorage } from "@ugoe/i18n/testing";
import { beforeAll, beforeEach, vi } from "vitest";
import type { PlayerHost, Player as PlayerType } from "../player.svelte";
import type { UgoeWasmModule, WebPuppet } from "../wasm";
import { tracked } from "./faceTracking";
import { FakeCanvas, type FakeOptions, fakePuppet } from "./fakePuppet";

/** WebPuppet.create が返す約束 (テストが好きな時に解く)。 */
export const pending: {
  canvas: FakeCanvas;
  resolve: (p: WebPuppet) => void;
  reject: (e: unknown) => void;
}[] = [];
/** 次の `requestAnimationFrame` で呼ばれるもの。 */
let nextFrame: FrameRequestCallback | null = null;

const wasm: UgoeWasmModule = {
  WebPuppet: {
    create: (canvas) =>
      new Promise<WebPuppet>((resolve, reject) =>
        pending.push({ canvas: canvas as unknown as FakeCanvas, resolve, reject }),
      ),
  },
};

export const container = { prepend() {} } as unknown as HTMLElement;

let Player: typeof PlayerType;

/** 偽の requestAnimationFrame と document を置いて Player を読み、テストごとに偽物の状態と localStorage を空にする。 */
export function setupPlayer(): void {
  beforeAll(async () => {
    globalThis.requestAnimationFrame = (callback) => {
      nextFrame = callback;
      return 1;
    };
    globalThis.cancelAnimationFrame = () => {
      nextFrame = null;
    };
    globalThis.devicePixelRatio ??= 1;
    globalThis.document ??= {
      createElement: () => new FakeCanvas(),
    } as unknown as Document;
    ({ Player } = await import("../player.svelte"));
  });

  beforeEach(() => {
    pending.length = 0;
    nextFrame = null;
    tracked.length = 0;
    vi.stubGlobal("localStorage", new MemoryStorage());
  });
}

export function player(host: Partial<PlayerHost> = {}): PlayerType {
  return new Player({
    loadWasm: async () => wasm,
    readFile: () => Promise.reject(new Error("no files")),
    ...host,
  });
}

/** 待っているマイクロタスクを流す (loadWasm と create の await の後まで)。 */
export async function settle(): Promise<void> {
  for (let i = 0; i < 5; i++) await Promise.resolve();
}

/** `attach` して偽の WebPuppet を渡し、描画の準備ができた Player。 */
export async function ready(options: FakeOptions = {}, host: Partial<PlayerHost> = {}) {
  const p = player(host);
  const attached = p.attach(container);
  await settle();
  const fake = fakePuppet(options);
  pending.at(-1)?.resolve(fake.puppet);
  await attached;
  return { p, ...fake };
}

/** `requestAnimationFrame` の 1 回分を進める (`now` はミリ秒)。 */
export function step(now: number): void {
  const callback = nextFrame;
  nextFrame = null;
  callback?.(now);
}

/** 付けているモジュールの名前と、付けているか。 */
export function moduleRows(p: PlayerType): [string, boolean][] {
  return p.modules.map((m) => [m.name, m.attached]);
}
