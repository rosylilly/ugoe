// @vitest-environment happy-dom
// パラメータのグループの節: 作る・名前を変える・消す・並べ替える (▲▼・ドラッグ・Alt + ↑↓) がそれぞれコマンドを 1 つ送ること
// (docs/editor.md#パラメータのグループ、docs/testing.md#エディタの部品のテスト)。
import { flushSync } from "svelte";
import { describe, expect, it } from "vitest";
import type { ParameterGroup } from "../../generated/types";
import {
  all,
  FakeDocument,
  get,
  pointer,
  render,
  settle,
  testModel,
} from "../../viewport/tools/test/harness.svelte";
import ParameterGroups from "./ParameterGroups.svelte";

function setup(groups: ParameterGroup[], module = false) {
  const doc = new FakeDocument();
  doc.model = {
    json: JSON.stringify({ ...JSON.parse(testModel()), parameter_groups: groups }),
    bundle: null,
  };
  if (module) (doc as { module: unknown }).module = { base: "base.ugoeproj" };
  const parameters = [
    {
      id: "AngleX",
      name: "Angle X",
      min: -30,
      max: 30,
      default: 0,
      kind: "normal" as const,
      mirror: null,
      group: "Face",
    },
  ];
  const { target } = render(ParameterGroups, { doc: doc.asDocument, parameters });
  return { doc, target };
}

function click(element: Element): void {
  (element as HTMLElement).click();
  flushSync();
}

function buttonByText(target: HTMLElement, text: string): HTMLButtonElement {
  const button = [...target.querySelectorAll("button")].find((b) => b.textContent?.trim() === text);
  if (!button) throw new Error(`no button "${text}"`);
  return button;
}

function buttonByLabel(target: HTMLElement, label: string): HTMLButtonElement {
  return get<HTMLButtonElement>(target, `button[aria-label="${label}"]`);
}

function type(input: HTMLInputElement, text: string): void {
  input.value = text;
  input.dispatchEvent(new Event("input", { bubbles: true }));
  flushSync();
}

function submit(form: HTMLFormElement): void {
  form.dispatchEvent(new SubmitEvent("submit", { bubbles: true, cancelable: true }));
  flushSync();
}

/** 閉じている節を開く。 */
function openSection(target: HTMLElement): void {
  click(get(target, ".ugoe-section__trigger"));
}

const FACE = { id: "Face", name: "顔" };
const BODY = { id: "Body", name: "Body" };
const EYES = { id: "Eyes", name: "Eyes" };

/** happy-dom はレイアウトをしないので、行を上から高さ 30px ずつ並べたことにする。 */
function layoutRows(target: HTMLElement): HTMLLIElement[] {
  const rows = all<HTMLLIElement>(target, "li[data-group]");
  rows.forEach((li, i) => {
    li.getBoundingClientRect = () => new DOMRect(0, i * 30, 200, 30);
  });
  return rows;
}

/** 行 `row` の名前を押して、`y` まで動かす (離さない)。 */
function dragTo(row: HTMLLIElement, y: number): HTMLElement {
  const list = row.parentElement as HTMLElement;
  const start = row.getBoundingClientRect().top + 15;
  pointer(get(row, ".name"), "pointerdown", [10, start]);
  pointer(list, "pointermove", [10, start + 6], { buttons: 1 });
  pointer(list, "pointermove", [10, y], { buttons: 1 });
  return list;
}

