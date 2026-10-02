<script lang="ts">
// 設定の画面 (ビューアの SettingsDialog と同じ形。T0907)。画面の言語 (OS に合わせる・日本語・English)・テーマ
// (OS に合わせる・ライト・ダーク)・ライセンスの表示。選んだ言語は settings.ts、テーマは @ugoe/ui の ThemeSetting が覚える。
// ライセンスは、このダイアログの上に LicenseDialog を重ねて開く (docs/ui.md#エディタの中の共通の形 の「重なるダイアログ」)。
import { getLocalePreference, LANGUAGES, type LocalePreference, setLocale } from "@ugoe/i18n";
import { Button, Dialog, Select, ThemeSetting } from "@ugoe/ui";
import LicenseDialog from "./LicenseDialog.svelte";
import { t } from "./lib/i18n";
import { saveLocalePreference } from "./lib/settings";

let { open = $bindable(false) }: { open?: boolean } = $props();

let locale = $state<LocalePreference>(getLocalePreference());
let licenseOpen = $state(false);

// 言語の名前は、その言語自身で書く (docs/ui.md#文体)。
let options = $derived([
  { value: "system" as LocalePreference, label: t("settings.language.system") },
  ...LANGUAGES.map((l) => ({ value: l.tag as LocalePreference, label: l.name })),
]);

function changeLocale(value: LocalePreference) {
  locale = value;
  setLocale(value);
  saveLocalePreference(value);
}
</script>

<Dialog bind:open title={t("settings.title")} size="sm">
  <Select label={t("settings.language.label")} value={locale} {options} onchange={changeLocale} />
  <p class="hint">{t("settings.language.description")}</p>
  <div class="theme"><ThemeSetting /></div>
  <div class="license">
    <Button onclick={() => (licenseOpen = true)}>{t("settings.license.open")}</Button>
    <p class="hint">{t("settings.license.description")}</p>
  </div>
</Dialog>

<LicenseDialog bind:open={licenseOpen} />

<style>
  .hint {
    margin: var(--ugoe-space-2) 0 0;
    font-size: var(--ugoe-font-size-sm);
    color: var(--ugoe-color-text-muted);
  }
  .theme,
  .license {
    margin-top: var(--ugoe-space-4);
  }
</style>
