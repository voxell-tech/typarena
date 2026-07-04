#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

mod any_index_map;
mod any_sparse_map;
pub mod id;
pub mod type_pool;
pub mod type_table;

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
