import { setLocale } from "@ugoe/i18n";
import { beforeEach, describe, expect, it } from "vitest";
import { describeDressUpError } from "./dressUp";
import { describeLimitError, describeLoadError } from "./loadError";

// 文言は日本語の訳で確かめる (英語の原文を確かめる it は、その中で言語を切り替える)。
beforeEach(() => setLocale("ja"));

/** ugoe-wasm の `LoadError` と同じ形の `Error` (`name`・`code`・補足の値)。 */
function codedError(
  name: string,
  code: string,
  fields: Record<string, string | number> = {},
  text = "failed",
) {
  return Object.assign(new Error(text), { name, code, ...fields });
}

const loadError = (code: string, fields: Record<string, string | number> = {}, text = "failed") =>
  codedError("LoadError", code, fields, text);

describe("describeLoadError", () => {
  it("translates every code of crates/ugoe-wasm/README.md#読み込みの失敗-t0434", () => {
    expect(describeLoadError(loadError("invalid_file", {}, "not a ugoe file"))).toBe(
      "モデルとして読めないファイルです (not a ugoe file)",
    );
    expect(describeLoadError(loadError("unsupported_version", { found: 9, supported: 3 }))).toBe(
      "新しい版の ugoe で保存されたファイルです (ファイルの版 9。このアプリが読めるのは版 3 まで)。アプリを更新してから開いてください",
    );
    expect(describeLoadError(loadError("unsupported_features", { features: "x.a, x.b" }))).toBe(
      "このアプリが対応していない機能を使ったファイルです (x.a, x.b)。アプリを更新してから開いてください",
    );
    expect(describeLoadError(loadError("invalid_model", { errors: 1 }, "model: bad"))).toBe(
      "ファイルのモデルが正しくありません (model: bad)",
    );
    expect(describeLoadError(loadError("compile"))).toBe("このモデルを組み立てられません (failed)");
    expect(describeLoadError(loadError("texture", {}, "full"))).toBe(
      "テクスチャを登録できません (full)",
    );
    expect(describeLoadError(loadError("device_lost"))).toContain("GPU が止まりました");
  });

  it("chooses the texture or the normal map and the chunk or the total limit", () => {
    const image = { texture: 1, width: 20000, height: 2, max: 16384 };
    expect(
      describeLoadError(loadError("image_limit_exceeded", { image: "texture", ...image })),
    ).toBe(
      "ファイルを開きませんでした: テクスチャ 1 の画像が 20,000 × 2 ピクセルで、一辺の上限の 16,384 ピクセルを超えます",
    );
    expect(
      describeLoadError(loadError("texture_too_large", { image: "normal_map", ...image })),
    ).toBe(
      "テクスチャ 1 の法線マップが 20,000 × 2 ピクセルで、この GPU で使える大きさ (一辺 16,384 ピクセル) を超えます",
    );
    expect(
      describeLoadError(loadError("invalid_texture", { image: "normal_map", texture: 0 }, "bad")),
    ).toBe("テクスチャ 0 の法線マップをデコードできません (bad)");
    expect(
      describeLoadError(
        loadError("read_limit_exceeded", { limit: "chunk_decompressed", chunk: "TEXR", max: 1024 }),
      ),
    ).toBe("ファイルを開きませんでした: チャンク TEXR を展開すると上限の 1,024 バイトを超えます");
    expect(
      describeLoadError(
        loadError("read_limit_exceeded", { limit: "total_decompressed", chunk: "MODL", max: 1024 }),
      ),
    ).toContain("合計で上限の 1,024 バイト");
  });

  it("uses the English source text in English", () => {
    setLocale("en");
    expect(describeLoadError(loadError("unsupported_version", { found: 9, supported: 3 }))).toBe(
      "This file was saved by a newer version of ugoe (file version 9; this app reads up to version 3). Update the app to open it.",
    );
  });

  it("keeps the message of unknown codes and other errors", () => {
    expect(describeLoadError(loadError("future_code", {}, "something new"))).toBe("something new");
    expect(describeLoadError(codedError("DressUpError", "invalid_file", {}, "module"))).toBe(
      "module",
    );
    expect(describeLoadError(new Error("plain"))).toBe("plain");
    expect(describeLoadError("text")).toBe("text");
  });
});

describe("describeLimitError", () => {
  it("translates the limit errors of load_module like the model ones", () => {
    const e = codedError("DressUpError", "read_limit_exceeded", {
      limit: "chunk_decompressed",
      chunk: "TEXR",
      max: 1024,
    });
    expect(describeLimitError(e)).toBe(
      "ファイルを開きませんでした: チャンク TEXR を展開すると上限の 1,024 バイトを超えます",
    );
    expect(describeDressUpError(e)).toBe(describeLimitError(e));
    expect(describeLimitError(codedError("DressUpError", "invalid_file"))).toBeNull();
    expect(describeLimitError(loadError("invalid_file"))).toBeNull();
  });
});
