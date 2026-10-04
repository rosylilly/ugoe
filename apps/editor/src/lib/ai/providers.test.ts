import { describe, expect, it } from "vitest";
import type { AiSettings } from "../ipc";
import { termsFor } from "./consent";
import { aiBlockedReason, presetFor, providerLabel } from "./providers";

function settings(patch: Partial<AiSettings> = {}): AiSettings {
  return {
    provider: {
      provider: "claude",
      claude: { model: "claude-opus-5-5" },
      openai: { base_url: "https://api.openai.com/v1", model: "gpt-6-sol", output: "json_schema" },
      time_limit_minutes: 15,
    },
    service: "Anthropic Claude API",
    local: false,
    key: "stored",
    key_required: true,
    key_env: "ANTHROPIC_API_KEY",
    env_key: false,
    send_externally: true,
    consent_remembered: false,
    ...patch,
  };
}

function openai(base_url: string, patch: Partial<AiSettings> = {}): AiSettings {
  const s = settings({ key: "not_stored", key_required: false, key_env: null, ...patch });
  s.provider = {
    ...s.provider,
    provider: "openai_compatible",
    openai: { ...s.provider.openai, base_url },
  };
  return s;
}

describe("aiBlockedReason", () => {
  it("allows a provider with a key", () => {
    expect(aiBlockedReason(settings())).toBeNull();
    expect(aiBlockedReason(settings({ key: "unavailable", env_key: true }))).toBeNull();
  });

  it("explains a missing key with the environment variable", () => {
    expect(aiBlockedReason(settings({ key: "not_stored" }))).toBe(
      "No API key for Anthropic Claude API. Add one in Settings, or set ANTHROPIC_API_KEY before starting the editor.",
    );
  });

  it("does not need a key for OpenAI-compatible servers", () => {
    expect(
      aiBlockedReason(openai("https://openrouter.ai/api/v1", { service: "OpenRouter" })),
    ).toBeNull();
  });

  it("blocks external services in projects that do not send, but not local servers", () => {
    expect(aiBlockedReason(settings({ send_externally: false }))).toMatch(/not to send/);
    expect(
      aiBlockedReason(openai("http://localhost:11434/v1", { send_externally: false, local: true })),
    ).toBeNull();
  });
});

describe("providers", () => {
  it("names the provider after the preset or the service", () => {
    expect(providerLabel(settings())).toBe("Claude");
    expect(providerLabel(openai("http://localhost:11434/v1/"))).toBe("Ollama");
    expect(
      providerLabel(
        openai("https://llm.example.com/v1", {
          service: "OpenAI-compatible API (llm.example.com)",
        }),
      ),
    ).toBe("OpenAI-compatible API (llm.example.com)");
    expect(presetFor("https://openrouter.ai/api/v1/")?.label).toBe("OpenRouter");
    expect(presetFor("https://example.com/v1")).toBeUndefined();
  });

  it("links the terms of known destinations only", () => {
    expect(termsFor("https://api.anthropic.com/v1/messages")?.org).toBe("Anthropic");
    expect(termsFor("https://api.openai.com/v1/chat/completions")?.org).toBe("OpenAI");
    expect(termsFor("https://openrouter.ai/api/v1/chat/completions")?.links).toHaveLength(2);
    expect(termsFor("https://llm.example.com/v1/chat/completions")).toBeNull();
    expect(termsFor("not a url")).toBeNull();
  });
});
