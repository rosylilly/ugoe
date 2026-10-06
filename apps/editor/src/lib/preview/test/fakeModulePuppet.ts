// 着せ替えの WebPuppet の偽物 (DressUp と DressUpSection のテストで使う。docs/testing.md#エディタの部品のテスト)。
import type { WebPuppet } from "../../viewport/wasm";

/** ugoe-wasm の DressUpError と同じ形のエラー。 */
export function dressUpError(code: string, fields: Record<string, string>): Error {
  return Object.assign(new Error(`${code} happened`), { name: "DressUpError", code, ...fields });
}

/**
 * 着せ替えの WebPuppet の偽物。バイト列はモジュールの名前の文字列で、`slots` にスロットを持つ。
 * `broken` のスロットのモジュールは付けられない (付けた後にスロットが消えた、などの代わり)。
 * `state.lost` の間は GPU を失った WebPuppet で、読み込みと付け外しが `device_lost` を投げる (ugoe-wasm と同じ)。
 */
export function fakePuppet(slotOf: Record<string, string>) {
  const loaded: string[] = [];
  let attached: string[] = [];
  const broken = new Set<string>();
  const state = { lost: false };
  /** 直前の `reload` で付け直せなかったもの (`detached_modules_json`)。 */
  let detached: unknown[] = [];
  const lost = () => {
    if (state.lost) throw dressUpError("device_lost", {});
  };
  const check = (names: string[]) => {
    lost();
    for (const name of names) {
      if (!loaded.includes(name)) throw dressUpError("not_loaded", { module: name });
      const slot = slotOf[name] ?? "?";
      if (broken.has(slot)) throw dressUpError("unknown_slot", { module: name, slot });
    }
  };
  const puppet = {
    has_model: () => true,
    is_lost: () => state.lost,
    load_module(bytes: Uint8Array) {
      lost();
      const name = new TextDecoder().decode(bytes);
      if (!(name in slotOf)) throw dressUpError("invalid_file", {});
      if (!loaded.includes(name)) loaded.push(name);
      return name;
    },
    unload_module(name: string) {
      lost();
      attached = attached.filter((n) => n !== name);
      const i = loaded.indexOf(name);
      if (i >= 0) loaded.splice(i, 1);
    },
    attach_module(name: string) {
      check([name]);
      attached = [...attached.filter((n) => slotOf[n] !== slotOf[name] && n !== name), name];
    },
    detach_module(name: string) {
      lost();
      if (!attached.includes(name)) throw dressUpError("not_attached", { module: name });
      attached = attached.filter((n) => n !== name);
    },
    set_modules(names: string[]) {
      check(names);
      attached = [...names];
    },
    attached_modules: () => [...attached],
    detached_modules_json: () => JSON.stringify(detached),
    modules_json: () =>
      JSON.stringify(
        loaded.map((name) => ({
          name,
          slot: slotOf[name],
          requires: {},
          attached: attached.includes(name),
          textures: 1,
          warnings: [],
        })),
      ),
  };
  /** モデルの読み直し (T0123): 付けていたものを 1 つずつ付け直し、付けられないものを外して一覧に残す。 */
  const reload = () => {
    const before = attached;
    attached = [];
    detached = [];
    for (const name of before) {
      try {
        check([name]);
        attached.push(name);
      } catch (e) {
        const { code, module, slot } = e as Error & Record<string, string>;
        detached.push({
          module: name,
          slot: slotOf[name],
          error: { code, message: (e as Error).message, module, slot },
          warning: `module \`${name}\` was detached: ${(e as Error).message}`,
        });
      }
    }
  };
  return {
    puppet: puppet as unknown as WebPuppet,
    broken,
    state,
    reload,
    attached: () => attached,
    loaded: () => [...loaded],
  };
}
