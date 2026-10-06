---
id: T0980
title: "マニュアル: T1662・T0979 などの後に、エディタのマニュアルの画像の残りを撮り直す"
status: done
milestone: M10
crates: [docs, tests]
depends_on: []
assignee: claude-worker-T0980
---

# T0980: マニュアル: T1662・T0979 などの後に、エディタのマニュアルの画像の残りを撮り直す

## 目的

T1662 (エディタの送る枠の行末の空き 4px)・T0979 (ダイアログの閉じるボタンの位置)・T1670・T1675 (ツリー) などで画面が少し変わったが、
エディタのマニュアルの画像は T0968・T0969・T1671 で撮った後に撮り直していないものが 36 枚ほどある (T1671 の候補)。

## 受け入れ条件

- [x] `just e2e --update-images --only editor` で撮り直し、アニメーション (呼吸・物理) の違いだけの画像は前のままにして、画面が変わった画像だけを入れた。どの画像を入れたかと、変わった理由 (T1662 など) をメモに書いた
- [x] 入れた画像をすべて開き、撮った機械の固有のもの (/home・worktree・mbx のハッシュ・乱数の一時フォルダ) が写っていないことを確かめた
- [x] 画像と説明の文が食い違っていれば、docs/manual の文を直した
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T0968・T0969・T1671 のメモ、tests/e2e/README.md#マニュアルの画像

## メモ

- T1671 のワーカーの報告から起票した (2026-10-07)。

### やったこと (claude-worker-T0980)

- `just e2e --update-images --only editor` (47 件通過) で撮り直すと 44 枚が変わった。前の画像との差 (しきい値 3% の差分の画像) を 1 枚ずつ見て分けた。
- **入れた 40 枚:**
  - ツリー・右の列・タイムラインの行末が 4px 左に寄り、長い名前の省略が 1 文字早くなった (T1662。送る枠の行末の空き):
    depth-rotation・dress-up-preview・expressions・keyform-grid・lighting・lighting-point・mesh-edit・mirror・
    module-compare-highlight・module-export-atlas (ツリーの `HairFront` → `HairFr…`)・module-glue・physics-preview・physics-rig・
    preview・preview-module-pose・preview-tracking・screen・screen-empty・slots・slots-draw-order・timeline・timeline-settings・
    tools-mesh-edit・tools-rotation・tools-warp-gizmo・tutorial-import・verify。
  - 右の列を切り出した画像の幅が 4px 狭くなった (311 → 307、312 → 308。同じく T1662): dress-up-errors・module-compare-details・
    module-compare-differences・module-compare-exported・module-compare-preview・physics-templates。
  - ダイアログの閉じるボタンが重ねて描くスクロールバーを避けて左へ寄った (T0979): ai-settings・settings-general・module-new・
    module-settings・module-keep-look・mcp-connect・mcp-claude-desktop (MCP の 2 枚は待ち受けのポートの番号も変わる。毎回違う)。
- **戻した 4 枚:** module-workspace (違いなし)・module-base-readonly (7 画素のにじみだけ)・module-compare-pose (袖のポーズのアニメーションと
  パラメータの値だけ。T1671 で撮ったばかり)・mcp-approval (経過の秒 `118 s` → `119 s` だけ)。
- 入れた 40 枚をすべて開き、/home・worktree・mbx のハッシュ・乱数の一時フォルダが写っていないことを見た (画面に出るパスは
  `/tmp/ugoe-manual/…` だけ)。PNG の付帯情報にもこれらの文字列は無い (`identify -verbose`)。
- 変わったのは 4px の寄りと省略だけで、docs/manual の文と食い違う所は無かった (文は直していない)。

### 確かめたこと

- `just e2e --update-images --only editor`: 47 件通過。`just e2e --only editor`: 16 件通過。
- `just ci`: 通過 (main 7b5a5ce9 の上。前面で実行。終了コード 0)。

### 起票の候補

- なし。
