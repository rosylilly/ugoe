<!--
  ThemeSetting: テーマの設定 (OS に合わせる・ライト・ダーク)。選ぶとすぐに切り替えて覚える (theme.svelte.ts)。
  アプリは起動時に `initTheme("<アプリ>.theme")` を呼んでおく。文言はこのパッケージのカタログ (`theme.*`) から引く。
-->
<script lang="ts">
import { t } from "../i18n";
import { getThemePreference, setThemePreference, type ThemePreference } from "../theme.svelte";
import SegmentedControl from "./SegmentedControl.svelte";

interface Props {
  /** 見えるラベル。既定は「テーマ」。 */
  label?: string;
  /** 切り替えた後に呼ぶ。 */
  onchange?: (value: ThemePreference) => void;
}

let { label, onchange }: Props = $props();

// 3 つなので SegmentedControl (設定のダイアログの中でも body に出す一覧に隠れない)。
const options = $derived([
  { value: "system" as ThemePreference, label: t("theme.system") },
  { value: "light" as ThemePreference, label: t("theme.light") },
  { value: "dark" as ThemePreference, label: t("theme.dark") },
]);

function change(value: ThemePreference) {
  setThemePreference(value);
  onchange?.(value);
}
</script>

<SegmentedControl
  label={label ?? t("theme.label")}
  value={getThemePreference()}
  {options}
  onchange={change}
/>
