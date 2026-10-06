<script lang="ts">
// 設定の画面 (ビューアの SettingsDialog と同じ形。T0907。下の「閉じる」はエディタの設定と同じ。T0818)。画面の言語 (OS に合わせる・日本語・English)・テーマ
// (OS に合わせる・ライト・ダーク)・ライセンスの表示。選んだ言語は @ugoe/i18n の setLocalePreference、テーマは @ugoe/ui の ThemeSetting が覚える。
// ライセンスは、このダイアログの上に LicenseDialog を重ねて開く (docs/ui.md#エディタの中の共通の形 の「重なるダイアログ」)。
import {
  getLocalePreference,
  LANGUAGES,
  type LocalePreference,
  setLocalePreference,
} from "@ugoe/i18n";
import { Button, Dialog, Select, ThemeSetting } from "@ugoe/ui";
import LicenseDialog from "./LicenseDialog.svelte";
import { t } from "./lib/i18n";

let { open = $bindable(false) }: { open?: boolean } = $props();

let licenseOpen = $state(false);

// 言語の名前は、その言語自身で書く (docs/ui.md#文体)。
let options = $derived([
  { value: "system" as LocalePreference, label: t("settings.language.system") },
  ...LANGUAGES.map((l) => ({ value: l.tag as LocalePreference, label: l.name })),
]);
</script>

<Dialog bind:open title={t("settings.title")} size="sm">
  <Select label={t("settings.language.label")} value={getLocalePreference()} {options} onchange={setLocalePreference} />
  <p class="hint">{t("settings.language.description")}</p>
  <div class="theme"><ThemeSetting /></div>
  <div class="license">
    <Button onclick={() => (licenseOpen = true)}>{t("settings.license.open")}</Button>
    <p class="hint">{t("settings.license.description")}</p>
  </div>
  <!-- 閉じるボタンを下にも置く (エディタの設定のダイアログと同じ形。選んだものはすぐに効くので、決定のボタンは無い) -->
  {#snippet actions()}
    <Button onclick={() => (open = false)}>{t("settings.close")}</Button>
  {/snippet}
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
