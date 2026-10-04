//! Undo / Redo の履歴と、状態ごとの検証の結果 (`docs/editor.md#undo`)。

use std::collections::VecDeque;

use ugoe_model::Model;
use ugoe_model::physics::PhysicsSettings;
use ugoe_model::validate::{ValidationError, validate, validate_physics};

/// [`crate::Document`] が覚える Undo の数の既定の上限。超えると古いものから捨てる。
pub const DEFAULT_UNDO_LIMIT: usize = 100;

/// Undo と Redo の積み (`T` は状態)。Undo と Redo を合わせて `limit` 個までしか持たない。
#[derive(Debug)]
pub(super) struct History<T> {
    /// 古いものが先頭。
    undo: VecDeque<T>,
    /// 次に Redo するものが末尾。
    redo: Vec<T>,
    limit: usize,
}

impl<T> History<T> {
    pub(super) fn new(limit: usize) -> Self {
        Self {
            undo: VecDeque::new(),
            redo: Vec::new(),
            limit,
        }
    }

    pub(super) fn limit(&self) -> usize {
        self.limit
    }

    /// 戻れる数。
    pub(super) fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// 進める数。
    pub(super) fn redo_len(&self) -> usize {
        self.redo.len()
    }

    /// 上限を変え、超える分を捨てる (古い Undo から。Undo が尽きたら遠い Redo から)。
    pub(super) fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
        self.trim();
    }

    /// 新しい状態に移ったので、前の状態 `prev` を Undo に積み、Redo を捨てる。
    pub(super) fn record(&mut self, prev: T) {
        self.redo.clear();
        self.undo.push_back(prev);
        self.trim();
    }

    /// `current` を 1 つ前の状態と入れ替える。戻れなければ `false`。
    pub(super) fn undo(&mut self, current: &mut T) -> bool {
        match self.undo.pop_back() {
            Some(prev) => {
                self.redo.push(std::mem::replace(current, prev));
                true
            }
            None => false,
        }
    }

    /// `current` を 1 つ後の状態と入れ替える。進めなければ `false`。
    pub(super) fn redo(&mut self, current: &mut T) -> bool {
        match self.redo.pop() {
            Some(next) => {
                self.undo.push_back(std::mem::replace(current, next));
                true
            }
            None => false,
        }
    }

    pub(super) fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    fn trim(&mut self) {
        while self.undo.len() + self.redo.len() > self.limit {
            if self.undo.pop_front().is_none() {
                self.redo.remove(0);
            }
        }
    }
}

/// ある状態の検証の結果。適用前の状態として比べるときに検証し直さないよう、状態ごとに覚える。
#[derive(Debug, Clone, Default)]
pub(super) struct Problems {
    /// `validate` の結果。
    pub(super) model: Vec<ValidationError>,
    /// `validate_physics` の結果 (物理の設定がなければ空)。
    pub(super) physics: Vec<ValidationError>,
}

impl Problems {
    pub(super) fn of(model: &Model, physics: Option<&PhysicsSettings>) -> Self {
        #[cfg(test)]
        CHECKS.with(|c| c.set(c.get() + 1));
        Self {
            model: validate(model),
            physics: physics
                .map(|p| validate_physics(p, model))
                .unwrap_or_default(),
        }
    }
}

#[cfg(test)]
thread_local! {
    static CHECKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// このスレッドで [`Problems::of`] を呼んだ回数 (検証を重ねていないかのテスト用)。
#[cfg(test)]
pub(crate) fn checks() -> usize {
    CHECKS.with(std::cell::Cell::get)
}
