// @ugoe/ui: エディタ・ビューア・スタジオで共有する UI の部品 (docs/ui.md、ADR 0029)。
// トークンは別に読む: `import "@ugoe/ui/tokens.css";`
// アプリは Bits UI を直接 import せず、ここから出す部品だけを使う。
// 出すのは部品と、アプリやほかのパッケージが使う関数と型だけ (名前で出す。部品の内部の関数は出さない。T0920)。

export { numberFieldCommit } from "./commit";
export { default as AlertDialog } from "./components/AlertDialog.svelte";
export { default as Button } from "./components/Button.svelte";
export { default as Checkbox } from "./components/Checkbox.svelte";
export { default as ColorPicker } from "./components/ColorPicker.svelte";
export { default as ContextMenu } from "./components/ContextMenu.svelte";
export { default as Dialog } from "./components/Dialog.svelte";
export { default as DropZone } from "./components/DropZone.svelte";
export { default as Icon, type IconName } from "./components/Icon.svelte";
export { default as IconButton } from "./components/IconButton.svelte";
export { default as Kbd } from "./components/Kbd.svelte";
export { default as Menu } from "./components/Menu.svelte";
export { default as Notice } from "./components/Notice.svelte";
export { default as NumberField } from "./components/NumberField.svelte";
export { default as Popover } from "./components/Popover.svelte";
export { default as Section } from "./components/Section.svelte";
export { default as SegmentedControl } from "./components/SegmentedControl.svelte";
export { default as Select } from "./components/Select.svelte";
export { default as Slider } from "./components/Slider.svelte";
export { default as Tabs } from "./components/Tabs.svelte";
export { default as TextField } from "./components/TextField.svelte";
export { default as ThemeSetting } from "./components/ThemeSetting.svelte";
export { default as Toaster } from "./components/Toaster.svelte";
export { default as Toggle } from "./components/Toggle.svelte";
export { default as Toolbar } from "./components/Toolbar.svelte";
export { default as Tooltip } from "./components/Tooltip.svelte";
export { default as TooltipProvider } from "./components/TooltipProvider.svelte";
export { mergeAttributes, type TriggerAttributes } from "./props";
export { currentTheme, initTheme, loadThemePreference, type Theme } from "./theme.svelte";
export { showToast, type ToastOptions } from "./toast.svelte";
export type {
  ButtonBaseProps,
  ButtonVariant,
  MenuEntry,
  Option,
  Side,
  Tone,
} from "./types";
