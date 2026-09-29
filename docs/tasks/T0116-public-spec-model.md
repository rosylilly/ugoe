---
id: T0116
title: 公開仕様: 骨格とモデル・バンドルの形式 (英語)
status: done
milestone: M5
crates: [spec, docs, scripts, AGENTS.md]
depends_on: [T0111, T0112, T0113, T0311]
assignee: claude-worker-T0116
---

# T0116: 公開仕様: 骨格とモデル・バンドルの形式 (英語)

## 目的

ADR 0021 に従い、リポジトリの `spec/` に英語の公開仕様 (CC BY 4.0) を作る。まず骨格と、モデルのデータ (`model.json`)・バンドル (`.ugoe`)・前方互換 (機能の宣言とフォールバック) を書く。

## 受け入れ条件

- [x] `spec/README.md` (目的・対象の読者・バージョン・草案であること・用語)、`spec/LICENSE` (CC BY 4.0 の全文)。ライセンスの検査 (`just licenses` や README の表) に `spec/` を反映する
- [x] `spec/model.md`: すべての型とフィールド、既定値、検証の規則。実装の型名やタスク ID に頼らず、それだけで読めること
- [x] `spec/bundle.md`: `.ugoe` のバイト列の形式 (チャンク・MessagePack・zstd・テクスチャ)、`schema_version`、機能の宣言、未知の値のフォールバック
- [x] `docs/spec/` (日本語) との対応表を置き、食い違いが見つかったら両方を直す
- [x] `just ci` が通る

## 参考

- `docs/adr/0021-public-specification.md`
- `docs/spec/model.md`、`docs/spec/file-formats.md`

## メモ

### やったこと

- `spec/README.md` (目的・読者・版の対応表・草案・RFC 2119 の用語・用語集・ライセンスと取り込みの扱い)、`spec/LICENSE` (CC BY 4.0 の legalcode.txt を公式の URL から取得してそのまま置いた)。
- `spec/model.md`: `Model` のすべての型とフィールド (型・既定値・必須か)、JSON と MessagePack の対応 (`null`・enum の 4 つの表現・数値の型)、座標系、ID、既知のパラメータ ID と値域、アバタープロファイル・スロット・オーサリング用のデータ、物理・モーション・表情のデータ、モジュールと合成、未知の値とフォールバック、検証の規則。
- `spec/bundle.md`: コンテナのバイト列 (ヘッダ・チャンク表・16 バイト境界・予約フラグ)、チャンクの種類と個数、MessagePack・zstd・上限、テクスチャと法線マップの対応、バンドルとモジュールのファイル、読み込みの手順と検査の順序、版 (コンテナ・スキーマ・プロファイル)、前方互換 (未知のチャンク・フィールド・値、`FEAT`、基準の機能、1.0 以降の機能の表、再保存)、単体の JSON。
- 対応表は `docs/spec/public-spec.md` に置いた (日本語の開発用の文書から英語版へ向ける。英語版は日本語の文書にリンクしない方針のため)。
- ライセンス (受け入れ条件のため、範囲外のルートの README.md も直した): `scripts/check-licenses.mjs` が `spec/LICENSE` を CC BY 4.0 の条文か検査する (`just licenses`)。README のライセンスと文書の一覧に `spec/` を足した。
- `scripts/check-md-links.mjs`: Markdown の相対リンクとアンカー (GitHub の規則) の検査。`node scripts/check-md-links.mjs docs spec` で全文書が通る。

### 境目

- 評価・描画・物理・モーションの **意味 (式・順序)** は T0117、アバタープロファイルの **準拠レベル・トラッキングの式・アプリの規則** は T0118。`spec/model.md` は、これらのデータ (フィールド・既定値・検証) だけを書き、意味は「planned の文書」と書いてリンクしない (まだファイルがないため)。T0117・T0118 で置き換えてリンクにする。
- 機能の宣言・`test.` の予約は T0115 の資料 (`assets/compat/`、`docs/testing.md`) と合わせた。
- AGENTS.md の「仕様とコードの同期」に、`spec/` の対応する節も同じ PR で直す規則を足した (team-lead の承認のうえで crates: に AGENTS.md を追加。T0119 の CI の検査ができるまで `spec/` が古くなるのを防ぐため)。
- `schema_version` を先に確かめる実装の変更は T0122 (日本語の文書は今の振る舞いのまま)。

### 実装と docs/spec の食い違い (実装を正として直したもの)

- `docs/spec/motion.md`: ベジェの制御点を「読み込み時にクランプ」とあったが、実装は評価時だけクランプする。直した。
- `docs/spec/model.md`: 必須・省略・`null` の扱い、`multiply_color` などの既定値、`indices` が `u16` であることが書かれていなかった。「JSON と MessagePack での表現」の節を足した。`evaluation.md#グルー` のリンク切れを直した。
- `docs/spec/file-formats.md`: MessagePack の入れ子の上限 (128) が書かれていなかった。`schema_version` をデコードの後に調べることを明記した (公開仕様は先に調べることを SHOULD にした → T0122)。`test.` の予約を足した。
- `docs/spec/modules.md`: 合成でテクスチャの `normal_map` の名前は付け替えないこと、挟むパーツの名前と親を足した。
- `docs/manual/physics.md`: `#出力` のリンク切れを直した。

### 残した課題・後続の候補

- `check-md-links.mjs` を `just ci` / CI に入れる (justfile は範囲外。T0119 の同期の検査と一緒に)。
- スロットの `colliders` は物理の設定があるときだけ検査される (`validate_physics`)。物理のないバンドルでは存在しない衝突体を指しても通る。
- 物理の出力の `particle` の範囲は検証しない (ランタイムが無視する)。公開仕様にもそう書いた。
- `docs/spec/avatar-profile.md` の「ライセンスは MIT」は、英語版 (CC BY 4.0) を作る T0118 で整理する。
