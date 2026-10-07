---
id: T0189
title: "png クレートの上流に、PLTE の長さが 3 の倍数でないときのパニックを報告する (外部への投稿。利用者の承認が要る)"
status: todo
milestone: M11
crates: [docs]
depends_on: []
assignee:
---

# T0189: png クレートの上流に、PLTE の長さが 3 の倍数でないときのパニックを報告する (外部への投稿。利用者の承認が要る)

## 目的

T0188 の fuzz (`image` のターゲット) が、`png` 0.18.1 (その時点の最新) のパニックを見つけた。PLTE チャンクの長さが 3 の倍数でないと、
パレットの展開で範囲外の添字になる。ugoe は `ugoe_format::image::decode_png` で `read_info` の後に断るようにして避けた
(`ImageError::Decode`。入力は `crates/ugoe-format/tests/broken_input.rs` の `found_by_fuzzing`)。ほかの利用者のためにも、上流に報告する。

**利用者の承認が要る** (外部 (GitHub の image-rs/image-png) への投稿)。作業者は報告の文を用意し、利用者の承認を得るまで投稿しない。
誰の名前で投稿するか (利用者が自分で投稿するか) も利用者が決める。

## 受け入れ条件

- [ ] 上流の issue・PR と最新の版を調べ、同じ不具合がすでに報告・修正されていないかを確かめた (されていれば、その URL と直った版をメモに書き、投稿はしない)
- [ ] 報告の文 (英語。版・再現の最小の入力 (16 進数か Rust の小さな再現のコード)・パニックの場所とメッセージ・期待する振る舞い) をメモに書いた。
      ugoe のコードを写さず、`png` の公開 API だけで再現する
- [ ] 利用者の承認を得て投稿した (か、利用者が投稿した) URL をメモに書いた。承認が得られなければ `blocked` にして止まる
- [ ] 上流で直ったときの扱い (`decode_png` の検査は残す・`png` の版を上げる) を crates/ugoe-format/src/image.rs の近くのコメントか README から
      たどれるようにした (範囲の外の 1 行のコメントなら、行ってメモに書く)
- [ ] `just ci` が通る

## 参考

- T0188 のメモ (回した時間と見つかったもの・起票の候補)、crates/ugoe-format/src/image.rs (`decode_png`)、crates/ugoe-format/tests/broken_input.rs
- https://github.com/image-rs/image-png

## メモ

- T0068 で起票した (元の候補: T0188 (`png` の PLTE の長さの不具合を上流に報告する))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
