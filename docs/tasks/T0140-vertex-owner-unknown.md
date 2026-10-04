---
id: T0140
title: 物理の頂点への出力の対象 (VertexOwner) を未知の値として読めるようにする
status: in-progress
milestone: M9
crates: [ugoe-model, ugoe-physics, ugoe-runtime, ugoe-editor-core, ugoe-ai, ugoe-cli, apps/editor, docs, spec]
depends_on: [T0136]
assignee: claude-worker-T0140
---

# T0140: 物理の頂点への出力の対象 (VertexOwner) を未知の値として読めるようにする

## 目的

T0136 のレビューで見つけた前方互換の穴を塞ぐ。`PhysicsTarget::Vertices` の `owner` (`VertexOwner`、JSON では `{"mesh": ..}` / `{"warp": ..}`) は
外部タグの enum で `Unknown` のバリアントを持たない。新しい版で対象の種類 (例: `{"bone": ..}`) が増えると、`PhysicsTarget` 自体は拡張可能なのに、
物理の設定全体の読み込みがエラーになる (ADR 0013、`docs/spec/file-formats.md#未知の値の読み込み`)。

`ugoe-model` の公開 API の変更なので、multi-agent.md に従って「インターフェースを先に入れる」小さな PR にする。

## 受け入れ条件

- [ ] `VertexOwner` に `Unknown` を足し、未知のキーの 1 要素のオブジェクトを `Unknown` として読む (外部タグには `#[serde(other)]` が使えないので手で書く)。`Unknown` は書き出せない
- [ ] `ugoe_model::unknown::in_physics` が場所 (`rigs[..].outputs[..].target.owner` など) を返す。フォールバック (その出力を無視する) を物理・合成・エディタで適用する
- [ ] `docs/spec/model.md#未知の値`・`docs/spec/file-formats.md#未知の値の読み込み` の一覧と、公開仕様 (`spec/`) の対応する節を更新した
- [ ] 未知の対象を含む JSON と MessagePack を読むテストを足した
- [ ] `just ci` が通る

## 参考

- docs/tasks/T0136-review-core-model.md (所見の表)
- docs/adr/0013-*.md、docs/spec/file-formats.md#未知の値の読み込み

## メモ
