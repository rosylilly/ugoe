// ugoe-wasm (`just wasm` で generated/ に生成される) の読み込み。
//
// 生成物は gitignore しているので、無くてもフロントエンドのビルドと型検査は通るようにする:
// - 実行時は import.meta.glob で探し、無ければ null を返す (ビューポートは案内を表示する)。
// - 型は tsconfig.json の paths で generated/ を優先し、無ければ bindings/web/stub/ugoe_wasm.d.ts を使う。
import type * as UgoeWasm from "ugoe-wasm";

export type { WebPuppet } from "ugoe-wasm";

const modules = import.meta.glob<typeof UgoeWasm>("./generated/ugoe_wasm.js");

let loading: Promise<typeof UgoeWasm | null> | null = null;

/** wasm を一度だけ読み込んで初期化する。生成物が無ければ null。 */
export function loadUgoeWasm(): Promise<typeof UgoeWasm | null> {
  loading ??= (async () => {
    const load = modules["./generated/ugoe_wasm.js"];
    if (!load) return null;
    const wasm = await load();
    await wasm.default();
    return wasm;
  })();
  return loading;
}
