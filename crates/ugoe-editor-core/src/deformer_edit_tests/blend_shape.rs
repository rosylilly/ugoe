//! ブレンドシェイプ格子のセルの見た目を保つことのテスト。

use super::*;

#[test]
fn deformer_edits_keep_the_look_of_blend_shape_cells() {
    // 作成・付け替え・外す・回転の中心の移動は、ブレンドシェイプ格子のセルも写し直す
    // (`docs/editor.md#ブレンドシェイプ`)。新しい親が動かない (ブレンドシェイプを持たない) 限り、
    // ブレンドシェイプを動かした姿でも見た目が変わらない。
    let model = fixtures::blend_shapes();
    let poses: &[&[(&str, f32)]] = &[
        REST,
        &[("Smile", 1.0)],
        &[("Wide", 1.0)],
        &[("Wide", 0.0), ("Smile", 0.5)],
        &[("X", 1.0), ("Smile", 1.0), ("Wide", 1.0)],
    ];
    // Mouth を包むワープ New (ブレンドシェイプを持たない)。
    let wrapped = applied(&model, warp(vec![mesh("Mouth")], 2, 2));
    assert_same_look(&model, &wrapped, poses);
    let cases = [
        (&model, warp(vec![def("Face")], 1, 1)),
        (&model, rotation(vec![def("Jaw")], None)),
        (&wrapped, reparent(def("Jaw"), Some("New"))),
        (&wrapped, reparent(mesh("Mouth"), None)),
        (&wrapped, unwrap("New")),
    ];
    for (before, command) in cases {
        let after = applied(before, command);
        assert_same_look(before, &after, poses);
    }
    let mouth = wrapped.meshes.iter().find(|m| m.id == "Mouth");
    assert_eq!(mouth.unwrap().keyforms.blend_shapes.len(), 2);

    // 回転の中心の移動は、ブレンドシェイプ格子のセルの原点にも同じだけ足す。
    let delta = Vec2::new(8.0, -4.0);
    let moved = applied(
        &model,
        Command::MoveRotationOrigin {
            id: "Jaw".into(),
            delta,
        },
    );
    let origins = |m: &Model| -> Vec<Vec2> {
        match &deformer(m, "Jaw").kind {
            DeformerKind::Rotation(r) => r.keyforms.all_keyforms().map(|k| k.origin).collect(),
            _ => panic!("Jaw is a rotation deformer"),
        }
    };
    for (a, b) in origins(&model).into_iter().zip(origins(&moved)) {
        assert!((a + delta).abs_diff_eq(b, TOL), "{a} + {delta} != {b}");
    }
    assert_same_look(&model, &moved, &[REST, &[("Smile", 1.0)]]);
}
