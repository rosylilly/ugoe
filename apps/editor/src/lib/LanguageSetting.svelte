<!--
  設定の「General」の画面の言語 (docs/editor.md#言語)。「OS に合わせる」(既定)・言語自身の名前で書いた言語から選び、
  すぐに切り替えて覚える (lib/locale.ts)。
-->
<script lang="ts">
import { getLocalePreference, LANGUAGES, type LocalePreference } from "@ugoe/i18n";
import { SegmentedControl } from "@ugoe/ui";
import { t } from "./i18n";
import { changeLocale } from "./locale";

const options = $derived([
  { value: "system" as LocalePreference, label: t("settings.language.system") },
  ...LANGUAGES.map((l) => ({ value: l.tag as LocalePreference, label: l.name })),
]);
</script>

<div class="language">
  <!-- 選択肢が 4 つまでなので SegmentedControl (設定の画面は <dialog> の最上位の層にあり、body に出す Select の一覧が隠れるため)。 -->
  <SegmentedControl
    label={t("settings.language.label")}
    value={getLocalePreference()}
    {options}
    onchange={changeLocale}
  />
  <p class="hint">{t("settings.language.description")}</p>
</div>

<style>
  .language {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-2);
  }
  .hint {
    margin: 0;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }
</style>
