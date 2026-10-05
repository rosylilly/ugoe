---
id: T1609
title: "エディタ: モジュールの適用のエラーに直し方の案内を出し、keep_look の作り直しの警告を捨てない"
status: done
milestone: M9
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: []
assignee: claude-worker-T1609
---

# T1609: エディタ: モジュールの適用のエラーに直し方の案内を出し、keep_look の作り直しの警告を捨てない

## 目的

- T0678 で `EditError::Module` とコード (`module.collider_needs_base_physics` など) を足したが、画面は文を出すだけで直し方 (本体を開くなど) を案内しない (T0678 の候補)。
- `Document::set_module_settings` (`crates/ugoe-editor-core/src/document.rs`) は `keep_look` のときの `inference::rederive` の警告 (`RederiveWarning`) を捨て、
  `apply_with_report` は空の報告を返す (T0667 の所見、T0678 の候補)。

## 受け入れ条件

- [x] モジュールの適用のエラーのうち直し方があるものに、案内 (文と、できれば本体を開くなどの操作) を出す。どのコードに案内を付けたかを docs/editor.md に書いた
- [x] `set_module_settings` の `keep_look` の作り直しの警告が `ApplyReport` で返り、画面に出る。editor-core のテストがある
- [x] `en.json`・`ja.json` に訳があり、`just i18n` が通る
- [x] `just ci` が通る

## 参考

- T0678・T0667 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0678 の案内、T0678/T0667 の `keep_look` の警告)。
- やったこと:
  - editor-core: `Document::set_module_settings` が `keep_look` の `inference::rederive` の警告を捨てずに返し、`apply_with_report`
    (と `apply_coalescing`) の `ApplyReport.warnings` に載せる。付け替えで親が変わった対の確かめ (`check_reparented_pairs`) も、
    ほかの編集と同じく行う (名前を変えたときは ID が変わるので前のモデルと突き合わせられず、その確かめは働かない)。
    先に失敗するテスト `module_settings_keep_look_reports_failed_remirrors` (ミラー元から離した `SkirtR` を持つ Dress を Hair へ移す)。
  - 画面: 警告はすでに `EditorDocument.applyWith` → インスペクタの `RederiveNotice` に出る経路 (T0689) なので、バックエンドが返せば出る
    (`document.test.ts` に報告の警告が `warnings` に入るテストを足した)。
  - 案内: `moduleErrorHelp` (`src/lib/moduleProject.ts`) と `ModuleErrorHelp.svelte`。インスペクタの上のエラーとモジュールの設定の画面に、
    `edit.read_only_base`・`module.read_only_base`・`module.collider_needs_base_physics`・`module.compose_failed` (原因が
    `compose.parameter_not_exposed`・`unknown_mask`・`unknown_collider`・`pose_not_exposed`・`missing_parameter`・`missing_slot`・`avatar_level`)
    の案内の文と「Open base project」(本体が `.ugoeproj` のとき。`FileMenuFlow.openModuleBase`。未保存なら確かめる・設定の画面を閉じる) を出す。
    `.ugoe` の本体は開けないので書き出し直すよう案内する。`EditorDocument.failure` に元のエラー (`Message`) を残す。
    一覧と理由は `docs/editor.md#モジュールの適用のエラーの案内`。
  - テスト: `moduleProject.test.ts`・`fileMenu.test.ts`・`document.test.ts`。`just i18n`・`just ci` は通った。
- やらなかったこと: 画面を実際に起動しての目視は行っていない (vitest と svelte-check まで)。`module.unknown_slot`・`module.invalid_name`・
  `edit.module_shape` には案内を付けていない (文で足りるか、モジュールの側を直すもの)。
- 起票の候補:
  - `keep_look` で名前も同時に変えると、`check_reparented_pairs` が前のモデルと ID で突き合わせられない。接頭辞を付け替えて比べるか検討する。
  - 「Open base project」で本体を開いた後、直し終えたらモジュールに戻る道 (最近開いたモジュールを開き直すボタンなど) が無い (本体の問題の枠の同じボタンも同じ)。
