// AI のプロバイダの選択 (docs/editor.md#ai-の設定と送信の同意、ADR 0022)。設定の画面の選択肢と、
// AI の機能を使えるかの判断。providers.test.ts で検査する。
import { t } from "../i18n";
import type { AiSettings, OutputMode } from "../ipc";

/** OpenAI 互換の API のよく使う送り先 (選ぶとベースの URL とモデルを埋める)。 */
export interface OpenAiPreset {
  /** 選択肢の名前 (製品名。手元のサーバは「(this computer)」を訳して付ける)。読むたびに今の言語で作る。 */
  readonly label: string;
  base_url: string;
  /** 既定のモデル。空なら利用者が入れる (`placeholder` を示す)。 */
  model: string;
  /** モデルの欄の例。読むたびに今の言語で作る。 */
  readonly placeholder: string;
}

// 製品名とモデルの ID は訳さない。説明の付く名前と例は getter で読むたびに t() を通す。
export const OPENAI_PRESETS: readonly OpenAiPreset[] = [
  {
    // i18n-ignore: 製品名
    label: "OpenAI",
    base_url: "https://api.openai.com/v1",
    model: "gpt-6-sol",
    placeholder: "gpt-6-sol",
  },
  {
    // i18n-ignore: 製品名
    label: "OpenRouter",
    base_url: "https://openrouter.ai/api/v1",
    model: "",
    get placeholder() {
      return t("settings.ai.presets.openRouterModel");
    },
  },
  {
    // i18n-ignore: 製品名
    label: "Google Gemini",
    base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
    model: "gemini-3.8-flash",
    placeholder: "gemini-3.8-flash",
  },
  {
    get label() {
      return t("settings.ai.presets.local", { name: "Ollama" });
    },
    base_url: "http://localhost:11434/v1",
    model: "",
    get placeholder() {
      return t("settings.ai.presets.ollamaModel");
    },
  },
  {
    get label() {
      // i18n-ignore: 製品名
      return t("settings.ai.presets.local", { name: "LM Studio" });
    },
    base_url: "http://localhost:1234/v1",
    model: "",
    get placeholder() {
      return t("settings.ai.presets.lmStudioModel");
    },
  },
];

/** ベースの URL に合う選択肢 (末尾の `/` は無視)。なければ `undefined` (手で入れた送り先)。 */
export function presetFor(baseUrl: string): OpenAiPreset | undefined {
  const trim = (u: string) => u.trim().replace(/\/+$/, "");
  return OPENAI_PRESETS.find((p) => trim(p.base_url) === trim(baseUrl));
}

/** 答えを JSON にさせる方法の説明 (`label` は読むたびに今の言語で作る)。 */
export const OUTPUT_MODES: readonly { readonly value: OutputMode; readonly label: string }[] = [
  {
    value: "json_schema",
    get label() {
      return t("settings.ai.output.jsonSchema");
    },
  },
  {
    value: "json_object",
    get label() {
      return t("settings.ai.output.jsonObject");
    },
  },
  {
    value: "prompt",
    get label() {
      return t("settings.ai.output.prompt");
    },
  },
];

/** 手元のサーバの選択肢の短い名前 (「(this computer)」を付けない)。 */
const LOCAL_NAMES: Record<string, string> = {
  "http://localhost:11434/v1": "Ollama",
  // i18n-ignore: 製品名
  "http://localhost:1234/v1": "LM Studio",
};

/** 画面に出すプロバイダの短い名前 (「Use …」など)。 */
export function providerLabel(s: AiSettings): string {
  if (s.provider.provider === "claude") return "Claude";
  const preset = presetFor(s.provider.openai.base_url);
  if (!preset) return s.service;
  return LOCAL_NAMES[preset.base_url] ?? preset.label;
}

/** AI に送れない理由。送れるなら `null`。 */
export function aiBlockedReason(s: AiSettings): string | null {
  if (!s.local && !s.send_externally) {
    return t("settings.ai.blocked.notSending");
  }
  if (s.key_required && s.key !== "stored" && !s.env_key) {
    return s.key_env
      ? t("settings.ai.blocked.noKeyEnv", { service: s.service, env: s.key_env })
      : t("settings.ai.blocked.noKey", { service: s.service });
  }
  return null;
}
