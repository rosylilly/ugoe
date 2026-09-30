---
id: T0644
title: エディタ: AI への追加の注文と、注文を添えた作り直し
status: in-progress
milestone: M6
crates: [ugoe-ai, ugoe-editor-core, apps/editor, docs]
depends_on: [T0637, T0504]
assignee: claude-worker-T0644
---

# T0644: エディタ: AI への追加の注文と、注文を添えた作り直し

## 目的

利用者の要望。実際の Claude API で自動リギングができたので、AI に追加の注文を書けるようにする。
AI タブに注文の欄を置き、分類・リギングの提案に添えて送る。できた提案を、注文を添えて作り直せるようにする。

## 受け入れ条件

- [ ] `ugoe-ai`: 分類 (`classify_layers`) とリギング (`propose_rig_with_ai`) が、利用者の注文 (自由な文) を受け取り、プロンプトに利用者の注文として区切って入れる。注文を指示の上書き (プロンプトインジェクション) として扱わない書き方にし、出力の JSON Schema は変えない
- [ ] 作り直し: 前の提案の答え (ラベルと RigParams) と注文を送り、新しい提案を作る。前の提案を置き換えるか並べるかを決めて書く
- [ ] 今の仕組みで反映できない注文 (テンプレートにないリグなど) は、AI の答えの理由 (rationale) に「反映できない」と示させ、画面に出す。出力の形に項目を足すなら Schema と録画を合わせる
- [ ] エディタ: AI タブに注文の欄 (複数行)。「Propose rig」「Classify」に添えて送る。提案ごとに「注文を添えて作り直す」。送る内容の同意 (T0638) に注文の文字数も含まれる。注文の欄の中身をプロジェクトに残すかを決めて書く
- [ ] OpenAI 互換のプロバイダ (T0512) でも同じに動く。録画した応答でテスト (注文あり・作り直し・反映できない注文)
- [ ] ヘッドレスのブラウザで確かめる。操作マニュアル (`docs/manual/ai.md`) に書く
- [ ] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0637-editor-ai-proposals.md`・`docs/tasks/T0504-auto-rig.md`・`docs/tasks/T0510-ai-acceptance.md` のメモ
- `docs/ai-integration.md#自動リギング`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