describe("ParameterGroups", () => {
  it("shows the count in the heading and creates a group from the empty state", async () => {
    const { doc, target } = setup([]);
    expect(get(target, ".ugoe-section__trigger").textContent).toContain("Groups (0)");
    openSection(target);
    expect(target.textContent).toContain("No groups yet.");
    click(buttonByText(target, "New group"));
    const [id, name] = [...target.querySelectorAll<HTMLInputElement>("form input")];
    type(id as HTMLInputElement, " Face ");
    type(name as HTMLInputElement, "顔");
    submit(get<HTMLFormElement>(target, "form"));
    await settle();
    expect(doc.commands).toEqual([
      { op: "add_parameter_group", group: { id: "Face", name: "顔" } },
    ]);
    // 作れたら欄を閉じる。
    flushSync();
    expect(target.querySelector("form")).toBeNull();
  });

  it("opens the section and the form with the + button", () => {
    const { target } = setup([FACE]);
    click(buttonByLabel(target, "New parameter group"));
    expect(target.querySelectorAll("form input")).toHaveLength(2);
  });

  it("renames and deletes a group", async () => {
    const { doc, target } = setup([FACE, BODY]);
    openSection(target);
    expect(target.textContent).toContain("1 parameter");
    click(buttonByText(target, "顔 (Face)"));
    const input = get<HTMLInputElement>(target, "form input");
    expect(input.value).toBe("顔");
    type(input, "Head");
    submit(get<HTMLFormElement>(target, "form"));
    await settle();
    click(buttonByText(target, "顔 (Face)"));
    click(buttonByText(target, "Delete group"));
    await settle();
    expect(doc.commands).toEqual([
      { op: "rename_parameter_group", id: "Face", name: "Head" },
      { op: "remove_parameter_group", id: "Face" },
    ]);
  });

  it("moves groups up and down, and not past the ends", async () => {
    const { doc, target } = setup([FACE, BODY]);
    openSection(target);
    expect(buttonByLabel(target, "Move 顔 (Face) up").disabled).toBe(true);
    expect(buttonByLabel(target, "Move Body down").disabled).toBe(true);
    click(buttonByLabel(target, "Move 顔 (Face) down"));
    await settle();
    click(buttonByLabel(target, "Move Body up"));
    await settle();
    expect(doc.commands).toEqual([
      { op: "reorder_parameter_group", id: "Face", before: null },
      { op: "reorder_parameter_group", id: "Body", before: "Face" },
    ]);
  });

  it("moves a group by dragging it and shows where it drops", async () => {
    const { doc, target } = setup([FACE, BODY, EYES]);
    openSection(target);
    const [face, , eyes] = layoutRows(target);
    if (!face || !eyes) throw new Error("no rows");
    // Eyes を Face の上半分へ: Face の前に落とす
    const list = dragTo(eyes, 5);
    expect(eyes.classList.contains("dragging")).toBe(true);
    expect(face.classList.contains("drop-before")).toBe(true);
    pointer(list, "pointerup", [10, 5]);
    await settle();
    // Face を最後の行の下半分へ: 末尾に落とす
    dragTo(face, 85);
    expect(eyes.classList.contains("drop-after")).toBe(true);
    pointer(list, "pointerup", [10, 85]);
    await settle();
    expect(doc.commands).toEqual([
      { op: "reorder_parameter_group", id: "Eyes", before: "Face" },
      { op: "reorder_parameter_group", id: "Face", before: null },
    ]);
    // 離した直後のクリックで名前の編集欄を開かない
    expect(target.querySelector("form")).toBeNull();
  });

  it("sends nothing when a drag ends where it started or is cancelled with Esc", async () => {
    const { doc, target } = setup([FACE, BODY, EYES]);
    openSection(target);
    const [face, body] = layoutRows(target);
    if (!face || !body) throw new Error("no rows");
    // Body を Face の下半分 (= Body の前) へ: 動かない
    let list = dragTo(body, 20);
    expect(target.querySelector(".drop-before, .drop-after")).toBeNull();
    pointer(list, "pointerup", [10, 20]);
    await settle();
    list = dragTo(body, 5);
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    flushSync();
    expect(body.classList.contains("dragging")).toBe(false);
    pointer(list, "pointerup", [10, 5]);
    await settle();
    expect(doc.commands).toEqual([]);
  });

  it("does not start a drag from the up and down buttons", async () => {
    const { doc, target } = setup([FACE, BODY]);
    openSection(target);
    const [, body] = layoutRows(target);
    if (!body) throw new Error("no rows");
    const up = buttonByLabel(target, "Move Body up");
    pointer(up, "pointerdown", [10, 45]);
    pointer(body.parentElement as HTMLElement, "pointermove", [10, 0], { buttons: 1 });
    expect(body.classList.contains("dragging")).toBe(false);
    pointer(body.parentElement as HTMLElement, "pointerup", [10, 0]);
    await settle();
    expect(doc.commands).toEqual([]);
  });

  it("moves a group with Alt + arrow keys on its name", async () => {
    const { doc, target } = setup([FACE, BODY]);
    openSection(target);
    const name = buttonByText(target, "Body");
    expect(name.getAttribute("aria-keyshortcuts")).toBe("Alt+ArrowUp Alt+ArrowDown");
    // 修飾キーの無い矢印は何もしない
    name.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true }));
    name.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "ArrowUp",
        altKey: true,
        bubbles: true,
        cancelable: true,
      }),
    );
    await settle();
    // 端では送らない
    buttonByText(target, "顔 (Face)").dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "ArrowUp",
        altKey: true,
        bubbles: true,
        cancelable: true,
      }),
    );
    await settle();
    expect(doc.commands).toEqual([{ op: "reorder_parameter_group", id: "Body", before: "Face" }]);
  });

  it("is read-only in a module project", () => {
    const { target } = setup([FACE], true);
    expect(buttonByLabel(target, "New parameter group").disabled).toBe(true);
    openSection(target);
    expect(target.textContent).toContain("Parameter groups belong to the base model");
    // 一覧の欄とボタンは BaseLock の fieldset ごと無効になる。
    expect(get<HTMLFieldSetElement>(target, "fieldset.base-lock").disabled).toBe(true);
  });
});
