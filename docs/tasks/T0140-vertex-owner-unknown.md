---
id: T0140
title: 物理の頂点への出力の対象 (VertexOwner) を未知の値として読めるようにする
status: done
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

- [x] `VertexOwner` に `Unknown` を足し、未知のキーの 1 要素のオブジェクトを `Unknown` として読む (外部タグには `#[serde(other)]` が使えないので手で書く)。`Unknown` は書き出せない
- [x] `ugoe_model::unknown::in_physics` が場所 (`rigs[..].outputs[..].target.owner` など) を返す。フォールバック (その出力を無視する) を物理・合成・エディタで適用する
- [x] `docs/spec/model.md#未知の値`・`docs/spec/file-formats.md#未知の値の読み込み` の一覧と、公開仕様 (`spec/`) の対応する節を更新した
- [x] 未知の対象を含む JSON と MessagePack を読むテストを足した
- [x] `just ci` が通る

## 参考

- docs/tasks/T0136-review-core-model.md (所見の表)
- docs/adr/0013-*.md、docs/spec/file-formats.md#未知の値の読み込み

## メモ

- やったこと
  - `VertexOwner` に `Unknown` を足し、`Deserialize` を手で書いた (`CurveTarget` と同じ形)。知らないキーの 1 要素のオブジェクト (中身は読み捨てる) と、
    `mesh` / `warp` 以外の文字列 (新しい版の中身の無い種類) を `Unknown` として読む。知っているキーで中身が ID でないもの・要素が 0 個や 2 個以上・`"mesh"` の文字列はエラー。
    `Unknown` は `skip_serializing` で書き出せず、ts / schema には出さない (生成物は doc コメントの差分だけ)。
  - `unknown::in_physics` が `rigs[..].outputs[..].target.owner` を返す (`read_bundle_with_report` の警告・`model_to_json` 系の `UnknownValues`・エディタの保存の拒否に乗る)。
  - フォールバック (その出力を無視する): 物理 (`Target::Ignore`、動的物理の判定に数えない、出力の値を計算しない)・検証 (重みの数と範囲も検査しない)・
    合成 (付け替えずにそのまま残す)・エディタ (`forget_removed` は外さない、モジュールの分割と差分は何もしない)。
  - テスト: JSON と MessagePack (`to_vec_named`、`.ugoe` と同じ) の読み込み、壊れた形のエラー、`in_physics` の場所、物理で無視されること、合成とエディタで残ること、保存の拒否。
    MessagePack のテストのため `ugoe-model` の dev-dependencies に `rmp-serde` を足した (ワークスペースの依存。ランタイムの依存は増えない)。
  - 仕様: `docs/spec/model.md#未知の値`・`#物理の検証`、`docs/spec/file-formats.md#未知の値の読み込み` の表、`docs/spec/physics.md`、`spec/model.md` (`VertexOwner` の説明と Unknown values の表。「not extensible」を消した)。
  - `crates:` の外の小さな変更: `scripts/check-spec.mjs` の `notExtensible` を外し、`tools/spec-check` (公開仕様だけで書いた読み込み器) も未知の `VertexOwner` を警告付きで読んで出力を `unknown` にするようにしてテストを足した (`just spec` が公開仕様との一致を検査するため)。
  - `SCHEMA_VERSION` は変えていない (既存のファイルの読み書きは変わらない。ほかの `Unknown` の追加と同じ)。
- やらなかったこと
  - エディタの `SetPhysicsOutputs` / リグの追加は、未知の出力先 (`PhysicsTarget::Unknown`) と同じく未知の対象を拒否しない (既存の振る舞いに揃えた)。保存の時点で `UnknownValues` で拒否される。
  - `apps/editor` のフロントエンドは変えていない (`Unknown` は書き出せないので IPC に出てこない)。
- 起票の候補
  - エディタのコマンド (`SetPhysicsOutputs`・`CreatePhysicsRig` など) で、未知の出力先・対象を受け付けるか拒否するかを決める (今は `PhysicsBlend::Unknown` だけ拒否していて揃っていない)。
  - `assets/compat/` の未来の版を模したファイルに、未知の `VertexOwner` を含む物理を足す。
- `just ci`・`just wasm-check` は rebase 後に通った。
