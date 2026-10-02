import { flushSync } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import ThemeSetting from "./components/ThemeSetting.svelte";
import { click, get, render } from "./test/render";
import {
  currentTheme,
  getThemePreference,
  initTheme,
  loadThemePreference,
  saveThemePreference,
  setThemePreference,
} from "./theme.svelte";

const KEY = "ugoe-test.theme";

/** OS のテーマを差し替えられる matchMedia。 */
function fakeMatchMedia(dark: boolean) {
  const listeners = new Set<(e: MediaQueryListEvent) => void>();
  const media = {
    matches: dark,
    addEventListener: (_: string, fn: (e: MediaQueryListEvent) => void) => listeners.add(fn),
    removeEventListener: (_: string, fn: (e: MediaQueryListEvent) => void) => listeners.delete(fn),
  };
  vi.stubGlobal(
    "matchMedia",
    vi.fn(() => media),
  );
  return {
    listeners,
    set(next: boolean) {
      media.matches = next;
      for (const fn of listeners) fn({ matches: next } as MediaQueryListEvent);
      flushSync();
    },
  };
}

let stop: (() => void) | null = null;

beforeEach(() => {
  localStorage.clear();
  delete document.documentElement.dataset.theme;
});

afterEach(() => {
  stop?.();
  stop = null;
  vi.unstubAllGlobals();
});

describe("theme preference storage", () => {
  it("falls back to system for missing or unknown values", () => {
    expect(loadThemePreference(KEY)).toBe("system");
    localStorage.setItem(KEY, "sepia");
    expect(loadThemePreference(KEY)).toBe("system");
    saveThemePreference(KEY, "dark");
    expect(loadThemePreference(KEY)).toBe("dark");
  });

  it("works without a usable storage", () => {
    const broken = {
      getItem: () => {
        throw new Error("denied");
      },
      setItem: () => {
        throw new Error("denied");
      },
    } as unknown as Storage;
    expect(loadThemePreference(KEY, broken)).toBe("system");
    expect(() => saveThemePreference(KEY, "light", broken)).not.toThrow();
  });
});

describe("initTheme / setThemePreference", () => {
  it("applies the saved theme to <html> and removes it for system", () => {
    fakeMatchMedia(false);
    localStorage.setItem(KEY, "dark");
    stop = initTheme(KEY);
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(getThemePreference()).toBe("dark");

    setThemePreference("system");
    expect(document.documentElement.dataset.theme).toBeUndefined();
    expect(localStorage.getItem(KEY)).toBe("system");

    setThemePreference("light");
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(localStorage.getItem(KEY)).toBe("light");
  });

  it("follows the OS theme only while set to system", () => {
    const os = fakeMatchMedia(false);
    stop = initTheme(KEY);
    expect(currentTheme()).toBe("light");
    os.set(true);
    expect(currentTheme()).toBe("dark");
    setThemePreference("light");
    expect(currentTheme()).toBe("light");
    stop();
    stop = null;
    expect(os.listeners.size).toBe(0);
  });
});

describe("ThemeSetting", () => {
  it("offers the three choices and switches the theme", () => {
    fakeMatchMedia(false);
    stop = initTheme(KEY);
    const onchange = vi.fn();
    const root = render(ThemeSetting, { onchange });
    const radios = [...root.querySelectorAll<HTMLElement>("[role=radio]")];
    expect(radios.map((r) => r.textContent?.trim())).toEqual(["Match the OS", "Light", "Dark"]);
    expect(get(root, "[role=radio][aria-checked=true]").textContent?.trim()).toBe("Match the OS");

    click(radios[2] as HTMLElement);
    expect(onchange).toHaveBeenCalledWith("dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(localStorage.getItem(KEY)).toBe("dark");
  });
});
