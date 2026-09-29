// AI のプロバイダの選択 (docs/editor.md#ai-の設定と送信の同意、ADR 0022)。設定の画面の選択肢と、
// AI の機能を使えるかの判断。providers.test.ts で検査する。
import type { AiSettings, OutputMode } from "../ipc";

/** OpenAI 互換の API のよく使う送り先 (選ぶとベースの URL とモデルを埋める)。 */
export interface OpenAiPreset {
  label: string;
  base_url: string;
  /** 既定のモデル。空なら利用者が入れる (`placeholder` を示す)。 */
  model: string;
  placeholder: string;
}

export const OPENAI_PRESETS: readonly OpenAiPreset[] = [
  {
    label: "OpenAI",
    base_url: "https://api.openai.com/v1",
    model: "gpt-6-sol",
    placeholder: "gpt-6-sol",
  },
  {
    label: "OpenRouter",
    base_url: "https://openrouter.ai/api/v1",
    model: "",
    placeholder: "vendor/model (a vision model)",
  },
  {
    label: "Google Gemini",
    base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
    model: "gemini-3.8-flash",
    placeholder: "gemini-3.8-flash",
  },
  {
    label: "Ollama (this computer)",
    base_url: "http://localhost:11434/v1",
    model: "",
    placeholder: "a vision model you pulled",
  },
  {
    label: "LM Studio (this computer)",
    base_url: "http://localhost:1234/v1",
    model: "",
    placeholder: "the loaded vision model",
  },
];

/** ベースの URL に合う選択肢 (末尾の `/` は無視)。なければ `undefined` (手で入れた送り先)。 */
export function presetFor(baseUrl: string): OpenAiPreset | undefined {
  const trim = (u: string) => u.trim().replace(/\/+$/, "");
  return OPENAI_PRESETS.find((p) => trim(p.base_url) === trim(baseUrl));
}

/** 答えを JSON にさせる方法の説明。 */
export const OUTPUT_MODES: readonly { value: OutputMode; label: string }[] = [
  { value: "json_schema", label: "Structured output (JSON Schema)" },
  { value: "json_object", label: "JSON mode, validated and asked again" },
  { value: "prompt", label: "Schema in the prompt only, validated and asked again" },
];

/** 画面に出すプロバイダの短い名前 (「Use …」など)。 */
export function providerLabel(s: AiSettings): string {
  if (s.provider.provider === "claude") return "Claude";
  return presetFor(s.provider.openai.base_url)?.label.replace(/ \(.*\)$/, "") ?? s.service;
}

/** AI に送れない理由。送れるなら `null`。 */
export function aiBlockedReason(s: AiSettings): string | null {
  if (!s.local && !s.send_externally) {
    return "This project is set not to send data to external AI services.";
  }
  if (s.key_required && s.key !== "stored" && !s.env_key) {
    const env = s.key_env ? `, or set ${s.key_env} before starting the editor` : "";
    return `No API key for ${s.service}. Add one in Settings${env}.`;
  }
  return null;
}
