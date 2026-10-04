//! Undo の履歴の上限・続けての確定のまとめ・適用前の状態の検証の結果の記憶のテスト (`docs/editor.md#undo`)。

use std::time::{Duration, Instant};

use ugoe_model::fixtures;

use crate::document::{DEFAULT_UNDO_LIMIT, checks};
use crate::{COALESCE_WINDOW, Coalesce, Command, Document, EditError};

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

/// `doc` の (戻れる数, 進める数) と、`can_undo`・`can_redo` がそれと合うこと。
fn counts(doc: &Document) -> (usize, usize) {
    assert_eq!(doc.can_undo(), doc.undo_count() > 0);
    assert_eq!(doc.can_redo(), doc.redo_count() > 0);
    (doc.undo_count(), doc.redo_count())
}

#[test]
fn can_undo_and_can_redo_follow_the_history() {
    let mut doc = Document::new(fixtures::single_quad()).with_undo_limit(3);
    assert_eq!(counts(&doc), (0, 0));
    assert!(!doc.can_undo() && !doc.can_redo());
    doc.apply(set_default(0.1)).unwrap();
    doc.apply(set_default(0.2)).unwrap();
    assert_eq!(counts(&doc), (2, 0));
    assert!(doc.undo());
    assert_eq!(counts(&doc), (1, 1));
    assert!(doc.redo());
    assert_eq!(counts(&doc), (2, 0));
    // 拒否された編集は履歴を変えない。
    assert!(
        doc.apply(Command::SetParameterRange {
            id: "missing".into(),
            min: 0.0,
            default: 0.0,
            max: 1.0,
        })
        .is_err()
    );
    assert_eq!(counts(&doc), (2, 0));
    // 上限を超えると最も古いものを捨てる。
    doc.apply(set_default(0.3)).unwrap();
    doc.apply(set_default(0.4)).unwrap();
    assert_eq!(counts(&doc), (3, 0));
    assert!(doc.undo() && doc.undo() && doc.undo());
    assert_eq!(counts(&doc), (0, 3));
    assert!(!doc.can_undo() && doc.can_redo());
    // Undo の後の編集は Redo を捨てる。
    doc.apply(set_default(0.5)).unwrap();
    assert_eq!(counts(&doc), (1, 0));
    // 上限を下げると数も減る。
    doc.set_undo_limit(0);
    assert_eq!(counts(&doc), (0, 0));
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

/// `start` から `ms` ミリ秒後に、欄 `key` で確定した印。
fn field(key: &str, start: Instant, ms: u64) -> Coalesce {
    Coalesce::new(key, start + Duration::from_millis(ms))
}

#[test]
fn holding_an_arrow_key_makes_one_undo() {
    let mut doc = Document::new(fixtures::single_quad());
    let start = Instant::now();
    // 矢印キーの押し続け (30 ms ごとに 50 回。全体では COALESCE_WINDOW より長い)。
    for i in 1..=50 {
        doc.apply_coalescing(set_default(i as f32 * 0.01), field("x", start, i * 30))
            .unwrap();
    }
    assert!((default_of(&doc) - 0.5).abs() < 1e-6);
    assert_eq!(doc.undo_count(), 1);
    assert!(doc.undo());
    assert_eq!(default_of(&doc), 0.0);
    assert!(!doc.is_modified());
    // Redo はまとめた最後の値へ進む。
    assert!(doc.redo());
    assert!((default_of(&doc) - 0.5).abs() < 1e-6);
}

#[test]
fn a_pause_longer_than_the_window_starts_a_new_undo() {
    let mut doc = Document::new(fixtures::single_quad());
    let start = Instant::now();
    let window = COALESCE_WINDOW.as_millis() as u64;
    doc.apply_coalescing(set_default(0.1), field("x", start, 0))
        .unwrap();
    doc.apply_coalescing(set_default(0.2), field("x", start, window))
        .unwrap();
    assert_eq!(doc.undo_count(), 1);
    doc.apply_coalescing(set_default(0.3), field("x", start, 2 * window + 1))
        .unwrap();
    assert_eq!(doc.undo_count(), 2);
    assert!(doc.undo());
    assert!((default_of(&doc) - 0.2).abs() < 1e-6);
}

#[test]
fn another_field_or_edit_breaks_the_group() {
    let mut doc = Document::new(fixtures::single_quad());
    let start = Instant::now();
    doc.apply_coalescing(set_default(0.1), field("x", start, 0))
        .unwrap();
    // 別の欄はまとめない。
    doc.apply_coalescing(set_default(0.2), field("y", start, 10))
        .unwrap();
    assert_eq!(doc.undo_count(), 2);
    // まとめない適用を挟むと、同じ欄でもまとめない。
    doc.apply(set_default(0.3)).unwrap();
    doc.apply_coalescing(set_default(0.4), field("y", start, 20))
        .unwrap();
    assert_eq!(doc.undo_count(), 4);
    // 時刻が戻った印 (別の時計) はまとめない。
    doc.apply_coalescing(set_default(0.5), Coalesce::new("y", start))
        .unwrap();
    assert_eq!(doc.undo_count(), 5);
}

#[test]
fn undo_redo_and_save_break_the_group() {
    let mut doc = Document::new(fixtures::single_quad());
    let start = Instant::now();
    doc.apply_coalescing(set_default(0.1), field("x", start, 0))
        .unwrap();
    doc.apply_coalescing(set_default(0.2), field("x", start, 10))
        .unwrap();
    // Undo と Redo で同じ状態に戻っても、続けてはまとめない。
    assert!(doc.undo());
    assert!(doc.redo());
    doc.apply_coalescing(set_default(0.3), field("x", start, 20))
        .unwrap();
    assert_eq!(doc.undo_count(), 2);
    // 保存した状態には Undo で戻れる。
    doc.mark_saved();
    doc.apply_coalescing(set_default(0.4), field("x", start, 30))
        .unwrap();
    assert_eq!(doc.undo_count(), 3);
    assert!(doc.undo());
    assert!(!doc.is_modified());
}

#[test]
fn a_rejected_edit_keeps_the_group() {
    let mut doc = Document::new(fixtures::single_quad());
    let start = Instant::now();
    doc.apply_coalescing(set_default(0.1), field("x", start, 0))
        .unwrap();
    let err = doc
        .apply_coalescing(
            Command::SetParameterRange {
                id: "missing".into(),
                min: -1.0,
                default: 0.0,
                max: 1.0,
            },
            field("x", start, 10),
        )
        .unwrap_err();
    assert!(matches!(err, EditError::NotFound { .. }), "{err:?}");
    doc.apply_coalescing(set_default(0.2), field("x", start, 20))
        .unwrap();
    assert_eq!(doc.undo_count(), 1);
}
