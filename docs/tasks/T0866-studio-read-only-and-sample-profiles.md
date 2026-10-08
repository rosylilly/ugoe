---
id: T0866
title: "スタジオ: 見本のプロファイルや書けない場所のプロファイルを開いたら、自動の保存で書き換えず「写しを保存」に誘う (判断が要る)"
status: done
milestone: M8
crates: [apps/studio, assets, scripts, docs]
depends_on: []
assignee: claude-worker-T0866
---

# T0866: スタジオ: 見本のプロファイルや書けない場所のプロファイルを自動の保存で書き換えない (判断が要る)

## 目的

スタジオはプロファイルを変えると、保存先があれば自動で保存する (Control.svelte の `scheduleAutosave`)。T0865 で見本のプロファイル
(`assets/samples/ugoe-sample-profile.json`・`.en.json`) をリポジトリにコミットしたので、リポジトリの見本を開いて触ると、コミットしたファイルが
書き換わる (git の差分になる。今はマニュアルに `pnpm --filter @ugoe/studio sample-profile` で戻す方法を書いているだけ)。
読み取り専用の場所 (書けないフォルダ・読み取り専用のファイル) のプロファイルでは、自動の保存が毎回失敗する。
こうしたプロファイルでは自動の保存をせず、利用者を「名前を付けて保存」(写しを保存) に誘う。

## 判断が要る

「書き換えないプロファイル」をどう見分けるか:

- **案 A: ファイルが書けるかで決める。** 開いたとき (と保存に失敗したとき) に書けないと分かったら、そのプロファイルを読み取り専用として扱う。
  見本のプロファイルはリポジトリの中では書けるので、A だけでは見本を守れない。
