---
id: T0716
title: 見本: 法線マップ付きの見本を共通の関数 (group_bulge_normal_maps) で作り直す
status: done
milestone: M4
crates: [ugoe-cli, assets, docs]
depends_on: [T0518]
assignee: claude-worker-T0716
---

# T0716: 見本: avatar-lit を共通の関数で作り直す

## 目的

T0518 の後続の候補。T0518 で、部位のまとまりでふくらませる方法を `ugoe_rigging::group_bulge_normal_maps` に一般化した。
見本の生成 (`crates/ugoe-cli/src/sample/lit.rs`) は、独自の同じ処理 (と半画素のずれ) を持ったままになっている。共通の関数に寄せると輪郭の画素が変わる
(T0518 の見積もり: 輪郭で最大 110°、形の中の平均 0.02〜11°) ので、見本を作り直す。

## 受け入れ条件

- [x] lit.rs を `group_bulge_normal_maps` と `NormalMap::downscale` に置き換え、独自の `extend_edges` と `downscale` を消す
- [x] `assets/samples/avatar-lit.ugoe` と `avatar-lit.jpg` を作り直し、前後の絵を並べて目で確かめる。輪郭の違いの大きさをメモに書く
- [x] avatar.ugoe と着せ替えの見本は変えない。ファイルの大きさが大きく増えない。最新であることのテストは今の許容で通す
- [x] `just ci`・`just test-gpu` が通る

## 参考

- `docs/tasks/T0518-ai-normal-map-groups.md`・`docs/tasks/T0707-lit-sample-avatar.md` のメモ

## メモ

### やったこと

- `crates/ugoe-cli/src/sample/lit.rs`: まとまりのふくらみを `ugoe_rigging::group_bulge_normal_maps` に、縮小を `NormalMap::downscale` に置き換え、
  独自の格子・`extend_edges`・`downscale` を消した。PSD の取り込みはレイヤーを切り抜いたまま置くので、画素の位置は `origin + (x, y)`・
  `canvas_per_texel` は 1 (格子の升目 = キャンバスの 1 画素なので、`radius`・`smoothing` の値は変えていない)。まとまりと設定 (`GROUPS`) も変えていない。
- `assets/samples/avatar-lit.ugoe` を作り直した (1,565,336 → 1,543,722 バイト。法線マップは合わせて約 198KB → 約 177KB)。
  `avatar-lit.jpg` も同じ並び (全体を 1024 で描き、640×640+192+40 を切り抜いて 360 に縮小。ライティングなし・`directional:1,0,-1`・
  `-1,0,-1`・`0,1,-1`、強さ 0.6、環境光 0.55。前の jpg をこの手順で再現して RMSE 2% で一致を確かめた) で作り直した (123KB → 113KB)。
  `assets/samples/README.md` の大きさと、共通の関数で作ることを直した。
- avatar.ugoe と着せ替えの見本は変えていない。最新であることのテストは今の許容 (画素ごとの差 2) のまま通る。

### 輪郭の違いの大きさ

前の見本と新しい見本の法線マップ (半分の解像度) の角度の差。「中」は 2×2 の元の画素がすべて形の中の画素、「縁」は一部だけが形の中の画素:

- 輪郭 (縁) の最大: 後ろ髪 118°・前髪 106°・体 82°・腕 72〜74°・顔 64°・耳 47〜52°・耳飾り 43〜44°。縁の平均は 5〜38°。
- 中の平均: 顔の上の部位 (目・口・鼻・眉) 0.02〜1.1°、顔 0.8°・体 0.3°・首 0.5°・腕 0.4〜0.7°・髪 1.3〜1.8°・耳 3〜3.5°・耳飾り 10〜12°
  (耳飾りは細いので、ほとんどの画素が輪郭の近く)。T0518 の見積もりと合う。
- 描画 (1024 の全体) の差は RMSE 0.4〜0.6%。並べて見ても違いは分からない (肩の上の腕の輪郭の線も出ていない)。

画像 (scratchpad): `<scratchpad>/T0716/before-after.png` (上が前・下が新しい jpg)・`face-before-after.png`・`body-before-after.png`
(顔・体の拡大。左・右・上の光、上が前)。前の見本は `<scratchpad>/T0716/avatar-lit.ugoe`・`avatar-lit.jpg`、作る手順は `panels.sh`。

### 確かめたこと

- `just ci`・`just test-gpu` (`CARGO_BUILD_JOBS=8`)。

### 確かめていないこと・後続の候補

- ビューア・エディタの画面では見ていない (描画は `ugoe render` の llvmpipe だけ)。ファイルの形は変わらないので読み込みは同じ。
- `ugoe_ai::normal` の既定のまとまり (T0518) と見本の `GROUPS` はまだ別に書いてある (値は同じ)。見本を `ugoe_ai` の提案から作ることもできるが、
  ugoe-cli が ugoe-ai の既定に左右されると見本が勝手に変わるので寄せていない。
