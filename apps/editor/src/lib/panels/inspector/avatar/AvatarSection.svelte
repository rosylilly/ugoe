<!--
  インスペクタ: アバタープロファイル (docs/editor.md#アバタープロファイルの編集)。何も選んでいないときに出す。
  準拠のレベル・メタデータ・利用許諾・標準の当たり判定を update_avatar_profile で編集し、
  準拠の検査 (check_avatar_profile) の結果を出す。トラッキングの上書き・視線・スロットはここでは編集しない。
-->
<script lang="ts">
import { Button, Checkbox, Notice, Select, TextField } from "@ugoe/ui";
import type { EditorDocument } from "../../../document.svelte";
import { errorText, messageText } from "../../../errors";
import type { AvatarCheck, AvatarMeta, Command } from "../../../generated/types";
import { t } from "../../../i18n";
import { ipc } from "../../../ipc";
import { elementLabel } from "../../../labels";
import {
  avatarPermissions,
  checkSummary,
  commercialUsages,
  createCommand,
  creditNotations,
  hitAreaCommand,
  levelName,
  MAX_LEVEL,
  metaCommand,
  modifications,
  type OptionalText,
  optionalText,
  parseLines,
  STANDARD_HIT_AREAS,
  usageFlags,
} from "./avatar";

interface Props {
  doc: EditorDocument;
}

let { doc }: Props = $props();

const model = $derived(doc.parsedModel);
const profile = $derived(model?.avatar ?? null);
const meta = $derived(profile?.meta ?? null);

let check = $state.raw<AvatarCheck | null>(null);
let checkError = $state<string | null>(null);
/** 最後に送った問い合わせの番号 (古い応答を捨てる)。 */
let request = 0;

$effect(() => {
  // ドキュメントが変わるたびに調べ直す (パラメータを足しただけでも満たすレベルが変わる)。
  void doc.revision;
  if (!doc.model) {
    check = null;
    return;
  }
  const id = ++request;
  ipc
    .checkAvatarProfile()
    .then((result) => {
      if (id !== request) return;
      check = result;
      checkError = null;
    })
    .catch((e) => {
      if (id !== request) return;
      check = null;
      checkError = errorText(e);
    });
});

const summary = $derived(check ? checkSummary(check) : null);

async function apply(command: Command | null): Promise<void> {
  if (command) await doc.apply(command);
}

function setMeta(patch: Partial<AvatarMeta>): Promise<void> {
  return meta ? apply(metaCommand(meta, patch)) : Promise.resolve();
}

function inputValue(event: Event): string {
  return (event.currentTarget as HTMLTextAreaElement).value;
}

const optionalTexts = $derived<{ key: OptionalText; label: string; placeholder?: string }[]>([
  { key: "version", label: t("inspector.avatar.meta.version") },
  { key: "copyright", label: t("inspector.avatar.meta.copyright") },
  { key: "contact", label: t("inspector.avatar.meta.contact") },
  // i18n-ignore: URL の書き出しの例
  { key: "license_url", label: t("inspector.avatar.meta.licenseUrl"), placeholder: "https://" },
]);

const levelOptions = $derived(
  Array.from({ length: MAX_LEVEL }, (_, i) => ({
    value: String(i + 1),
    label: t("inspector.avatar.levelOption", { level: i + 1, name: levelName(i + 1) ?? "" }),
  })),
);

const thumbnailOptions = $derived([
  { value: "", label: t("inspector.avatar.none") },
  ...(model?.textures ?? []).map((texture, i) => ({
    value: String(i),
    label: t("inspector.avatar.meta.thumbnailOption", { index: i, name: texture.name }),
  })),
]);

const hitAreaOptions = $derived([
  { value: "", label: t("inspector.avatar.none") },
  ...(model?.hit_areas ?? []).map((hit) => ({
    value: hit.id,
    label: t("inspector.avatar.hitAreas.option", {
      id: hit.id,
      mesh: elementLabel(model?.meshes.find((m) => m.id === hit.mesh)?.name ?? null, hit.mesh),
    }),
  })),
]);

const ids = $props.id();
</script>

{#if model}
  <section class="avatar">
    <h3>{t("inspector.avatar.title")}</h3>
    {#if !profile || !meta}
      <p class="muted">{t("inspector.avatar.empty")}</p>
      <Button small onclick={() => apply(createCommand(model.meta.name))}
        >{t("inspector.avatar.create")}</Button
      >
    {:else}
      <div class="ed-field">
        <span id="{ids}-level">{t("inspector.avatar.level")}</span>
        <Select
          aria-labelledby="{ids}-level"
          value={String(profile.level)}
          options={levelOptions}
          onchange={(v) => apply({ op: "update_avatar_profile", level: Number(v) })}
        />
      </div>

      <h4>{t("inspector.avatar.meta.title")}</h4>
      <div class="ed-field">
        <label for="{ids}-name">{t("inspector.avatar.meta.name")}</label>
        <TextField
          id="{ids}-name"
          value={meta.name}
          onchange={(value) => setMeta({ name: value.trim() })}
        />
      </div>
      <div class="column">
        <label for="{ids}-authors">{t("inspector.avatar.meta.authors")}</label>
        <textarea
          id="{ids}-authors"
          rows="2"
          value={meta.authors.join("\n")}
          onchange={(e) => setMeta({ authors: parseLines(inputValue(e)) })}
        ></textarea>
      </div>
      {#each optionalTexts as field (field.key)}
        <div class="ed-field">
          <label for="{ids}-{field.key}">{field.label}</label>
          <TextField
            id="{ids}-{field.key}"
            value={meta[field.key] ?? ""}
            placeholder={field.placeholder}
            onchange={(value) => setMeta({ [field.key]: optionalText(value) })}
          />
        </div>
      {/each}
      <div class="column">
        <label for="{ids}-references">{t("inspector.avatar.meta.references")}</label>
        <textarea
          id="{ids}-references"
          rows="2"
          value={meta.references.join("\n")}
          onchange={(e) => setMeta({ references: parseLines(inputValue(e)) })}
        ></textarea>
      </div>
      <div class="ed-field">
        <span id="{ids}-thumbnail">{t("inspector.avatar.meta.thumbnail")}</span>
        <Select
          aria-labelledby="{ids}-thumbnail"
          value={meta.thumbnail === null ? "" : String(meta.thumbnail)}
          options={thumbnailOptions}
          onchange={(v) => setMeta({ thumbnail: v === "" ? null : Number(v) })}
        />
      </div>

      <h4>{t("inspector.avatar.permissions.title")}</h4>
      <div class="ed-field">
        <span id="{ids}-avatar-permission">{t("inspector.avatar.permissions.avatar")}</span>
        <Select
          aria-labelledby="{ids}-avatar-permission"
          value={meta.avatar_permission}
          options={avatarPermissions()}
          onchange={(v) => setMeta({ avatar_permission: v })}
        />
      </div>
      <div class="ed-field">
        <span id="{ids}-commercial">{t("inspector.avatar.permissions.commercial")}</span>
        <Select
          aria-labelledby="{ids}-commercial"
          value={meta.commercial_usage}
          options={commercialUsages()}
          onchange={(v) => setMeta({ commercial_usage: v })}
        />
      </div>
      <div class="ed-field">
        <span id="{ids}-credit">{t("inspector.avatar.permissions.credit")}</span>
        <Select
          aria-labelledby="{ids}-credit"
          value={meta.credit_notation}
          options={creditNotations()}
          onchange={(v) => setMeta({ credit_notation: v })}
        />
      </div>
      <div class="ed-field">
        <span id="{ids}-modification">{t("inspector.avatar.permissions.modification")}</span>
        <Select
          aria-labelledby="{ids}-modification"
          value={meta.modification}
          options={modifications()}
          onchange={(v) => setMeta({ modification: v })}
        />
      </div>
      <fieldset>
        <legend>{t("inspector.avatar.permissions.allow")}</legend>
        {#each usageFlags() as flag (flag.key)}
          <div class="check">
            <Checkbox
              label={flag.label}
              checked={meta[flag.key]}
              onchange={(v) => setMeta({ [flag.key]: v })}
            />
          </div>
        {/each}
      </fieldset>
      <div class="column">
        <label for="{ids}-third-party">{t("inspector.avatar.meta.thirdPartyLicenses")}</label>
        <textarea
          id="{ids}-third-party"
          rows="2"
          value={meta.third_party_licenses ?? ""}
          onchange={(e) => setMeta({ third_party_licenses: optionalText(inputValue(e)) })}
        ></textarea>
      </div>

      <h4>{t("inspector.avatar.hitAreas.title")}</h4>
      {#if model.hit_areas.length === 0}
        <p class="muted">{t("inspector.avatar.hitAreas.empty")}</p>
      {:else}
        {#each STANDARD_HIT_AREAS as name (name)}
          <div class="ed-field">
            <span id="{ids}-hit-{name}">{name}</span>
            <Select
              aria-labelledby="{ids}-hit-{name}"
              value={profile.hit_areas.find((a) => a.name === name)?.hit_area ?? ""}
              options={hitAreaOptions}
              onchange={(v) => apply(hitAreaCommand(profile.hit_areas, name, v === "" ? null : v))}
            />
          </div>
        {/each}
      {/if}

      <div class="remove">
        <Button small onclick={() => apply({ op: "set_avatar_profile", profile: null })}
          >{t("inspector.avatar.remove")}</Button
        >
      </div>
    {/if}

    <h4>{t("inspector.avatar.check.title")}</h4>
    {#if checkError}
      <p class="error ugoe-selectable">{checkError}</p>
    {:else if check && summary}
      <Notice tone={summary.ok ? "success" : "warning"}>{summary.text}</Notice>
      <!-- プロファイルがないときのエラーは「model has no avatar profile」だけで、見出しと同じなので出さない。 -->
      {#if check.claimed !== null && check.errors.length > 0}
        <ul class="issues errors ugoe-selectable">
          {#each check.errors as issue, i (i)}
            <li>{messageText(issue)}</li>
          {/each}
        </ul>
      {/if}
      {#if check.warnings.length > 0}
        <ul class="issues warnings">
          {#each check.warnings as issue, i (i)}
            <li>{messageText(issue)}</li>
          {/each}
        </ul>
      {/if}
    {/if}
  </section>
{/if}

<style>
.column textarea {
  min-height: var(--ugoe-control-height);
  padding: 0 var(--ugoe-space-2);
  border: 1px solid var(--ugoe-color-control-border);
  border-radius: var(--ugoe-radius-md);
  background: var(--ugoe-color-surface);
  color: var(--ugoe-color-text);
  font: inherit;
}

.column textarea {
  padding: var(--ugoe-space-1) var(--ugoe-space-2);
}

.column textarea:focus-visible {
  outline: var(--ugoe-focus-ring);
  outline-offset: 1px;
}

.column {
  display: flex;
  flex-direction: column;
  gap: var(--ugoe-space-1);
  margin: var(--ugoe-space-2) 0;
}

.check {
  margin: var(--ugoe-space-1) 0;
}

fieldset {
  margin: var(--ugoe-space-2) 0;
  padding: var(--ugoe-space-2) var(--ugoe-space-3);
  border: 1px solid var(--ugoe-color-border);
  border-radius: var(--ugoe-radius-md);
}

.remove {
  margin-top: var(--ugoe-space-3);
}

.issues {
  margin: var(--ugoe-space-2) 0;
  padding-left: 1.2em;
  overflow-wrap: anywhere;
}

.errors li {
  color: var(--ugoe-color-danger);
}

h3 {
  margin: var(--ugoe-space-3) 0 var(--ugoe-space-2);
}

h4 {
  margin: var(--ugoe-space-4) 0 var(--ugoe-space-1);
}
</style>
