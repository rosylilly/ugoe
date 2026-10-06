// @vitest-environment happy-dom
// パラメータのスライダーの下のキーの印: 押せる範囲 (24px) のどこを押してもそのキーへ移り、隣の印と範囲が重なる所では
// 近い方のキーへ移る (docs/ui.md#そのほか、docs/testing.md#エディタの部品のテスト。T1634)。
// 一覧のグループの区切り: グループの順に見出しを出し、グループの無いパラメータは最後の「Other」にまとめる (T1630)。
// 区切りの開閉を覚える・絞り込みの欄 (T1646)。
import { MemoryStorage, useLocalStorage } from "@ugoe/i18n/testing";
import { flushSync } from "svelte";
import { beforeEach, describe, expect, it } from "vitest";
import { Selection } from "../selection.svelte";
import {
  all,
  FakeController,
  FakeDocument,
  render,
  testModel,
} from "../viewport/tools/test/harness.svelte";
import ParametersPanel from "./ParametersPanel.svelte";

/** 印の帯の幅 (CSS px)。値域 -30〜30 のキー -30・0・30 の印の中心は 8・108・208。 */
const WIDTH = 216;

function setup(keys: number[] = [-30, 0, 30]) {
  const doc = new FakeDocument();
  const model = JSON.parse(testModel());
  model.meshes[0].keyforms = {
    bindings: [{ parameter: "AngleX", keys }],
    keyforms: keys.map(() => ({ positions: [] })),
  };
  doc.model = { json: JSON.stringify(model), bundle: null };
  const selection = new Selection();
  selection.set([{ kind: "mesh", id: "Quad" }]);
  const viewport = new FakeController();
  const parameters = [
    {
      id: "AngleX",
      name: "Angle X",
      min: -30,
      max: 30,
      default: 0,
      kind: "normal" as const,
      mirror: null,
      group: null,
    },
  ];
  const { target } = render(ParametersPanel, {
    doc: doc.asDocument,
    selection,
    viewport: viewport.asController,
    parameters,
  });
  // happy-dom はレイアウトをしないので、帯の大きさを与える。
  const strip = target.querySelector<HTMLElement>(".markers");
  if (!strip) throw new Error("no marker strip");
  strip.getBoundingClientRect = () => new DOMRect(0, 0, WIDTH, 24);
  const markers = () => all<HTMLButtonElement>(target, "button.marker");
  return { viewport, markers };
}

/** 帯の左端から `x` の所でポインタで押す。 */
function clickAt(marker: HTMLButtonElement, x: number): void {
  marker.dispatchEvent(new MouseEvent("click", { bubbles: true, clientX: x, detail: 1 }));
  flushSync();
}

describe("ParametersPanel key markers", () => {
  it("draws one marker per key of the selected element", () => {
    const { markers } = setup();
    expect(markers().map((m) => m.getAttribute("aria-label"))).toEqual([
      "Go to key -30",
      "Go to key 0",
      "Go to key 30",
    ]);
  });

  it("moves to the key from either edge of the 24px target", () => {
    const { viewport, markers } = setup();
    const [, zero, last] = markers();
    if (!zero || !last) throw new Error("no markers");
    clickAt(zero, 108 - 11);
    expect(viewport.parameterValues.AngleX).toBe(0);
    clickAt(last, 208 + 11);
    expect(viewport.parameterValues.AngleX).toBe(30);
    clickAt(zero, 108 + 11);
    expect(viewport.parameterValues.AngleX).toBe(0);
  });

  it("picks the nearer key where two targets overlap", () => {
    // キー 0 と 3 の印の中心は 108 と 118 (押せる範囲が 14px 重なる)。後の印が上に描かれる。
    const { viewport, markers } = setup([0, 3]);
    const [zero, three] = markers();
    if (!zero || !three) throw new Error("no markers");
    clickAt(three, 110);
    expect(viewport.parameterValues.AngleX).toBe(0);
    clickAt(zero, 116);
    expect(viewport.parameterValues.AngleX).toBe(3);
  });

  it("moves to the marker's own key when pressed from the keyboard", () => {
    const { viewport, markers } = setup([0, 3]);
    const three = markers()[1];
    if (!three) throw new Error("no marker");
    // Enter・Space の click は detail が 0 で、位置を持たない。
    three.dispatchEvent(new MouseEvent("click", { bubbles: true, detail: 0 }));
    flushSync();
    expect(viewport.parameterValues.AngleX).toBe(3);
  });
});

