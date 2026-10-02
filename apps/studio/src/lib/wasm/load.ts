// ugoe-wasm の JS バインディングを読み込む。
//
// バインディングは `pnpm wasm` (= `just wasm apps/studio/src/lib/wasm/generated`) で生成し、
// コミットしない。生成していなくてもビルドと型検査が通るよう、実行時は import.meta.glob で探す。
import type * as UgoeWasm from "ugoe-wasm";

export type { WebPuppet } from "ugoe-wasm";
export type UgoeWasmModule = typeof UgoeWasm;

const found = import.meta.glob<UgoeWasmModule>("./generated/ugoe_wasm.js");

let pending: Promise<UgoeWasmModule | null> | undefined;

/** wasm を初期化して返す (2 回目以降は同じもの)。生成物が無ければ `null`。 */
export function loadWasm(): Promise<UgoeWasmModule | null> {
  pending ??= (async () => {
    const importer = found["./generated/ugoe_wasm.js"];
    if (importer === undefined) return null;
    const module = await importer();
    await module.default();
    return module;
  })();
  return pending;
}
