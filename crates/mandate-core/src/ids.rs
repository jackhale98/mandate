//! Typed `u32` identifiers with monotonic allocators (DEC-041 rule 6).
//! IDs are never reused within a world.

use serde::{Deserialize, Serialize};

/// Define a typed ID newtype over `u32`.
#[macro_export]
macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug,
            serde::Serialize, serde::Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub u32);

        impl $name {
            /// Index into a dense `Vec` keyed by this ID.
            #[inline]
            pub fn index(self) -> usize {
                self.0 as usize
            }
        }

        impl From<u32> for $name {
            fn from(v: u32) -> Self {
                Self(v)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}#{}", stringify!($name), self.0)
            }
        }
    };
}

/// Monotonic allocator. Persisted in saves so IDs stay unique across sessions.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdAllocator {
    next: u32,
}

impl IdAllocator {
    pub fn new() -> Self {
        Self { next: 0 }
    }

    /// Start allocation after an existing number of IDs.
    pub fn starting_at(next: u32) -> Self {
        Self { next }
    }

    /// Allocate the next raw ID. Panics only on `u32` exhaustion, which is an
    /// unrecoverable modelling error (≥ 4 billion entities of one type).
    #[allow(clippy::expect_used)]
    pub fn alloc_raw(&mut self) -> u32 {
        let id = self.next;
        self.next = self.next.checked_add(1).expect("ID space exhausted");
        id
    }

    pub fn alloc<T: From<u32>>(&mut self) -> T {
        T::from(self.alloc_raw())
    }

    pub fn peek_next(&self) -> u32 {
        self.next
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    define_id!(TestId);

    #[test]
    fn allocation_is_monotonic_and_unique() {
        let mut a = IdAllocator::new();
        let ids: Vec<TestId> = (0..100).map(|_| a.alloc()).collect();
        for w in ids.windows(2) {
            assert!(w[0] < w[1]);
        }
        assert_eq!(a.peek_next(), 100);
        assert_eq!(ids[5].index(), 5);
    }
}
