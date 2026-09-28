---
id: T0308
title: 当たり判定 (HitArea) の API
status: done
milestone: M3
crates: [ugoe, ugoe-runtime, ugoe-wasm]
depends_on: []
assignee: claude-worker-T0308
---

# T0308: 当たり判定 (HitArea) の API

## 目的

`Model::hit_areas` (メッシュの変形後の形で判定する) を、アプリがキャンバス座標で問い合わせられるようにする。
アバタープロファイルの標準の名前 (`Head`・`Body`) の対応 (`Model.avatar.hit_areas`、T0110) を引けるようにもする。

## 受け入れ条件

- [x] `Puppet` (と必要なら `ModelInstance`) に、点を含む当たり判定の ID を返す API がある (手前から順。非表示・不透明度 0 の扱いを決めて書く)
- [x] 変形後の形で判定するテスト (パラメータを動かすと当たる場所が変わる)
- [x] wasm (`WebPuppet`) からも呼べる
- [x] `docs/spec/evaluation.md` か `docs/spec/model.md` に判定の規則を書く
- [x] `just ci`・`just wasm-check` が通る

## 参考

- `crates/ugoe-wasm/src/hit.rs` (エディタの選択用の、要素のヒットテスト)

## メモ

- **規則** (`docs/spec/evaluation.md#9-当たり判定`): 変形後の三角形の内外で判定 (辺の上を含む、向きは問わない)。
  **可視性・不透明度は見ない** (判定専用の透明なメッシュを置けるように。除きたいホストは `drawables()[m].visible`
  を見る)。複数当たったら手前から (描画順の逆、同じメッシュどうしは宣言順)。判定は直前の `update()` の形を読むだけ。
- **ランタイム** (`ugoe-runtime/src/hit.rs`): `ModelInstance::mesh_contains` / `hit_area_contains` / `hit_areas_at`
  (添字)。`CompiledModel` が当たり判定のメッシュをコンパイル時に解決する (`hit_area_index` / `hit_area_mesh`)。
- **SDK**: `Puppet::hit_areas_at(point) -> Vec<&str>` と `Puppet::hit_test(id, point) -> bool` (未知の ID は `false`)。
- **wasm**: `WebPuppet.hit_area_ids()` / `hit_areas_at(x, y)` / `hit_area_test(id, x, y)`。
- エディタの要素選択 (`ugoe-wasm/src/hit.rs`) の三角形判定はランタイムの `mesh_contains` に移し、可視性の条件だけを
  wasm 側に残した (挙動は同じ。三角形判定のテストもランタイムへ移した)。
- 範囲外の変更: `apps/editor/src/lib/viewport/ugoe-wasm-stub.d.ts` に新しい wasm メソッドの型を 3 行足した
  (`just wasm` の生成物と一致を確認済み)。
- **やらなかったこと / 後続**: アバタープロファイルの標準名 (`Head`・`Body`、`Model.avatar.hit_areas`) からの判定は、
  T0110 がモデルに対応を入れていないので未実装。T0110 のマージ後に `Puppet` (と `WebPuppet`) へ標準名で引く版
  (例: `hit_test_standard(name, point)`) を足す小さなタスクを起票する。仕様書にも `> 未決:` で書いた。
  エディタの UI (当たり判定の作成・表示) はこのタスクの範囲外。
