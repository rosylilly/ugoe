---
id: T0884
title: "ui-audit: 3 つのアプリをまとめて流す重い回でも、スタジオの -autosave-error の場面が帯を待てるようにし、場面の失敗で全体が時間切れにならないようにする"
status: in-progress
milestone: M8
crates: [tests/ui-audit, docs]
depends_on: []
assignee: claude-worker-T0884
---

# T0884: ui-audit の autosave-error の場面の揺れ

## 目的

T0880 の確認で、`just ui-audit` (3 つのアプリをまとめて流す重い回) の強制色 (forced colors) の回で、スタジオの `-autosave-error` の場面が
「知らせが出ない」(自動の保存の失敗の帯が出ない) と落ち、そのまま全体が時間切れになった。`just ui-audit --only studio` を流し直すと通る。
負荷で帯を待つ時間 (`tests/ui-audit/lib/apps.mjs` の `autosaveFailureScene` まわり。帯や前の知らせを待つ 5 秒) が足りないと見られる。
重い回でも場面が安定して通り、1 つの場面の失敗で全体が止まらないようにする。

## 受け入れ条件

- [ ] 落ちる原因を確かめた (帯を待つ時間・自動の保存の時機 (変えてから約 0.8 秒)・前の場面の知らせが閉じる待ち・強制色の回だけで起きる理由)。
      確かめ方 (CPU に負荷をかけて `--only studio` を流す、まとめて流すなど) と結果をメモに書いた
- [ ] 帯を待つ処理を負荷に強くした (固定の待ち (`waitForTimeout`) を条件の待ちにする、待つ時間を延ばす、自動の保存を偽のホストから直に起こす、など。
      選んだ案と理由をメモに)
- [ ] 場面が失敗したとき、全体が時間切れにならずに、その場面の失敗として報告して次へ進む (か、すぐに失敗で終わる)。時間切れの扱いを直した
- [ ] `just ui-audit` (3 つのアプリをまとめて) を少なくとも 2 回流して、スタジオの `-autosave-error`・`-autosave-denied` が通ることを確かめた
- [ ] tests/ui-audit/README.md の該当の説明を直した (待ち方を変えたなら)
- [ ] `just ci` が通る

## 参考

- T0880 のメモ (確かめたこと)、T0874 (自動の保存の失敗の場面を足した)、`tests/ui-audit/lib/apps.mjs` (`autosaveFailureScene`・スタジオの場面)、tests/ui-audit/README.md

## メモ

- T0089 で起票した (元の候補: T0880 (ui-audit のスタジオの -autosave-error の場面が、3 つのアプリをまとめて流す重い回の強制色で「知らせが出ない」と落ちて
  時間切れになった。`--only studio` では通る。帯を待つ 5 秒を負荷に強くするか、時間切れの扱いを直す))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。番号はスタジオの帯 (T0874 と同じ)。
- スタジオの画面のコード (apps/studio) を変える必要が分かったら (自動の保存の時機など)、案をメモに書いて止まる。
