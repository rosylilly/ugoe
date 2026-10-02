<!--
  インスペクタ: アバタープロファイル (docs/editor.md#アバタープロファイルの編集)。何も選んでいないときに出す。
  準拠のレベル・メタデータ・利用許諾・標準の当たり判定を update_avatar_profile で編集し、
  準拠の検査 (check_avatar_profile) の結果を出す。トラッキングの上書き・視線・スロットはここでは編集しない。
-->
<script lang="ts">
import type { EditorDocument } from "../../../document.svelte";
import { errorText } from "../../../errors";
import type { AvatarCheck, AvatarMeta, Command, Model } from "../../../generated/types";
import { ipc } from "../../../ipc";
import { elementLabel } from "../../../labels";
import {
  AVATAR_PERMISSIONS,
  COMMERCIAL_USAGES,
  CREDIT_NOTATIONS,
  checkSummary,
  createCommand,
  hitAreaCommand,
  LEVEL_NAMES,
  MAX_LEVEL,
  MODIFICATIONS,
  metaCommand,
  type OptionalText,
  optionalText,
  parseLines,
  STANDARD_HIT_AREAS,
  USAGE_FLAGS,
} from "./avatar";

interface Props {
  doc: EditorDocument;
}

let { doc }: Props = $props();

function parse(json: string | undefined): Model | null {
  if (!json) return null;
  try {
    return JSON.parse(json) as Model;
  } catch {
    return null;
  }
}

const model = $derived(parse(doc.model?.json));
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

function setText(field: OptionalText, event: Event): Promise<void> {
  return setMeta({ [field]: optionalText((event.currentTarget as HTMLInputElement).value) });
}

function inputValue(event: Event): string {
  return (event.currentTarget as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement).value;
}

const optionalTexts: { key: OptionalText; label: string; placeholder?: string }[] = [
  { key: "version", label: "Version" },
  { key: "copyright", label: "Copyright" },
  { key: "contact", label: "Contact" },
  { key: "license_url", label: "License URL", placeholder: "https://" },
];
</script>

{#if model}
  <section class="avatar">
    <h3>Avatar profile</h3>
    {#if !profile || !meta}
      <p class="muted">
        The model has no avatar profile. Create one to declare the conformance level, the metadata and the
        usage permissions. Permissions start at the most restrictive values.
      </p>
      <button onclick={() => apply(createCommand(model.meta.name))}>Create avatar profile</button>
    {:else}
      <label class="row">
        <span>Level</span>
        <select
          value={profile.level}
          onchange={(e) => apply({ op: "update_avatar_profile", level: Number(inputValue(e)) })}
        >
          {#each Array.from({ length: MAX_LEVEL }, (_, i) => i + 1) as level (level)}
            <option value={level}>{level} — {LEVEL_NAMES[level]}</option>
          {/each}
        </select>
      </label>

      <h4>Metadata</h4>
      <label class="row">
        <span>Name</span>
        <input value={meta.name} onchange={(e) => setMeta({ name: inputValue(e).trim() })} />
      </label>
      <label class="column">
        <span>Authors (one per line)</span>
        <textarea
          rows="2"
          value={meta.authors.join("\n")}
          onchange={(e) => setMeta({ authors: parseLines(inputValue(e)) })}
        ></textarea>
      </label>
      {#each optionalTexts as field (field.key)}
        <label class="row">
          <span>{field.label}</span>
          <input
            value={meta[field.key] ?? ""}
            placeholder={field.placeholder}
            onchange={(e) => setText(field.key, e)}
          />
        </label>
      {/each}
      <label class="column">
        <span>References (one per line)</span>
        <textarea
          rows="2"
          value={meta.references.join("\n")}
          onchange={(e) => setMeta({ references: parseLines(inputValue(e)) })}
        ></textarea>
      </label>
      <label class="row">
        <span>Thumbnail</span>
        <select
          value={meta.thumbnail === null ? "" : String(meta.thumbnail)}
          onchange={(e) => {
            const v = inputValue(e);
            return setMeta({ thumbnail: v === "" ? null : Number(v) });
          }}
        >
          <option value="">None</option>
          {#each model.textures as texture, i (i)}
            <option value={String(i)}>{i}: {texture.name}</option>
          {/each}
        </select>
      </label>

      <h4>Permissions</h4>
      <label class="row">
        <span>Who may use it as an avatar</span>
        <select
          value={meta.avatar_permission}
          onchange={(e) => setMeta({ avatar_permission: inputValue(e) as AvatarMeta["avatar_permission"] })}
        >
          {#each AVATAR_PERMISSIONS as option (option.value)}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      </label>
      <label class="row">
        <span>Commercial use</span>
        <select
          value={meta.commercial_usage}
          onchange={(e) => setMeta({ commercial_usage: inputValue(e) as AvatarMeta["commercial_usage"] })}
        >
          {#each COMMERCIAL_USAGES as option (option.value)}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      </label>
      <label class="row">
        <span>Credit</span>
        <select
          value={meta.credit_notation}
          onchange={(e) => setMeta({ credit_notation: inputValue(e) as AvatarMeta["credit_notation"] })}
        >
          {#each CREDIT_NOTATIONS as option (option.value)}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      </label>
      <label class="row">
        <span>Modification</span>
        <select
          value={meta.modification}
          onchange={(e) => setMeta({ modification: inputValue(e) as AvatarMeta["modification"] })}
        >
          {#each MODIFICATIONS as option (option.value)}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      </label>
      <fieldset>
        <legend>Allow</legend>
        {#each USAGE_FLAGS as flag (flag.key)}
          <label class="check">
            <input
              type="checkbox"
              checked={meta[flag.key]}
              onchange={(e) => setMeta({ [flag.key]: (e.currentTarget as HTMLInputElement).checked })}
            />
            {flag.label}
          </label>
        {/each}
      </fieldset>
      <label class="column">
        <span>Third-party licenses</span>
        <textarea
          rows="2"
          value={meta.third_party_licenses ?? ""}
          onchange={(e) => setMeta({ third_party_licenses: optionalText(inputValue(e)) })}
        ></textarea>
      </label>

      <h4>Standard hit areas</h4>
      {#if model.hit_areas.length === 0}
        <p class="muted">The model has no hit areas.</p>
      {:else}
        {#each STANDARD_HIT_AREAS as name (name)}
          <label class="row">
            <span>{name}</span>
            <select
              value={profile.hit_areas.find((a) => a.name === name)?.hit_area ?? ""}
              onchange={(e) => {
                const v = inputValue(e);
                return apply(hitAreaCommand(profile.hit_areas, name, v === "" ? null : v));
              }}
            >
              <option value="">None</option>
              {#each model.hit_areas as hit (hit.id)}
                <option value={hit.id}>
                  {hit.id} ({elementLabel(model.meshes.find((m) => m.id === hit.mesh)?.name ?? null, hit.mesh)})
                </option>
              {/each}
            </select>
          </label>
        {/each}
      {/if}

      <button class="remove" onclick={() => apply({ op: "set_avatar_profile", profile: null })}>
        Remove avatar profile
      </button>
    {/if}

    <h4>Conformance</h4>
    {#if checkError}
      <p class="error">{checkError}</p>
    {:else if check && summary}
      <p class={summary.ok ? "ok" : "warn"}>{summary.text}</p>
      <!-- プロファイルがないときのエラーは「model has no avatar profile」だけで、見出しと同じなので出さない。 -->
      {#if check.claimed !== null && check.errors.length > 0}
        <ul class="issues errors">
          {#each check.errors as issue, i (i)}
            <li>{issue.message}</li>
          {/each}
        </ul>
      {/if}
      {#if check.warnings.length > 0}
        <ul class="issues warnings">
          {#each check.warnings as issue, i (i)}
            <li>{issue.message}</li>
          {/each}
        </ul>
      {/if}
    {/if}
  </section>
{/if}

<style>
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin: 4px 0;
}

.row input,
.row select {
  width: 12em;
  min-width: 0;
}

.column textarea {
  font: inherit;
}

.column {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin: 4px 0;
}

.check {
  display: flex;
  align-items: center;
  gap: 6px;
}

fieldset {
  margin: 4px 0;
  padding: 4px 8px;
  border: 1px solid color-mix(in srgb, currentColor 25%, transparent);
  border-radius: 4px;
}

.remove {
  margin-top: 8px;
}

.ok {
  padding: 4px 6px;
  border: 1px solid #16a34a;
  border-radius: 4px;
  background: color-mix(in srgb, #16a34a 12%, transparent);
}

.warn {
  padding: 4px 6px;
  border: 1px solid #d97706;
  border-radius: 4px;
  background: color-mix(in srgb, #d97706 12%, transparent);
}

.issues {
  margin: 4px 0;
  padding-left: 1.2em;
  overflow-wrap: anywhere;
}

.errors li {
  color: #dc2626;
}

h3 {
  margin: 8px 0 4px;
}

h4 {
  margin: 10px 0 2px;
}
</style>
