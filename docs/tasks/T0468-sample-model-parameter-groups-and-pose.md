---
id: T0468
title: 見本のモデルにパラメータのグループとポーズを持たせるか決める
status: done
milestone: M9
crates: [ugoe-cli, assets/samples, bindings/web, tests/conformance, docs]
depends_on: []
assignee: claude-worker-T0468
---

# T0468: 見本のモデルにパラメータのグループとポーズを持たせるか決める

## 目的

`assets/samples` の見本のモデル (`ugoe sample` が作る) は、パラメータのグループとポーズを持たない。T0463 の `@ugoe/runtime` の
`*_json()` の出力のキーを本物の wasm で比べるテスト (`queries`) は、フィクスチャにグループとポーズを足して確かめている (T0463 の候補)。
見本に持たせれば、アプリ・npm の例・適合テストで同じものを確かめられる。見本の大きさと見た目 (avatar-*.jpg) が変わるので、持たせるかを決める。

## 受け入れ条件

- [x] 持たせるかの判断と理由をメモに書いた (持たせないなら、ここで閉じる)
- [x] 持たせるなら: `ugoe sample avatar`・`ugoe sample dress-up` がパラメータのグループとポーズ (例: 腕や手の差分の切り替え) を持つモデルを作り、`assets/samples` を作り直した。assets/samples/README.md を更新した
- [x] 持たせるなら: `queries` のテストが見本を使う (フィクスチャへの足し込みを減らせるなら減らした)
- [x] `just ci` が通る

## 参考

- T0463 のメモ (起票の候補)
- assets/samples/README.md、crates/ugoe-cli/src/sample/

## メモ

- T0032 で起票した (元の候補: T0463 (見本のモデル))。

### 判断と理由

- **持たせた。** 影響を調べた結果、どれも小さかったため。
  - 見た目: ポーズはどちらも先頭の要素が既定の姿で、後ろの要素 (`Wave`) はパーツの不透明度を 0 にしたので、ポーズの段を通らない描画
    (`ugoe render` の時間を進めない描画・適合テストの静的なケース) でも見た目は変わらない。`avatar.ugoe`・`dress-up.ugoe` の既定の
    `ugoe render` の PNG が前とバイト単位で同じことを確かめた。`avatar-*.jpg`・`dress-up.jpg` は作り直していない。
  - 大きさ: `avatar.ugoe`・`avatar-lit.ugoe`・`avatar-atlas.ugoe` は各 +448 バイト、`dress-up.ugoe` は +4KB (手のテクスチャ)。モジュールは変わらない。
  - 適合テストの参照値: 参照はコミットしておらず実行のたびに `ugoe::Puppet` で作るので、作り直すものは無い (参照値は変えていない)。
  - e2e: 見本のパラメータの区切りやポーズを数える検査は無い。文書のスクリーンショット (docs/manual/images) は手で撮ったもので、
    パラメータの一覧を写したもの (ビューア・エディタのプレビュー) は、今後撮り直すと区切りが出る (今のものは古くなるだけで、検査は無い)。
  - パラメータのグループは表示用で評価に効かない。利点: アプリ・npm の例・適合テストでグループとポーズを見本のまま確かめられる。
- 判断が分かれうる点 (利用者に見せて変えられる): ポーズの中身。アバターは PSD に差分の絵が無いので「耳飾りの有無」(中身の無いパーツとの
  グループ) にし、着せ替えの本体はコードで描けるので「手を振る」差分を描いた。

### やったこと

- `crates/ugoe-cli/src/sample.rs`: パラメータのグループの表 `PARAMETER_GROUPS` (`Head`・`Body`・`Eyes`・`Brows`・`Mouth`。2 つの見本で共有)。
  アバターはグループの定義を組み始める前の空のモデルに書き (足すコマンドが無いため)、割り当ては `SetParameterGroup`。
  ポーズは `add_pose`: パーツ `earwear` (顔の子) へ耳飾りを `SetMeshProperties` で移し、中身の無い `no_earwear` と 1 グループにする (`SetPose`)。
  テスト `parameters_are_grouped_and_the_pose_hides_the_earrings`、`committed_sample_is_up_to_date` にグループ・パーツ・ポーズの比較を足した。
- `crates/ugoe-cli/src/sample/dress_up.rs`: 同じグループ。パーツ `HandsDown` (中身なし)・`Wave` (不透明度 0) とポーズ、手の絵 `waving_hand`
  (メッシュ `HandWave`、体のワープの子、描画順 700)。テスト `parameters_are_grouped_and_the_pose_raises_a_hand` (モジュールを付けても切り替わる)。
- `assets/samples` を作り直した (`avatar`・`avatar-lit`・`avatar-atlas`・`dress-up/dress-up.ugoe`)。README に「パラメータのグループとポーズ」の節と、
  組み立て方の表の行・`ugoe render` で試す方法を書いた。crates/ugoe-cli/README.md にも一言。
- 適合テストの `queries`: 項目 `pose-and-groups` (フィクスチャにグループを足していたもの) を消し、ポーズとグループは見本から出させる。
  `motion-events` も見本のアバターで動かすようにし、フィクスチャ `pose_and_effects` を書き出す処理 (run.mjs の `ugoe fixture` と配信) を
  消した (`--only queries` で ugoe-cli をビルドしなくなった)。tests/conformance/README.md・bindings/web/README.md を直した。
  `just conformance --only queries` が通り、`coverage` (すべての形を比べた) も ok。
- `crates:` の外の小さな変更: `crates/ugoe-ai/tests/module_rig_sample.rs` の 2 つのテストが、見本の本体にポーズのグループが無いことを
  前提にしていた (作業用のモデルのポーズは本体のグループの後ろにモジュールのグループが並ぶ) ので、本体のグループの数を足して数えるようにした。
- 確かめたこと (WSL2): `just ci` (main の c7d44594 の上で通った)、`just conformance --only queries`、`ugoe render` での既定の見た目の一致と、
  モーションのパーツの不透明度のカーブで `Wave`・`no_earwear` を表示した描画。

### やらなかったこと

- `just conformance` の全体 (c・wasm・unity・godot) は流していない (`queries` だけ)。参照はその場で作るので、見本の変更で参照値の作り直しは要らない。
- `avatar-*.jpg`・`dress-up.jpg` は作り直していない (既定の見た目が同じため)。ポーズの差分を示す画像も足していない。
- 文書のスクリーンショット (docs/manual/images) は撮り直していない。

### 起票の候補

- エディタ中核にパラメータのグループを足す・消す・名前を変えるコマンドが無い (`SetParameterGroup` は既存のグループへの割り当てだけ)。
  見本のアバターはグループの定義を空のモデルに直接書いている。
- ビューアにポーズの要素を選ぶ画面が無い (エディタのプレビューにはある)。見本で試せるようにするなら。
- 着せ替えの見本のスロット `Outfit` の `pose_parts` に `Wave` を公開し、`Sailor` の袖を `links` で追従させる見本 (モジュールのポーズの追従の見本)。
- 見本にポーズを要求するモーション (例: `wave`。パーツの不透明度のカーブ) を足す (今は `ugoe render` で試すのにモーションの JSON を手で書く)。
- 見本のパラメータの一覧を写したスクリーンショットの撮り直し (区切りが出るようになった)。