- **案 B: プロファイルの形式に印を足す** (例: `"template": true`)。印のあるプロファイルは開いても自動の保存をせず、最初の保存を「名前を付けて保存」にする。
  写しには印を書かない。見本の 2 つのファイル (sample-profile.mjs が作るもの) に印を付ける。プロファイルの形式 (docs/studio.md#プロファイルの形式) が変わる。
- 案 C: 今のまま (マニュアルの戻し方だけ)。

推奨: **A と B の両方。** B で見本を守り、A で書けない場所の自動の保存の失敗を止める。どちらも画面では同じ扱い
(保存の印に「読み取り専用」などを出し、変えたら「写しを保存」を促す。`Mod+S` は「名前を付けて保存」になる)。

## 受け入れ条件

- [x] 判断 (見分け方と画面の扱い) をメモに書いた
- [x] 書き換えないプロファイルを開いて変えても、自動の保存が元のファイルを書かない (テスト: バックエンドの保存の口が呼ばれない)。`Mod+S` と「保存」は保存先を選ばせる
- [x] 写しを保存した後は、写しが保存先になり、自動の保存が戻る
- [x] (A) 書けない場所のプロファイルで、保存に失敗し続けない (失敗を 1 度知らせて読み取り専用の扱いにする、など)。テストで確かめた
- [x] (B) 見本の 2 つのファイルに印があり、`sample-profile.mjs --check` と `apps/studio/src-tauri/tests/sample_profile.rs` が通る。docs/studio.md#プロファイルの形式 に印を書いた
- [x] 文言は日英 (`@ugoe/i18n` の検査が通る)。docs/manual/studio.md#見本のプロファイル の戻し方の説明を書き直した
- [x] 画面を変えたら `just e2e --only studio` と `just ui-audit --only studio` が通る
- [x] `just ci` が通る

## 参考

- T0865 のメモ (注意と起票の候補)、docs/studio.md#シーンとプロファイル・#プロファイルの形式・#プロファイルのパス、docs/manual/studio.md#見本のプロファイル
- apps/studio/src/Control.svelte (`scheduleAutosave`・`saveProfile`)、apps/studio/scripts/sample-profile.mjs、ADR 0028・ADR 0033

## メモ

- T0078 で起票した (元の候補: T0865 (見本のプロファイルや読み取り専用の場所のプロファイルを開いたとき、自動の保存で書き換えない (「写しを保存」に誘う) か検討する))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- Windows・macOS の読み取り専用の扱い (属性・権限) は実機で確かめられない。確かめた範囲をメモに書く。
- 判断 (2026-10-08): 利用者が案 両方 (ファイルが書けるかで見分け、プロファイルの形式にも見本の印 (例: "template": true) を足す。どちらかなら自動の保存をせず「写しを保存」に誘う) を選んだ。

### やったこと (T0866)

- **判断 (見分け方と画面の扱い):** 利用者の判断どおり A と B の両方。
  - B: プロファイルの形式に見本の印 `template` (真偽。省くと偽) を足した。スタジオは書かない (`Profile::write` が外す。写しには付かない)。
    `sample-profile.mjs` が見本の 2 つのファイルに `"template": true` を付け、`resolveSampleProfile` (e2e・ui-audit が使う写し) は外す。
  - A: 開いたときに `profile::is_writable` (書く口で開くだけ。中身も時刻も変えない) で読み取り専用のファイルを見分ける。フォルダに書けないことは、
    今の保存先への保存に失敗したときに見分ける (失敗を 1 度エラーで知らせ、以後は書かない)。
  - どちらも `Stage` の `read_only` (`ReadOnly::Template`・`NotWritable`) にし、`ProfileView.readOnly` で画面に渡す。保存の印は「見本」・
    「読み取り専用」。変えると「写しを保存…」のボタン付きの知らせを出す。「保存」(Mod+S) は保存先を選ばせる (メニューの名前も「写しを保存…」)。
    別のプロファイルを開く前に捨ててよいか確かめる。写しを保存すると写しが保存先になり、印を外して自動の保存に戻る。
- バックエンド: `Stage::save_profile(None)` は書き換えないプロファイルでは書かずに `None` を返す (画面を通らない呼び出しでも書かない)。
  テストは stage.rs の `keeps_template_profiles_and_saves_copies`・`stops_saving_profiles_that_cannot_be_written` (Unix の権限。root では飛ばす)、
  profile.rs の `reads_the_template_mark_but_does_not_write_it`、tests/sample_profile.rs (見本の印がある)。
- 画面: 自動の保存の決まりを `apps/studio/src/lib/autosave.ts` (`Autosave`・`needsSavePath`・`hasUnsavedChanges`) に出し、
  `autosave.test.ts` で書き換えないプロファイルでは保存の口が呼ばれないことを確かめた。文言は日英 (`profile.saveCopy`・`template`・`readOnly`・
  `templateHint`・`readOnlyHint`)。
- 文書: docs/studio.md (形式の表に `template`、新しい節「書き換えないプロファイル」、保存の説明)、docs/manual/studio.md (見本のプロファイルの
  戻し方を書き直し、「書き換えないプロファイル」の節)、docs/manual-check.md、apps/studio/README.md、tests/e2e/README.md。
- 範囲 (`crates:`) の外の小さな変更: tests/e2e/specs/studio.test.mjs に見本の印のあるプロファイルの場面を足した (印を残した写しを最近の
  プロファイルに置いて開き、シーンを足しても待ってもファイルが変わらず、「Save a copy…」が出る)。tests/ui-audit/lib/studio-host.mjs の偽の
  ホストの `ProfileView` に `readOnly` を足した (コミットした見本の場所なので `template`。本物のホストと同じになり、知らせも監査される)。
- scripts/check-pack.mjs: 見本のプロファイルはスタジオが書き換えなくなったので、`--force` が無くてもモデルと同じくいつも上書きで写す
  (前の一式の、印の無い前の版の見本を残さないため)。
- 確かめたこと: `just ci` 通過、`just e2e --only studio` と `just e2e --only studio --manual` 通過 (マニュアルの画像は写しを使うので変わらない)、
  `just ui-audit --only studio` 通過 (指摘 0)。

### やらなかったこと・確かめていないこと

- Windows の読み取り専用の属性・macOS の権限での見分けは実機で確かめていない (`OpenOptions::write` で開けるかを見るだけなので、
  属性の付いたファイルは開けないはず)。Linux の権限 (ファイル 0444・フォルダ 0555) はテストで確かめた。
- 自動の保存の一度きりの失敗 (ディスクがいっぱい、など) でも書き換えないプロファイルになる (写しを保存するか開き直せば戻る)。失敗の種類で
  分けてはいない。
- 保存のダイアログで元の見本のファイルを選べば、利用者が選んだので書く (印は消える)。止めてはいない。

### 起票の候補

- 書き換えないプロファイルの訳を外部 API (`studio-api.md`) のプロファイルの様子にも載せるか (今は操作ウィンドウだけ)。
