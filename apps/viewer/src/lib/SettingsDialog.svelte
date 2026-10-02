<script lang="ts">
// 設定の画面。いまは画面の言語 (OS に合わせる・日本語・English) だけ。選んだ言語は覚える (settings.ts)。
import { getLocalePreference, LANGUAGES, type LocalePreference, setLocale } from "@ugoe/i18n";
import { Dialog, Select } from "@ugoe/ui";
import { t } from "./i18n";
import { saveLocalePreference } from "./settings";

let { open = $bindable(false) }: { open?: boolean } = $props();

let locale = $state<LocalePreference>(getLocalePreference());

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
</Dialog>

<style>
  .hint {
    margin: var(--ugoe-space-2) 0 0;
    font-size: var(--ugoe-font-size-sm);
    color: var(--ugoe-color-text-muted);
  }
</style>
