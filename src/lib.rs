#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

mod any_index_map;
mod any_sparse_map;
pub mod id;
pub mod type_pool;
pub mod type_table;

pub(crate) use maybe_send::MaybeSend;
pub(crate) use maybe_sync::MaybeSync;

/// Bound applied to every stored value type.
///
/// With the `send` feature enabled it requires [`Send`], so the dyn
/// columns and the stores holding them are [`Send`] too. Without the
/// feature it is a no-op that every type satisfies.
#[cfg(feature = "send")]
mod maybe_send {
    pub trait MaybeSend: Send {}

    impl<T: Send + ?Sized> MaybeSend for T {}
}

#[cfg(not(feature = "send"))]
mod maybe_send {
    pub trait MaybeSend {}

    impl<T: ?Sized> MaybeSend for T {}
}

/// Bound applied to every stored value type.
///
/// With the `sync` feature enabled it requires [`Sync`], so the dyn
/// columns and the stores holding them are [`Sync`] too. Without the
/// feature it is a no-op that every type satisfies.
#[cfg(feature = "sync")]
mod maybe_sync {
    pub trait MaybeSync: Sync {}

    impl<T: Sync + ?Sized> MaybeSync for T {}
}

#[cfg(not(feature = "sync"))]
mod maybe_sync {
    pub trait MaybeSync {}

    impl<T: ?Sized> MaybeSync for T {}
}

/// Index of a per-type column, shared by [`TypePool`] and
/// [`TypeTable`].
///
/// Stable for the lifetime of its store: a type's column keeps the
/// same id once created, so callers can cache the id to skip the
/// [`TypeId`](core::any::TypeId) hash lookup on later access.
///
/// [`TypePool`]: type_pool::TypePool
/// [`TypeTable`]: type_table::TypeTable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColumnId(usize);

impl ColumnId {
    /// A sentinel id that will never refer to a live column.
    pub const PLACEHOLDER: Self = Self(usize::MAX);

    /// Creates a column id from its index in the store's column
    /// vector.
    pub(crate) fn new(index: usize) -> Self {
        Self(index)
    }

    pub fn index(self) -> usize {
        self.0
    }
}

#[cfg(all(test, feature = "send"))]
mod send_tests {
    use crate::type_pool::TypePool;
    use crate::type_table::TypeTable;

    fn assert_send<T: Send>() {}

    #[test]
    fn stores_are_send() {
        assert_send::<TypePool>();
        assert_send::<TypeTable<u32>>();
    }
}

#[cfg(all(test, feature = "sync"))]
mod sync_tests {
    use crate::type_pool::TypePool;
    use crate::type_table::TypeTable;

    fn assert_sync<T: Sync>() {}

    #[test]
    fn stores_are_sync() {
        assert_sync::<TypePool>();
        assert_sync::<TypeTable<u32>>();
    }
}
