// デスクトップアプリの署名と公証を、環境変数から決める (scripts/app-build.mjs と apps/studio/scripts/build-vcam.mjs が使う)。
// 手順と環境変数の一覧は docs/release.md#デスクトップアプリの署名。
//
// 環境変数が無ければ署名しない (今までと同じ)。フォークや手元のビルドは何も設定しなくても作れる。

import { spawnSync } from "node:child_process";

/** Windows の署名のコマンド。Tauri の `bundle.windows.signCommand` と同じ形 (`%1` が署名するファイル)。 */
export const WINDOWS_SIGN_ENV = "UGOE_WINDOWS_SIGN_COMMAND";

/**
 * 署名しなかった理由を呼ぶ側が渡す環境変数 (GitHub Actions のタグでないジョブ。署名の Secrets が environment `release` にだけ
 * あるため)。署名の環境変数が無くて署名しないとき、ジョブの要約にこの理由を出す。
 */
export const UNSIGNED_REASON_ENV = "UGOE_UNSIGNED_REASON";

/** macOS の公証の資格情報の組 (Tauri が読む環境変数。どちらか 1 組が揃っていれば公証する)。 */
export const NOTARIZE_ENV_SETS = [
  ["APPLE_API_ISSUER", "APPLE_API_KEY", "APPLE_API_KEY_PATH"],
  ["APPLE_ID", "APPLE_PASSWORD", "APPLE_TEAM_ID"],
];

/**
 * 署名と公証に関わる環境変数 (Tauri と artifact-signing-cli が読むもの)。GitHub Actions は置いていない Secrets を空の文字列で
 * 渡すが、Tauri は空の値も「設定されている」と読むので、ビルドに渡す前に空のものを消す (`withoutEmptySigningEnv`)。
 */
export const SIGNING_ENV = [
  WINDOWS_SIGN_ENV,
  "APPLE_CERTIFICATE",
  "APPLE_CERTIFICATE_PASSWORD",
  "APPLE_SIGNING_IDENTITY",
  ...NOTARIZE_ENV_SETS.flat(),
  "AZURE_CLIENT_ID",
  "AZURE_CLIENT_SECRET",
  "AZURE_TENANT_ID",
  "AZURE_ARTIFACT_SIGNING_ACCOUNT",
  "AZURE_ARTIFACT_SIGNING_CERTIFICATE_PROFILE",
];

/** 署名の環境変数のうち、空の値のものを除いた写し。 */
export function withoutEmptySigningEnv(env) {
  const out = { ...env };
  for (const name of SIGNING_ENV) {
    if (name in out && (out[name] ?? "").trim() === "") delete out[name];
  }
  return out;
}

const set = (env, name) => (env[name] ?? "").trim() !== "";

/** 署名しない理由。呼ぶ側の理由 (`UNSIGNED_REASON_ENV`) があればそれを、無ければ `fallback` (足りない環境変数) を返す。 */
const unsignedReason = (env, fallback) =>
  set(env, UNSIGNED_REASON_ENV) ? `${env[UNSIGNED_REASON_ENV].trim()}; ${fallback}` : fallback;

/**
 * 1 つの OS 向けのビルドで、署名と公証をするか (純粋な関数。テストする)。
 *
 * - `sign`・`notarize`: するか。`reason`: しないときの理由 (ジョブの要約に出す)。
 * - `config`: `tauri build --config` で重ねる設定 (Windows の signCommand)。無ければ null。
 * - `error`: 設定が半端で、黙って署名せずに作ると取り違えるとき (何も作らずに止まる)。
 */
