//! What the values behind a loop variable or a piped stream can look like, as far as flags go.
//!
//! The locus bindings beside this one say WHERE such a value points. They cannot say whether the
//! value is a flag: `for f in -delete` and `echo -delete | xargs` both bind something that reads
//! as a worktree path while the command receives `-delete`.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::BitOr;

/// A small set of yes/no facts, one bit each. `T` names what they are facts about, so a bit that
/// means one thing in one set cannot be tested in another.
#[derive(Debug, PartialEq, Eq)]
pub struct Facts<T>(u8, PhantomData<T>);

impl<T> Clone for Facts<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Facts<T> {}

impl<T> Default for Facts<T> {
    fn default() -> Self {
        Self::NONE
    }
}

impl<T> Facts<T> {
    pub const NONE: Self = Self(0, PhantomData);

    #[must_use]
    pub const fn bit(n: u8) -> Self {
        Self(1 << n, PhantomData)
    }

    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0, PhantomData)
    }

    pub fn has(self, fact: Self) -> bool {
        self.0 & fact.0 != 0
    }

    pub fn set(&mut self, fact: Self, on: bool) {
        if on {
            self.0 |= fact.0;
        }
    }

    pub fn clear(&mut self, fact: Self) {
        self.0 &= !fact.0;
    }

    #[must_use]
    pub fn when(fact: Self, on: bool) -> Self {
        if on { fact } else { Self::NONE }
    }
}

impl<T> BitOr for Facts<T> {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        self.union(other)
    }
}

/// Marks the facts about a set of values a variable or stream may take.
#[derive(Debug, PartialEq, Eq)]
pub enum Items {}

pub type ItemShape = Facts<Items>;

/// Some value may begin with `-`.
pub const LEAD: ItemShape = Facts::bit(0);
/// Some value may hold a blank, so an unquoted use becomes several words.
pub const SPLIT: ItemShape = Facts::bit(1);
/// Some value may be empty, so an unquoted use becomes none.
pub const EMPTY: ItemShape = Facts::bit(2);
/// Nothing is known about the values.
pub const UNKNOWN: ItemShape = LEAD.union(SPLIT).union(EMPTY);

thread_local! {
    static LOOP_SHAPES: RefCell<Vec<(String, ItemShape)>> = const { RefCell::new(Vec::new()) };
    static STDIN_SHAPES: RefCell<Vec<ItemShape>> = const { RefCell::new(Vec::new()) };
}

pub struct ItemShapeGuard(bool);

impl Drop for ItemShapeGuard {
    fn drop(&mut self) {
        if self.0 {
            LOOP_SHAPES.with(|v| v.borrow_mut().pop());
        } else {
            STDIN_SHAPES.with(|v| v.borrow_mut().pop());
        }
    }
}

#[must_use]
pub fn enter_loop_shape(name: String, shape: ItemShape) -> ItemShapeGuard {
    LOOP_SHAPES.with(|v| v.borrow_mut().push((name, shape)));
    ItemShapeGuard(true)
}

/// The innermost shape bound to `name`; [`UNKNOWN`] when none is.
pub fn loop_shape(name: &str) -> ItemShape {
    LOOP_SHAPES.with(|v| v.borrow().iter().rev().find(|(n, _)| n == name).map_or(UNKNOWN, |(_, s)| *s))
}

#[must_use]
pub fn enter_stdin_shape(shape: ItemShape) -> ItemShapeGuard {
    STDIN_SHAPES.with(|v| v.borrow_mut().push(shape));
    ItemShapeGuard(false)
}

/// The shape of the items on stdin; [`UNKNOWN`] when no stage set one.
pub fn stdin_shape() -> ItemShape {
    STDIN_SHAPES.with(|v| v.borrow().last().copied().unwrap_or(UNKNOWN))
}

/// What `$name` is bound to, by [`super::expand_vars`]' precedence: a loop (unknown items) or a
/// certain value.
pub enum Binding {
    Loop,
    Value(String),
    Unbound,
}

pub fn binding(name: &str) -> Binding {
    if super::LOOP_VARS.with(|v| v.borrow().iter().any(|l| l.name == name)) {
        return Binding::Loop;
    }
    super::VARS.with(|v| {
        v.borrow()
            .iter()
            .rev()
            .find(|b| b.name == name)
            .map_or(Binding::Unbound, |b| Binding::Value(b.value.clone()))
    })
}

/// Every name a loop or an assignment has bound so far.
pub fn bound_names() -> Vec<String> {
    let loops: Vec<String> = super::LOOP_VARS.with(|v| v.borrow().iter().map(|l| l.name.clone()).collect());
    super::VARS.with(|v| loops.into_iter().chain(v.borrow().iter().map(|b| b.name.clone())).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unbound_shapes_are_unknown_and_guards_unwind() {
        let safe = ItemShape::NONE;
        assert_eq!(loop_shape("f"), UNKNOWN);
        assert_eq!(stdin_shape(), UNKNOWN);
        {
            let _outer = enter_loop_shape("f".into(), safe);
            let _stdin = enter_stdin_shape(safe);
            assert_eq!(loop_shape("f"), safe);
            assert_eq!(loop_shape("g"), UNKNOWN);
            assert_eq!(stdin_shape(), safe);
            {
                let _inner = enter_loop_shape("f".into(), UNKNOWN);
                assert_eq!(loop_shape("f"), UNKNOWN);
            }
            assert_eq!(loop_shape("f"), safe);
        }
        assert_eq!(loop_shape("f"), UNKNOWN);
        assert_eq!(stdin_shape(), UNKNOWN);
    }

    #[test]
    fn facts_combine_and_clear() {
        let mut f = ItemShape::NONE;
        assert!(!f.has(LEAD));
        f.set(LEAD, false);
        assert_eq!(f, ItemShape::NONE);
        f.set(LEAD, true);
        assert!(f.has(LEAD) && !f.has(SPLIT));
        assert_eq!(LEAD | SPLIT | EMPTY, UNKNOWN);
        f.clear(LEAD);
        assert_eq!(f, ItemShape::NONE);
        assert_eq!(ItemShape::when(SPLIT, true), SPLIT);
        assert_eq!(ItemShape::when(SPLIT, false), ItemShape::NONE);
    }
}
