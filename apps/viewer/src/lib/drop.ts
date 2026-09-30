// ウィンドウにドロップしたファイルを開く (App.svelte)。
//
// Tauri のウィンドウでは OS からのドロップを Tauri が受けてパスを渡す (`ViewerHost.listenFileDrop`) ので、
// パスから読み、`path:` の場所を付ける (起動し直した後も覚えた組や光源を戻せる。T0713)。
// ブラウザでは HTML の File API で読むので、場所は分からない。
import { detectModelKind, type ModelFile } from "./modelFile";
import type { SampleTarget } from "./samples";

/**
 * ドロップしたファイルを開く。モデルとモジュール (.ugoemod) を一緒に落としたら、モデルをモジュールと一緒に開く
 * (別のモデルなら前のモジュールは一覧から除かれ、落としたモジュールが付く)。モデルは最後の 1 つだけを開く。
 * モジュールだけなら、1 つずつ読み込む。
 */
export async function openDroppedFiles(
  files: readonly ModelFile[],
  target: SampleTarget,
): Promise<void> {
  const isModule = (f: ModelFile) => detectModelKind(f) === "module";
  const model = files.filter((f) => !isModule(f)).at(-1);
  const modules = files.filter(isModule);
  if (model !== undefined)
    await target.load(
      model,
      modules.map((file) => ({ file, attach: true })),
    );
  else for (const file of modules) await target.load(file);
}
