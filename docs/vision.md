# ビジョン

## 目標

1. **Live2D Cubism と機能的に互換であること。** Cubism で作れる表現 (パラメータ駆動の変形、デフォーマ階層、クリッピング、物理、モーション、表情など) を、ugoe でも同等の品質で作れて動かせる。
2. **ポータブルなランタイムであること。** 同じ Rust コアがネイティブ (Windows / macOS / Linux / iOS / Android)、Web (WebGPU / WebGL2)、ゲームエンジン (Unity / Unreal / Godot / Bevy) で動く。
3. **ツールチェインが自動化しやすいこと。** エディタの操作はすべてシリアライズ可能なコマンドとして表現する。これにより CLI・スクリプト・AI エージェントからも同じ操作ができる。
4. **AI によるリギング支援。** PSD から「とりあえず動く」モデルを自動で作り、人間はそれを直すことに集中できるようにする。

## 非目標

- Live2D のファイル形式 (`.moc3`、`.cmo3`、`.can3` など) の読み書き。
- Live2D Cubism SDK の API 互換 (関数名・クラス構造の再現)。
- Live2D の内部アルゴリズムのビット単位の再現。見た目と操作感が同等であれば十分とする。
- 3D モデルの扱い。

> Live2D の JSON 系ファイル (`.motion3.json` など) のインポートは、法的・ライセンス上の確認が済むまで扱わない。確認できたら ADR を書いて判断する。

## 機能互換の範囲

Cubism の機能と、ugoe での対応と、目標マイルストーン ([roadmap.md](roadmap.md)):

| Cubism の機能 | ugoe での名前 | マイルストーン |
| --- | --- | --- |
| パラメータ | Parameter | M1 |
| アートメッシュ | Mesh | M1 |
| パーツ (不透明度・階層) | Part | M1 |
| キーフォーム (最大 3 軸 + 組み合わせ) | KeyformGrid (軸数に制限なし) | M1 |
| ワープデフォーマ | WarpDeformer | M1 |
| 回転デフォーマ | RotationDeformer | M1 |
| 描画順 | draw_order キーフォーム | M1 |
| クリッピングマスク・反転マスク | masks (`MaskRef`) / mask_mode / mask_only / mask_strength | M1 |
| 合成モード (通常・加算・乗算) | BlendMode | M1 |
| 乗算色・スクリーン色 | multiply_color / screen_color | M1 |
| グルー | Glue | M2 |
| ブレンドシェイプ | ParameterKind::BlendShape | M3 |
| 物理演算 | PhysicsSettings | M3 |
| モーション (カーブ・フェード・ループ) | Motion | M3 |
| 表情 | Expression | M3 |
| 目パチ・呼吸・リップシンク | エフェクト | M3 |
| ポーズ (パーツ切り替え) | Pose | M3 |
| 当たり判定 | HitArea | M3 |
| オフスクリーン描画 (パーツ単位の合成) | 未定 | M5 以降 |
| 拡張合成モード (スクリーン・オーバーレイなど) | BlendMode の拡張 | M5 以降 |

ugoe 独自の拡張 (互換の範囲外):

- キーフォーム軸の数に制限がない。
- 標準パラメータ ID ([spec/model.md](spec/model.md#標準パラメータ)) を使って、モーションや物理プリセットをモデル間で使い回せる。
- マスクの拡張 ([ADR 0009](adr/0009-clipping-mask-semantics.md)): マスク専用メッシュ、通常マスクと反転マスクの併用、共通部分、入れ子の継承、効き具合 (`mask_strength`) の調整。
- すべての編集操作をコマンドとして記録・再生できる。
- AI による自動リギング。
