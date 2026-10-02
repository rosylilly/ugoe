// アプリに同梱した見本 (assets/samples、MIT)。ツールバーの「サンプル」から開く。
// ファイルは vite.config.ts の `sampleModels` が assets/samples の .ugoe と .ugoemod をすべて開発サーバーで配り、
// ビルドの出力の `samples/` に入れる。
import type { ModelFile, ModuleToOpen } from "@ugoe/player";
import { t } from "./i18n";

/** 見本と一緒に読み込む着せ替えのモジュール。 */
export interface SampleModule {
  /** `samples/` からのパス。 */
  file: string;
  /** 読み込んだときに付けるか (付けないものはパネルから付け替えて試す)。 */
  attach: boolean;
}

export interface Sample {
  id: string;
  /** メニューに出す名前 (今の言語)。 */
  readonly label: string;
  /** モデル (`samples/` からのパス)。 */
  model: string;
  modules: readonly SampleModule[];
}

export const SAMPLES: readonly Sample[] = [
  {
    id: "avatar",
    get label() {
      return t("samples.avatar.label");
    },
    model: "avatar.ugoe",
    modules: [],
  },
  {
    // `ugoe sample avatar --lit` で作る (assets/samples/README.md#avatar-lit)。
    id: "avatar-lit",
    get label() {
      return t("samples.avatarLit.label");
    },
    model: "avatar-lit.ugoe",
    modules: [],
  },
  {
    // `ugoe sample dress-up` で作る (assets/samples/README.md#dress-up)。
    id: "dress-up",
    get label() {
      return t("samples.dressUp.label");
    },
    model: "dress-up/dress-up.ugoe",
    modules: [
      { file: "dress-up/long-hair.ugoemod", attach: true },
      { file: "dress-up/twin-tails.ugoemod", attach: false },
      { file: "dress-up/cat-ears.ugoemod", attach: true },
      { file: "dress-up/sailor.ugoemod", attach: true },
    ],
  },
];

/** 同梱するファイル (`samples/` からのパス。重なりなし)。 */
export function sampleFiles(samples: readonly Sample[] = SAMPLES): string[] {
  const files = samples.flatMap((s) => [s.model, ...s.modules.map((m) => m.file)]);
  return [...new Set(files)];
}

/** 見本を開く先 (`Player`)。 */
export interface SampleTarget {
  /** モデルを開き、`modules` を読み込む (前に付けていた組があれば、その後で戻すか尋ねる。T0711)。 */
  load(file: ModelFile, modules?: readonly ModuleToOpen[]): Promise<void>;
}

/**
 * 見本を開く: ファイルをすべて読んでから、モデルをモジュールと一緒に開く (`attach` のものは付ける)。
 * 読めないファイルがあれば、何も開かずに例外を投げる。
 */
export async function openSample(
  sample: Sample,
  target: SampleTarget,
  read: (path: string) => Promise<ModelFile>,
): Promise<void> {
  const [model, ...modules] = await Promise.all([
    read(sample.model),
    ...sample.modules.map((m) => read(m.file)),
  ]);
  if (model === undefined) return;
  await target.load(
    model,
    sample.modules.flatMap((m, i) => {
      const file = modules[i];
      return file === undefined ? [] : [{ file, attach: m.attach }];
    }),
  );
}
