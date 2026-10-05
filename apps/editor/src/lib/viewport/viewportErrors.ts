// ビューポートの WebPuppet の作成と読み込みの失敗の文 (T0941。docs/ui.md#webpuppet-の寿命)。@ugoe/player のカタログの文は
// ビューア・スタジオ向け (「アプリを起動し直して」) なので、GPU やブラウザの都合の失敗はエディタのカタログの文にする。
import { describeLoadError, errorCode } from "@ugoe/player";
import { errorText } from "../errors";
import { t } from "../i18n";

/** 作成の失敗 (`CreateError`。T0475) の `code` ごとの文。 */
const CREATE_ERROR_TEXTS: Record<string, () => string> = {
  backend_unavailable: () => t("viewport.createError.backendUnavailable"),
  context_unavailable: () => t("viewport.createError.contextUnavailable"),
  adapter_unavailable: () => t("viewport.createError.adapterUnavailable"),
  device_unavailable: () => t("viewport.createError.deviceUnavailable"),
};

/**
 * WebPuppet を作れなかった理由の文。`CreateError` (crates/ugoe-wasm/README.md#作成の失敗-t0475) はエディタのカタログの文、
 * 知らないコードとほかの例外は例外の文 (`errorText`)。
 */
export function createErrorText(e: unknown): string {
  const code = errorCode(e, ["CreateError"]);
  const text = code === null ? undefined : CREATE_ERROR_TEXTS[code];
  return text?.() ?? errorText(e);
}

/**
 * モデルを読み込めなかった理由の文。GPU が止まった (`device_lost`) ものはエディタのカタログの文 (エディタを起動し直す案内。
 * スクリーンショットの失敗と同じ文)、ほかは @ugoe/player の `describeLoadError` (ビューアと同じ訳。T1611)。
 */
export function loadErrorText(e: unknown): string {
  if (errorCode(e, ["LoadError"]) === "device_lost") return t("viewport.deviceLost");
  return describeLoadError(e);
}
