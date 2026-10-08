// 属性の値とモデルのバイト列の解釈。DOM と wasm に触れないので、Node でテストする (attributes.test.ts)。

/** 描画に使うグラフィックス API。`"auto"` は WebGPU を試し、使えなければ WebGL2 にする。 */
export type Backend = "auto" | "webgpu" | "webgl2";

/** `backend` 属性の値。知らない値や省略は `"auto"`。 */
export function parseBackend(value: string | null | undefined): Backend {
  const v = value?.trim().toLowerCase();
  return v === "webgpu" || v === "webgl2" ? v : "auto";
}

/**
 * GPU の電力の希望 (`WebPuppet.create` の `power_preference`。crates/ugoe-wasm/README.md#gpu-の電力の希望-t0499)。
 * `"default"` は希望なし。希望なので、効くかはブラウザと OS が決める。
 */
export type PowerPreference = "default" | "low-power" | "high-performance";

/** `power-preference` 属性の値。知らない値や省略は `"default"`。 */
export function parsePowerPreference(value: string | null | undefined): PowerPreference {
  const v = value?.trim().toLowerCase();
  return v === "low-power" || v === "high-performance" ? v : "default";
}

/** `power-preference` 属性の値が知らない値か (省略・空・`default`・`low-power`・`high-performance` 以外)。警告を出すのに使う。 */
export function isUnknownPowerPreference(value: string | null | undefined): boolean {
  const v = value?.trim().toLowerCase();
  return !!v && v !== "default" && parsePowerPreference(v) === "default";
}

/**
 * `parameters` 属性の値 (`"AngleX: 15; EyeLOpen: 0.5"`) をパラメータの ID と値の組にする。
 *
 * - 区切りは `;` か改行、ID と値の区切りは `:` か `=`。ID は大文字と小文字を区別する
 *   (HTML の属性名は小文字になるので、パラメータごとの属性ではなく 1 つの属性にまとめた)。
 * - 値が数でない組と空の組は無視する。同じ ID が複数あれば後のものを使う。
 */
export function parseParameters(value: string | null | undefined): Map<string, number> {
  const result = new Map<string, number>();
  if (!value) return result;
  for (const entry of value.split(/[;\n]/)) {
    const m = entry.match(/^\s*([^:=\s]+)\s*[:=]\s*(\S+)\s*$/);
    if (!m) continue;
    const [, id, text] = m;
    const n = Number(text);
    if (id !== undefined && text !== undefined && Number.isFinite(n)) result.set(id, n);
  }
  return result;
}

/** パラメータの ID と値の組を `parameters` 属性の値にする (`parseParameters` の逆)。 */
export function formatParameters(values: Iterable<[string, number]>): string {
  return Array.from(values, ([id, v]) => `${id}: ${v}`).join("; ");
}

/**
 * バイト列が `.ugoe` バンドル (先頭が `UGOE`) か。そうでなければモデルの JSON として読む。
 * `docs/spec/file-formats.md` のコンテナのマジック。
 */
export function isBundle(bytes: Uint8Array): boolean {
  return bytes[0] === 0x55 && bytes[1] === 0x47 && bytes[2] === 0x4f && bytes[3] === 0x45;
}

/** モーションの優先度 (`WebPuppet.start_motion` の第 2 引数)。 */
export type MotionPriority = "idle" | "normal" | "force";

/**
 * `tracking-kept` 属性の値 (`"EyeLOpen EyeROpen"`) をパラメータの ID の列にする。区切りは空白か `,` か `;`。
 * 空の項目は除き、重なりは最初のものだけ残す。ID は大文字と小文字を区別する。
 */
export function parseIdList(value: string | null | undefined): string[] {
  if (!value) return [];
  return [...new Set(value.split(/[\s,;]+/).filter((id) => id !== ""))];
}

/** `motion-priority` 属性の値。知らない値や省略は `"idle"` (待機モーションとして流す)。 */
export function parseMotionPriority(value: string | null | undefined): MotionPriority {
  const v = value?.trim().toLowerCase();
  return v === "normal" || v === "force" ? v : "idle";
}

/**
 * `crossorigin` 属性の値を `src` の取得の資格情報にする (`<img crossorigin>` と同じ対応)。
 * `use-credentials` は `"include"` (別のオリジンにも Cookie などを送る)。省略・`anonymous`・知らない値は
 * `"same-origin"` (`fetch` の既定。同じオリジンにだけ送る)。
 */
export function parseCrossOrigin(value: string | null | undefined): RequestCredentials {
  return value?.trim().toLowerCase() === "use-credentials" ? "include" : "same-origin";
}
