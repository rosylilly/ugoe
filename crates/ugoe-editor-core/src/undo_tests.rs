//! Undo の履歴の上限と、適用前の状態の検証の結果の記憶のテスト (`docs/editor.md#undo`)。

use ugoe_model::fixtures;

use crate::document::{DEFAULT_UNDO_LIMIT, checks};
use crate::{Command, Document, EditError};

/// `X` の既定値を `default` にするコマンド (状態ごとに違うモデルを作るため)。
fn set_default(default: f32) -> Command {
    Command::SetParameterRange {
        id: "X".into(),
        min: -1.0,
        default,
        max: 1.0,
    }
}

fn default_of(doc: &Document) -> f32 {
    doc.model().parameters[0].default
}

/// `doc` で戻れるだけ Undo し、戻った回数を返す。
fn undo_all(doc: &mut Document) -> usize {
    let mut n = 0;
    while doc.undo() {
        n += 1;
    }
    n
}

#[test]
fn the_default_limit_is_used() {
    let doc = Document::new(fixtures::single_quad());
    assert_eq!(doc.undo_limit(), DEFAULT_UNDO_LIMIT);
    assert_eq!(doc.detached().undo_limit(), DEFAULT_UNDO_LIMIT);
    let doc = doc.with_undo_limit(3);
    assert_eq!(doc.detached().undo_limit(), 3);
}

#[test]
fn the_oldest_states_are_dropped_over_the_limit() {
    let mut doc = Document::new(fixtures::single_quad()).with_undo_limit(3);
    for i in 1..=5 {
        doc.apply(set_default(i as f32 * 0.1)).unwrap();
    }
    assert_eq!(undo_all(&mut doc), 3);
    // 0.0 (作った状態) と 0.1 は捨てた。
    assert!((default_of(&doc) - 0.2).abs() < 1e-6);
    // Redo は捨てていない分をすべてやり直せる。
    let mut redone = 0;
    while doc.redo() {
        redone += 1;
    }
    assert_eq!(redone, 3);
    assert!((default_of(&doc) - 0.5).abs() < 1e-6);
}

#[test]
fn is_modified_stays_true_when_the_saved_state_is_dropped() {
    let mut doc = Document::new(fixtures::single_quad()).with_undo_limit(2);
    assert!(!doc.is_modified());
    for i in 1..=3 {
        doc.apply(set_default(i as f32 * 0.1)).unwrap();
    }
    assert!(doc.is_modified());
    // 保存した状態 (作った状態) にはもう戻れないので、戻れるだけ戻っても変更ありのまま。
    assert_eq!(undo_all(&mut doc), 2);
    assert!(doc.is_modified());
    while doc.redo() {
        assert!(doc.is_modified());
    }
}

#[test]
fn is_modified_follows_a_saved_state_still_in_the_history() {
    let mut doc = Document::new(fixtures::single_quad()).with_undo_limit(2);
    for i in 1..=4 {
        doc.apply(set_default(i as f32 * 0.1)).unwrap();
    }
    doc.mark_saved();
    // 上限を超えて積んでも、保存した状態に Undo で戻れば変更なしになる。
    doc.apply(set_default(0.5)).unwrap();
    doc.apply(set_default(0.6)).unwrap();
    assert!(doc.is_modified());
    assert!(doc.undo());
    assert!(doc.undo());
    assert!(!doc.is_modified());
    assert!(!doc.undo());
    assert!(doc.redo());
    assert!(doc.is_modified());
}

#[test]
fn lowering_the_limit_drops_old_undo_first_then_far_redo() {
    let mut doc = Document::new(fixtures::single_quad());
    for i in 1..=4 {
        doc.apply(set_default(i as f32 * 0.1)).unwrap();
    }
    // 0.0 0.1 [0.2] 0.3 0.4 (今は 0.2)
    assert!(doc.undo());
    assert!(doc.undo());
    doc.set_undo_limit(3);
    // Undo を 1 つ (0.0) 捨てた。
    assert_eq!(undo_all(&mut doc), 1);
    assert!((default_of(&doc) - 0.1).abs() < 1e-6);
    // 0.1 [今] から Redo が 3 つ: 上限 1 にすると、近い Redo を 1 つだけ残す。
    doc.set_undo_limit(1);
    assert!(doc.redo());
    assert!((default_of(&doc) - 0.2).abs() < 1e-6);
    assert!(!doc.redo());
    assert!(doc.undo());
    assert!(!doc.undo());
}

#[test]
fn a_zero_limit_keeps_no_history() {
    let mut doc = Document::new(fixtures::single_quad()).with_undo_limit(0);
    doc.apply(set_default(0.1)).unwrap();
    assert!(doc.is_modified());
    assert!(!doc.undo());
    assert!((default_of(&doc) - 0.1).abs() < 1e-6);
}

#[test]
fn a_state_is_validated_once() {
    let mut doc = Document::new(fixtures::single_quad());
    let start = checks();
    // 作った状態は最初の適用で 1 度、適用後の状態はその適用で 1 度ずつ検証する。
    for i in 1..=3 {
        doc.apply(set_default(i as f32 * 0.1)).unwrap();
    }
    assert_eq!(checks() - start, 4);
    // Undo で戻った状態は検証の結果を覚えている。
    assert!(doc.undo());
    doc.apply(set_default(0.9)).unwrap();
    assert_eq!(checks() - start, 5);
    // 拒否しても、適用前の状態を検証し直さない。
    let mut broken = fixtures::single_quad().parameters.remove(0);
    broken.id = "Y".into();
    (broken.min, broken.max) = (1.0, -1.0);
    for _ in 0..2 {
        let err = doc
            .apply(Command::AddParameter {
                parameter: broken.clone(),
            })
            .unwrap_err();
        assert!(matches!(err, EditError::Invalid(_)), "{err:?}");
    }
    assert_eq!(checks() - start, 7);
}
