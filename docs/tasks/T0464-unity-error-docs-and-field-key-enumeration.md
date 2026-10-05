---
id: T0464
title: "Unity: 読み込みのエラーの説明とテストを直し、補足の値のキーを C ABI から列挙するか決める (判断が要る)"
status: todo
milestone: M9
crates: [bindings/unity, ugoe-ffi, docs]
depends_on: []
assignee:
---

# T0464: Unity: 読み込みのエラーの説明とテストを直し、補足の値のキーを C ABI から列挙するか決める (判断が要る)

## 目的

T0457 で C ABI のモデルの読み込みの失敗にも種類 (`Kind`) が付き、T0458 で Unity の `UgoeException.Fields` のキーを `codes.json` と比べる
`FieldKeys` の写しを置いた。次が残っている (T0457・T0458 の候補)。

1. Unity の `UgoeResult.Format`・`InvalidModel`・`UgoeException` の説明が、モデルの読み込みに種類が無かったころのまま。`Puppet.FromBundle` の失敗の `Kind` を確かめるテストも無い。
2. 着せ替えの失敗で新しいキー (`mask`・`collider` など) が `Fields` に入ることを、実際の失敗 (例: `unknown_mask` のモジュール) で確かめる C# のテストが無い (今は `codes.json` との一覧の比較だけ)。
3. **判断が要る:** C ABI に補足の値のキーを列挙する関数 (`ugoe_last_error_field_count`・`ugoe_last_error_field_at` など) を足せば、Unity の `FieldKeys` の写しが要らなくなる。
   T0458 は「ABI を増やすほどではない」として足さなかった。C ABI の公開 API を増やすかを利用者が決める。

## 受け入れ条件

- [ ] `UgoeResult.Format`・`InvalidModel`・`UgoeException` の doc コメントと bindings/unity の README を、読み込みの失敗の種類に合わせて直した
- [ ] `Puppet.FromBundle` の失敗 (壊れたバンドル・上限の超過) の `Kind` を確かめる C# のテストがある
- [ ] 実際の着せ替えの失敗 (`unknown_mask` のモジュールなど) で、`Fields` に新しいキーが入ることを確かめる C# のテストがある
- [ ] キーを列挙する関数を足すかの案と推奨をメモに書き、利用者の判断を得た。足すなら C ABI のヘッダ・`just refs`・Unity の宣言を揃え、`FieldKeys` の写しを外した
- [ ] Unity のテストを README の手順で流した (実際の Unity が無ければ、確かめた範囲をメモに)
- [ ] `just ci` が通る

## 参考

- T0457・T0458 のメモ (起票の候補)
- crates/ugoe-ffi/README.md、bindings/unity/README.md

## メモ

- T0032 で起票した (元の候補: T0457 (Unity)・T0458 (2 つ))。
