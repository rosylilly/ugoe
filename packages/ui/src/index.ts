// @ugoe/ui: エディタ・ビューア・スタジオで共有する UI の部品 (docs/ui.md、ADR 0029)。
// トークンは別に読む: `import "@ugoe/ui/tokens.css";`
// アプリは Bits UI を直接 import せず、ここから出す部品だけを使う。

export { default as Button } from "./components/Button.svelte";
export { default as Checkbox } from "./components/Checkbox.svelte";
export { default as IconButton } from "./components/IconButton.svelte";
export { default as NumberField } from "./components/NumberField.svelte";
export { default as Section } from "./components/Section.svelte";
export { default as SegmentedControl } from "./components/SegmentedControl.svelte";
export { default as Select } from "./components/Select.svelte";
export { default as Slider } from "./components/Slider.svelte";
export { default as Tabs } from "./components/Tabs.svelte";
export { default as Toggle } from "./components/Toggle.svelte";
export { default as Toolbar } from "./components/Toolbar.svelte";
export * from "./number";
export type { ButtonBaseProps, ButtonVariant, Option } from "./types";