describe("ParametersPanel groups", () => {
  let storage: MemoryStorage;
  beforeEach(() => {
    storage = new MemoryStorage();
    useLocalStorage(storage);
  });

  const param = (id: string, group: string | null) => ({
    id,
    name: id,
    min: 0,
    max: 1,
    default: 0,
    kind: "normal" as const,
    mirror: null,
    group,
  });

  function setupGroups(
    groups: { id: string; name: string }[],
    parameters: ReturnType<typeof param>[],
  ) {
    const doc = new FakeDocument();
    doc.model = {
      json: JSON.stringify({ ...JSON.parse(testModel()), parameter_groups: groups }),
      bundle: null,
    };
    const { target } = render(ParametersPanel, {
      doc: doc.asDocument,
      selection: new Selection(),
      viewport: new FakeController().asController,
      parameters,
    });
    return target;
  }

  /** 区切りの見出しと、その中のスライダーの名前 (一覧の下の「Groups」の節は除く)。 */
  function sections(target: HTMLElement): [string, string[]][] {
    return all<HTMLElement>(target, ".ugoe-section")
      .filter((s) => s.querySelector("input[type=range]") !== null)
      .map((s) => [
        s.querySelector(".ugoe-section__trigger")?.textContent?.trim() ?? "",
        all<HTMLInputElement>(s, "input[type=range]").map(
          (r) => r.getAttribute("aria-label") ?? "",
        ),
      ]);
  }

  it("divides the list by group in the groups' order, with the ungrouped ones last", () => {
    const target = setupGroups(
      [
        { id: "Body", name: "Body" },
        { id: "Face", name: "顔" },
        { id: "Empty", name: "Empty" },
      ],
      [
        param("AngleX", "Face"),
        param("Loose", null),
        param("BodyX", "Body"),
        param("Stray", "Missing"),
        param("EyeL", "Face"),
      ],
    );
    expect(sections(target)).toEqual([
      ["Body", ["BodyX"]],
      ["顔", ["AngleX", "EyeL"]],
      // 定義に無いグループを指すものもグループの無いものとまとめる。空のグループは出さない
      ["Other", ["Loose", "Stray"]],
    ]);
    // 見出しに数を出す
    const counts = all<HTMLElement>(target, ".ugoe-section__actions .count").map(
      (c) => c.textContent,
    );
    expect(counts).toEqual(["1", "2", "2"]);
  });

  it("lists the parameters without headings when no parameter is in a group", () => {
    const target = setupGroups(
      [{ id: "Face", name: "Face" }],
      [param("A", null), param("B", null)],
    );
    expect(sections(target)).toEqual([]);
    expect(
      all<HTMLInputElement>(target, "input[type=range]").map((r) => r.getAttribute("aria-label")),
    ).toEqual(["A", "B"]);
  });

  function trigger(target: HTMLElement, title: string): HTMLButtonElement {
    const found = all<HTMLButtonElement>(target, ".ugoe-section__trigger").find(
      (b) => b.textContent?.trim() === title,
    );
    if (!found) throw new Error(`no section ${title}`);
    return found;
  }

  function type(target: HTMLElement, text: string): void {
    const input = target.querySelector<HTMLInputElement>('input[type="search"]');
    if (!input) throw new Error("no filter");
    input.value = text;
    input.dispatchEvent(new Event("input", { bubbles: true }));
    flushSync();
  }

  const model = () =>
    [
      [
        { id: "Face", name: "Face" },
        { id: "Body", name: "Body" },
      ],
      [
        param("AngleX", "Face"),
        param("EyeL", "Face"),
        param("BodyX", "Body"),
        param("Loose", null),
      ],
    ] as const;

  it("remembers closed sections across opening the panel again", () => {
    const [groups, parameters] = model();
    const first = setupGroups([...groups], [...parameters]);
    trigger(first, "Face").click();
    trigger(first, "Other").click();
    flushSync();
    expect(storage.getItem("ugoe.editor.parameterSectionsClosed")).toBe('["Face",""]');
    first.remove();

    const again = setupGroups([...groups], [...parameters]);
    expect(
      ["Face", "Body", "Other"].map((title) => trigger(again, title).getAttribute("aria-expanded")),
    ).toEqual(["false", "true", "false"]);
    trigger(again, "Face").click();
    flushSync();
    expect(storage.getItem("ugoe.editor.parameterSectionsClosed")).toBe('[""]');
  });

  it("filters by name and ID and shows the matches in closed sections too", () => {
    storage.setItem("ugoe.editor.parameterSectionsClosed", '["Face"]');
    const [groups, parameters] = model();
    const target = setupGroups(
      [...groups],
      [...parameters.map((p) => (p.id === "EyeL" ? { ...p, name: "Left eye" } : p))],
    );
    const filter = target.querySelector<HTMLInputElement>('input[type="search"]');
    expect(filter?.getAttribute("aria-label")).toBe("Filter the parameters");
    expect(trigger(target, "Face").getAttribute("aria-expanded")).toBe("false");

    // 名前で
    type(target, "left");
    expect(sections(target)).toEqual([["Face", ["Left eye"]]]);
    expect(trigger(target, "Face").getAttribute("aria-expanded")).toBe("true");
    // ID で (大文字と小文字を区別しない)
    type(target, "bodyx");
    expect(sections(target)).toEqual([["Body", ["BodyX"]]]);

    // 絞り込み中に閉じても覚えない
    type(target, "e");
    trigger(target, "Face").click();
    flushSync();
    expect(storage.getItem("ugoe.editor.parameterSectionsClosed")).toBe('["Face"]');

    // 語を消すと覚えた開閉に戻る
    type(target, "");
    expect(
      ["Face", "Body", "Other"].map((title) =>
        trigger(target, title).getAttribute("aria-expanded"),
      ),
    ).toEqual(["false", "true", "true"]);
    expect(target.querySelector('[role="status"]')).toBeNull();
  });

  it("says when nothing matches", () => {
    const [groups, parameters] = model();
    const target = setupGroups([...groups], [...parameters]);
    type(target, "nothing");
    expect(sections(target)).toEqual([]);
    expect(target.querySelector('[role="status"]')?.textContent).toBe(
      'No parameter matches "nothing".',
    );
  });

  it("filters the list without headings too", () => {
    const target = setupGroups([], [param("AngleX", null), param("BodyX", null)]);
    type(target, "angle");
    expect(
      all<HTMLInputElement>(target, "input[type=range]").map((r) => r.getAttribute("aria-label")),
    ).toEqual(["AngleX"]);
  });
});
