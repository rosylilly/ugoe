// テストが読むリポジトリの中の場所。

import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export const REPO_ROOT = join(dirname(fileURLToPath(import.meta.url)), "../../..");
export const SPEC_EXAMPLES = join(REPO_ROOT, "spec/examples");
export const COMPAT = join(REPO_ROOT, "assets/compat");
