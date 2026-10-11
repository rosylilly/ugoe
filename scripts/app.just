# デスクトップアプリのビルド (justfile の `mod app`)。`just app::build` (または `just app build`) で呼ぶ。
# just のレシピの名前には `:` を使えないので、`app:build` ではなくモジュールの `app::build` にしている。

set shell := ["bash", "-euo", "pipefail", "-c"]
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

# 引数は `KEY=値` と tauri build の引数 (scripts/app-build.mjs、apps/editor/README.md#各-os-でのビルド)。
# TARGET はアプリ (既定はすべて)、PLATFORM は OS (既定は今の OS。Linux からは windows も作れる)。できたものは target/dist/ に集める。
# アプリのインストーラを作る: `just app::build TARGET=editor,viewer PLATFORM=linux,windows`
build *args:
    @node app-build.mjs {{args}}

# アプリのアイコン (apps/<アプリ>/src-tauri/icons/) を assets/brand/<アプリ>.svg から作り直す (Tauri の CLI の `tauri icon`)。
# Android・iOS 用に出力されるディレクトリは使わないので消す (docs/release.md#アプリのアイコン)
icons:
    pnpm --dir ../apps/editor tauri icon ../../assets/brand/editor.svg -o src-tauri/icons
    pnpm --dir ../apps/viewer tauri icon ../../assets/brand/viewer.svg -o src-tauri/icons
    pnpm --dir ../apps/studio tauri icon ../../assets/brand/studio.svg -o src-tauri/icons
    node -e "for (const a of ['editor', 'viewer', 'studio']) for (const d of ['android', 'ios']) require('fs').rmSync('../apps/' + a + '/src-tauri/icons/' + d, { recursive: true, force: true })"
