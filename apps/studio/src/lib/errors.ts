// バックエンドのエラー (lib/profile.ts の `StudioError`) を、今の言語の文にする (ADR 0029 の 4.)。
import { t } from "./i18n";
import { isStudioError, type StudioError } from "./profile";

const MISSING = {
  model: "errors.missingModel",
  module: "errors.missingModule",
  background: "errors.missingBackground",
} as const;

/** 文にする。バックエンドのエラーでなければ、例外の文をそのまま出す。 */
export function describeError(e: unknown): string {
  if (!isStudioError(e)) return e instanceof Error ? e.message : String(e);
  return describeStudioError(e);
}

export function describeStudioError(e: StudioError): string {
  switch (e.code) {
    case "io":
      return t("errors.io", { path: e.path, message: e.message });
    case "tooLarge":
      return t("errors.tooLarge", { path: e.path });
    case "notJson":
      return t("errors.notJson", { path: e.path });
    case "notProfile":
      return t("errors.notProfile", { path: e.path });
    case "futureVersion":
      return t("errors.futureVersion", {
        path: e.path,
        version: e.version,
        supported: e.supported,
      });
    case "invalid":
      return t("errors.invalid", { path: e.path, message: e.message });
    case "unknownScene":
      return t("errors.unknownScene");
    case "missing":
      return t(MISSING[e.asset], { path: e.path });
    case "unreadable":
      return t("errors.unreadable", { path: e.path, message: e.message });
    case "wrongType":
      return t("errors.wrongType", { path: e.path });
    case "dressUp":
      return t("errors.dressUp", { message: e.message });
    case "noModel":
      return t("errors.noModel");
    case "unknownMotion":
      return t("errors.unknownMotion", { name: e.name });
    case "unknownExpression":
      return t("errors.unknownExpression", { name: e.name });
    case "motionRejected":
      return t("errors.motionRejected");
    case "internal":
      return t("errors.internal", { message: e.message });
  }
}
