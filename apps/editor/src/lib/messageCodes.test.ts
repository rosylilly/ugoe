// バックエンドのエラー・警告のコードと翻訳のカタログ (`errors.<code>`) の対応 (ADR 0029、
// docs/editor.md#エラーと警告のコード)。コードの一覧は Rust から生成する (`just gen`)。
import { describe, expect, it } from "vitest";
import en from "../../locales/en.json";
import ja from "../../locales/ja.json";
import { messageCodes } from "./generated/messageCodes";

type Catalog = { [key: string]: string | Catalog };

const CATALOGS: Record<string, Catalog> = { en, ja };

/** 文の中の引数の名前 (`{{name}}`・`{{name, number}}`)。 */
function placeholders(text: string): string[] {
  return [...text.matchAll(/\{\{\s*([^,}\s]+)\s*(?:,[^}]*)?\}\}/g)].map((m) => m[1] ?? "");
}

/** `errors.<code>` の文。複数形 (`_one`・`_other` など) ならそのすべて。無ければ空。 */
function texts(catalog: Catalog, code: string): string[] {
  const path = code.split(".");
  const last = path.pop() ?? "";
  let node: string | Catalog | undefined = catalog.errors;
  for (const part of path) {
    node = typeof node === "object" ? node[part] : undefined;
  }
  if (typeof node !== "object") return [];
  return Object.entries(node)
    .filter(
      ([key]) => key === last || new RegExp(`^${last}_(zero|one|two|few|many|other)$`).test(key),
    )
    .map(([, text]) => text)
    .filter((text): text is string => typeof text === "string");
}

describe("message catalog", () => {
  const codes = Object.entries(messageCodes);

  it("lists the backend codes", () => {
    expect(codes.length).toBeGreaterThan(150);
  });

  for (const [language, catalog] of Object.entries(CATALOGS)) {
    it(`has a text for every code in ${language}.json`, () => {
      const missing = codes.filter(([code]) => texts(catalog, code).length === 0).map(([c]) => c);
      expect(missing).toEqual([]);
    });

    it(`uses only the arguments of the code in ${language}.json`, () => {
      const unknown = codes.flatMap(([code, args]) =>
        texts(catalog, code).flatMap((text) =>
          placeholders(text)
            .filter((name) => !args.includes(name))
            .map((name) => `${code}: {{${name}}}`),
        ),
      );
      expect(unknown).toEqual([]);
    });
  }
});
