---
id: T0917
title: UI 品質: 足りない部品と口を足す (TextField・tooltip・Menu の説明の行)
status: done
milestone: M10
crates: [packages, docs, apps/editor, apps/viewer, apps/studio]
depends_on: [T0915, T0912]
assignee: claude-worker-T0917
---

# T0917: UI 品質: 足りない部品と口を足す (TextField・tooltip・Menu の説明の行)

## 目的

T0907 の後続の候補として残った部品の不足を埋め、アプリに残る自前の部品と `title` を減らす。
基準は docs/ui.md (T0915 で足す「UI の品質の基準」を含む)。直せないもの・大きいものは後続のタスクにする (milestone は M10)。

## 受け入れ条件

- [x] `@ugoe/ui` に TextField (ラベル・補足・エラー・無効) を足し、Vitest でキーボード操作と aria の属性を検査した
- [x] `Slider`・`Select`・`NumberField` に tooltip の口を足した (トラッキングの平滑化の説明などを戻せるか判断する)
- [x] `Menu` の項目に説明の行を出せるようにした
- [x] 3 つのアプリで、標準の `<input type=text>` と自前の部品を TextField などに置き換えた (置き換えないものは理由を書く)
- [x] 残った `title` (行の中の説明: 物理のリグの欄・カーブのフェードの欄など) を、見える補足か Tooltip にした
- [x] docs/ui.md の部品の一覧を更新した
- [x] `just ci` が通る

## 参考

- docs/ui.md
- T0907 のメモの「後続の候補」
- docs/roadmap.md の M8 の「起票していない後続の候補」の UI の行

## メモ

- **やったこと:**
  - `@ugoe/ui`: `TextField` (ラベル・補足 `hint`・誤りの文 `error`・`invalid`・無効・`readonly`・`mono`・`size`・`tooltip`・`id` で外の `<label for>`)。
    `oninput` は打つたび、`onchange` は確定 (Enter・フォーカスを外す) で利用者が書き換えたときだけ (外から渡した値では呼ばない)、Esc で確定前に戻す
    (戻すものが無ければ外側へ渡す)。`Slider` (つまみ)・`NumberField`・`Select` に `tooltip`、`Select` に `invalid` (T0916 の候補)、`Slider` に
    `aria-describedby`、`Menu` の項目に説明の行 (`description`。`aria-labelledby` と `aria-describedby` で名前と説明を分ける。狭い窓では折り返す)。
    Vitest (キーボード・aria・外からの値・Tooltip の説明との並び・Select の誤り・Menu の説明) と、見本の画面 (文字の欄・誤りの Select・説明の行) を足した。
  - アプリ: 標準の文字の欄 (モジュールの名前 2・アバターのメタデータ・パラメータとグループの ID と名前・デフォーマの名前・物理のリグと衝突体の名前・
    スロットの名前・モーションと表情の名前・イベントの値・AI の設定のモデルと URL・MCP のポート・プレビューとビューアの絞り込み・スタジオのシーンの名前) を
    `TextField` に、AI の時間の上限を `NumberField` に。エディタの `app.css` の `.ugoe-dialog input[type="text"]` (仮の装い) を消した。
    ビューアの「サンプル」に見本の中身、スタジオの最近のプロファイルに場所 (名前はファイル名) を説明の行で出す。
  - `title`: 物理のリグの「基準」・テンプレートの「合成」は `Select` の `tooltip`、テンプレートの押せない理由は見える補足、カーブのフェードの欄・
    タイムラインの ◆ (読み上げの名前も付けた)・キーの印・静止姿勢の警告・検証のセルは Tooltip、AI の注文と依存・ビューアのイベントの読み方は見える補足に。
    キー名は種類に合わせた (`createMissing.description`・`summary.instructions`・`motions.eventsDescription`)。
  - トラッキングの平滑化の説明: 見える補足から `Slider` の `tooltip` に戻した (同じパネルの Toggle の説明と揃える・パネルが長い (T0915))。
  - docs/ui.md: 部品の一覧 (TextField・tooltip・invalid・説明の行)・`title` の約束・エディタの文字の欄の行・状態の表のエラー・
    「足りない部品と口 (T0917)」(残した標準の `<input>` と `title` の理由の表)。packages/ui/README.md。
- **`just ui-audit` (`--compare target/ui-audit-before`、基準線は main a099c46e):** 3 つのアプリとも +0 -0 (新しい崩れ無し)。基準線は
  エディタ 重なり 4 (23)・24px 未満 8 (540)・ARIA 1 (10)、ビューア Tab 1・ARIA 1 (28)、スタジオ 0。途中で、説明の行のあるメニューが
  360px の窓で横にはみ出した (+7) ので、メニューの幅を窓に収めて折り返すようにして 0 に戻した。
- **やらなかったこと (理由は docs/ui.md の表):** 空にできる数値の欄 (カーブのフェード・グルー・推定。`type="number"` のまま、フェードは Tooltip を付けた)、
  パラメータの値の欄、ツリーの名前の変更、API キー (`password`)、複数行の `<textarea>`。フォーカスを受けない要素の補いの `title`
  (ツリーの行・中継の印・タイムラインの行の名前と目盛り・ビューアの行の印と警告の全体)。
- **確かめていないこと:** 実物の WebView での Tooltip と説明の行の見え方 (ヘッドレスの Chrome の見本と ui-audit の画像だけ)、e2e (`just e2e`) は流していない
  (AI の設定の Base URL の欄は `<label for>` のまま見つかる作りにした)。
- **範囲外の変更:** `packages/face-tracking` (平滑化の説明。タスクの範囲の `packages` に含まれると読んだ)。
- **起票の候補:**
  - 空 (既定を使う) を持てる数値の部品 (`NumberField` の `optional` など)。カーブのフェード・グルーのキー・推定の軸と許す差を移す。
  - 複数行の文字の部品 (`TextArea`。アバターのメタデータ・AI の注文)。
  - 値を状態に持たない秘密の欄 (API キー) を `TextField` の見た目で出す口 (`type="password"` と値を持たない作り) にするか。
  - ツリーの名前の変更を `TextField` に (行の中で開く欄の口: 開いたときに全体を選ぶ・押下を行に渡さない)。