export function signingPlan(platform, env = process.env) {
  switch (platform) {
    case "windows": {
      if (!set(env, WINDOWS_SIGN_ENV)) {
        return {
          sign: false,
          notarize: false,
          config: null,
          reason: unsignedReason(env, `${WINDOWS_SIGN_ENV} is not set`),
        };
      }
      const command = env[WINDOWS_SIGN_ENV].trim();
      if (!command.includes("%1")) {
        return { error: `${WINDOWS_SIGN_ENV} must contain %1 (the path of the file to sign)` };
      }
      return {
        sign: true,
        notarize: false,
        config: { bundle: { windows: { signCommand: command } } },
        reason: null,
      };
    }
    case "macos": {
      if (!set(env, "APPLE_SIGNING_IDENTITY")) {
        const stray =
          NOTARIZE_ENV_SETS.flat().find((n) => set(env, n)) ??
          (set(env, "APPLE_CERTIFICATE") ? "APPLE_CERTIFICATE" : null);
        if (stray) {
          return { error: `${stray} is set but APPLE_SIGNING_IDENTITY is not (set both to sign)` };
        }
        return {
          sign: false,
          notarize: false,
          config: null,
          reason: unsignedReason(env, "APPLE_SIGNING_IDENTITY is not set"),
        };
      }
      if (set(env, "APPLE_CERTIFICATE") !== set(env, "APPLE_CERTIFICATE_PASSWORD")) {
        return { error: "set both APPLE_CERTIFICATE and APPLE_CERTIFICATE_PASSWORD (or neither)" };
      }
      // 組の一部だけがあるのは取り違え (公証されないまま配ってしまう) なので止める
      for (const names of NOTARIZE_ENV_SETS) {
        const given = names.filter((n) => set(env, n));
        if (given.length > 0 && given.length < names.length) {
          const lacking = names.filter((n) => !set(env, n));
          return {
            error: `notarization needs ${names.join(", ")} (missing ${lacking.join(", ")})`,
          };
        }
      }
      const notarize = NOTARIZE_ENV_SETS.some((names) => names.every((n) => set(env, n)));
      return {
        sign: true,
        notarize,
        config: null,
        reason: notarize ? null : "no notarization credentials (APPLE_API_* or APPLE_ID)",
      };
    }
    default:
      return {
        sign: false,
        notarize: false,
        config: null,
        reason: "Linux packages are not signed",
      };
  }
}

/** Tauri と同じく、signCommand を空白で分け、`%1` をファイルのパスに替える。 */
export function windowsSignArgv(command, file) {
  return command
    .trim()
    .split(/\s+/)
    .map((part) => part.replaceAll("%1", file));
}

/**
 * Windows の署名のコマンドが設定されていれば、ファイルに署名する (Tauri が署名しないもの。スタジオの仮想カメラの DLL など)。
 * 設定が無ければ何もしない。署名したら true。失敗したら例外。
 */
export function signWindowsFile(file, env = process.env) {
  if (!set(env, WINDOWS_SIGN_ENV)) return false;
  const [cmd, ...args] = windowsSignArgv(env[WINDOWS_SIGN_ENV], file);
  const r = spawnSync(cmd, args, { stdio: "inherit", env, shell: process.platform === "win32" });
  if (r.error) throw new Error(`failed to run ${cmd}: ${r.error.message}`);
  if (r.status !== 0) throw new Error(`signing ${file} failed (exit ${r.status})`);
  return true;
}

/** 公証に使う notarytool の資格情報の引数 (`signingPlan` が notarize: true のときだけ呼ぶ)。 */
export function notarytoolAuthArgs(env = process.env) {
  if (NOTARIZE_ENV_SETS[0].every((n) => set(env, n))) {
    return [
      "--issuer",
      env.APPLE_API_ISSUER,
      "--key-id",
      env.APPLE_API_KEY,
      "--key",
      env.APPLE_API_KEY_PATH,
    ];
  }
  return [
    "--apple-id",
    env.APPLE_ID,
    "--password",
    env.APPLE_PASSWORD,
    "--team-id",
    env.APPLE_TEAM_ID,
  ];
}

/** ジョブの要約 (GitHub Actions の `$GITHUB_STEP_SUMMARY`) に書く Markdown の表。 */
export function signingSummary(rows) {
  const yes = (v) => (v ? "yes" : "no");
  const lines = [
    "### Code signing",
    "",
    "| app | OS | signed | notarized | note |",
    "| --- | --- | --- | --- | --- |",
  ];
  for (const { app, platform, plan } of rows) {
    const notarized = platform === "macos" ? yes(plan.notarize) : "-";
    lines.push(
      `| ${app} | ${platform} | ${yes(plan.sign)} | ${notarized} | ${plan.reason ?? ""} |`,
    );
  }
  return `${lines.join("\n")}\n`;
}
