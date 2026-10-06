// Vitest の前に読む: 画面の文言を英語 (原文) にする。OS の言語に依らず、同じ文で検査できるようにする。
import { setLocale } from "@ugoe/i18n";
import { vi } from "vitest";

setLocale("en");

// 部品のテストは本物の wasm (`just wasm` の生成物) を読まない。生成物があると、テストの片付けで fetch が中断されて
// 処理されない rejection になり、テストが通っても Vitest が落ちる (T1641)。wasm が要るテストはファイルの中で
// 偽物を返す `vi.mock` を書き直す (`panels/timeline/timeline-state.test.ts` など)。
vi.mock("./lib/viewport/wasm", () => ({ loadUgoeWasm: vi.fn(async () => null) }));
