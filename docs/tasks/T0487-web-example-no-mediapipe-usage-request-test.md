---
id: T0487
title: "bindings/web: 見本をヘッドレスの Chrome で開き、MediaPipe の利用状況の送信が出ないことを自動で確かめる"
status: todo
milestone: M9
crates: [bindings/web, tests/conformance, docs]
depends_on: []
assignee:
---

# T0487: bindings/web: 見本をヘッドレスの Chrome で開き、MediaPipe の利用状況の送信が出ないことを自動で確かめる

## 目的

T0947 で、bindings/web の見本でも MediaPipe の利用状況の送信 (`odml.pa.googleapis.com`) を止め、版を上げたときにバンドルの送り方の変化を見つける検査を足した。
見本を開いて実際に要求が出ないことは手で確かめただけ (T0947 の候補)。

## 受け入れ条件

- [ ] bindings/web の見本をヘッドレスの Chrome で開き、顔のトラッキングの `FaceLandmarker` を作った (`createFromOptions` の) 後に、
      `odml.pa.googleapis.com` への要求がネットワークに出ないことを確かめる自動のテストがある (カメラは偽の映像。ネットワークの記録の取り方をメモに書く)
- [ ] 止める処理を一時的に外すとテストが落ちることを確かめ、メモに書いた
- [ ] テストの走らせ方を docs/testing.md・tests/conformance/README.md (適合テストに置くなら)・bindings/web/README.md に書き、CI のどこで走るか (`conformance` のジョブなど) を決めてメモに書いた
- [ ] `just ci` が通る

## 参考

- T0947 のメモ (やったこと・起票の候補)
- T0484 (ヘッドレスの Chrome を使う適合テストの前例)、tests/conformance/web/avatar.js・tests/conformance/README.md

## メモ

- T0041 で起票した (元の候補: T0947 (見本の自動のテスト))。
