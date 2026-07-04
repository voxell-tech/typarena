use alloc::boxed::Box;
use alloc::vec::Vec;
use core::any::TypeId;

use hashbrown::HashMap;
use hashbrown::hash_map::Entry;
use sparse_map::{Key, SparseMap};

use crate::ColumnId;
use crate::any_sparse_map::DynSparseMap;

/// Handle returned by [`TypePool::insert`].
///
/// Encodes both the column and the position within it. Pass it back
/// to [`TypePool::get`], [`TypePool::get_mut`], or
/// [`TypePool::remove`] to reach the value with no hash lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PoolKey {
    col: ColumnId,
    sparse_key: Key,
}

impl PoolKey {
    /// A sentinel key that will never refer to a live value.
    pub const PLACEHOLDER: Self = Self {
        col: ColumnId::PLACEHOLDER,
        sparse_key: Key::PLACEHOLDER,
    };

    /// Returns the [`ColumnId`] encoded in this key.
    pub fn col_id(self) -> ColumnId {
        self.col
    }
}

/// Heterogeneous append-only store.
///
/// Each call to [`TypePool::insert`] allocates a new slot and
/// returns a [`PoolKey`] encoding the column and position.
/// Subsequent lookups use the key directly with no hash-map
/// overhead.
///
/// ## Mental model
///
/// | [`PoolKey`] | `f32` col | `String` col |
/// |-------------|-----------|--------------|
/// | {col:0,..}  | 3.14      | -            |
/// | {col:1,..}  | -         | "hello"      |
/// | {col:0,..}  | 2.71      | -            |
pub struct TypePool {
    column_ids: HashMap<TypeId, ColumnId>,
    columns: Vec<DynSparseMap>,
}

impl TypePool {
    /// Creates an empty [`TypePool`].
    pub fn new() -> Self {
        Self {
            column_ids: HashMap::new(),
            columns: Vec::new(),
        }
    }

    pub fn ensure_column<T: 'static>(&mut self) -> ColumnId {
        match self.column_ids.entry(TypeId::of::<T>()) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(e) => {
                let col = ColumnId::new(self.columns.len());
                e.insert(col);
                self.columns.push(Box::new(SparseMap::<T>::new()));
                col
            }
        }
    }

    pub fn contains(&self, key: &PoolKey) -> bool {
        self.columns
            .get(key.col.index())
            .map(|c| c.dyn_contains(&key.sparse_key))
            .unwrap_or_default()
    }

    /// Inserts `value` and returns a [`PoolKey`] identifying it.
    pub fn insert<T: 'static>(&mut self, value: T) -> PoolKey {
        self.insert_with_key(
            #[inline(always)]
            |_| value,
        )
    }

    /// Like [`Self::insert`], but calls `create` with the
    /// [`PoolKey`] before the value is placed in storage.
    pub fn insert_with_key<T: 'static>(
        &mut self,
        create: impl FnOnce(PoolKey) -> T,
    ) -> PoolKey {
        let col = self.ensure_column::<T>();
        // SAFETY: `col` was just assigned for T by ensure_column.
        let column = unsafe {
            self.columns[col.index()].downcast_unchecked_mut::<T>()
        };
        let key = column.insert_with_key(|_, key| {
            create(PoolKey {
                col,
                sparse_key: key,
            })
        });
        PoolKey {
            col,
            sparse_key: key,
        }
    }

    /// Returns a reference to the value at `key`, or `None`
    /// if it has been removed.
    pub fn get<T: 'static>(&self, key: &PoolKey) -> Option<&T> {
        self.columns
            .get(key.col.index())?
            .downcast_ref::<T>()?
            .get(&key.sparse_key)
    }

    /// Returns a mutable reference to the value at `key`,
    /// or `None` if it has been removed.
    pub fn get_mut<T: 'static>(
        &mut self,
        key: &PoolKey,
    ) -> Option<&mut T> {
        self.columns
            .get_mut(key.col.index())?
            .downcast_mut::<T>()?
            .get_mut(&key.sparse_key)
    }

    /// Removes and returns the value at `key`, or `None` if
    /// already removed.
    pub fn remove<T: 'static>(&mut self, key: &PoolKey) -> Option<T> {
        self.columns
            .get_mut(key.col.index())?
            .downcast_mut::<T>()?
            .remove(&key.sparse_key)
    }

    /// Provides `(&mut T, &mut TypePool)` simultaneously.
    ///
    /// The value at `key` is temporarily taken out of its slot
    /// for the duration of `f`, then restored to the same slot.
    /// The [`PoolKey`] remains valid after the call.
    ///
    /// Returns `None` if `key` is absent.
    pub fn scope<T: 'static, R>(
        &mut self,
        key: &PoolKey,
        f: impl FnOnce(&mut T, &mut Self) -> R,
    ) -> Option<R> {
        let mut value = {
            self.columns
                .get_mut(key.col.index())?
                .downcast_mut::<T>()?
                .take(&key.sparse_key)?
        };
        let result = f(&mut value, self);
        if let Some(col) = self
            .columns
            .get_mut(key.col.index())
            .and_then(|c| c.downcast_mut::<T>())
        {
            col.restore(&key.sparse_key, value);
        }
        Some(result)
    }

    /// Removes the value at `key` without knowing its type.
    ///
    /// Returns `true` if an entry was present and removed.
    pub fn dyn_remove(&mut self, key: &PoolKey) -> bool {
        match self.columns.get_mut(key.col.index()) {
            Some(col) => col.dyn_remove(&key.sparse_key),
            None => false,
        }
    }

    /// Returns the [`TypeId`] of the values held in `key`'s
    /// column, or `None` if the column does not exist.
    pub fn value_type_id(&self, key: &PoolKey) -> Option<TypeId> {
        self.columns.get(key.col.index()).map(|c| c.type_id_of())
    }

    /// Returns the number of values stored in the column for `T`.
    pub fn len<T: 'static>(&self) -> usize {
        self.column::<T>().map(SparseMap::len).unwrap_or_default()
    }

    /// Returns `true` if the column for `T` holds no values.
    pub fn is_empty<T: 'static>(&self) -> bool {
        self.len::<T>() == 0
    }

    /// Iterates shared references to every value in the column for
    /// `T`.
    ///
    /// Yields nothing if no column for `T` has been created yet.
    pub fn iter<'a, T: 'static>(
        &'a self,
    ) -> impl Iterator<Item = &'a T> + 'a {
        self.column::<T>().into_iter().flat_map(SparseMap::iter)
    }

    /// Removes and yields every value in the column for `T`, leaving
    /// that column empty.
    pub fn drain<T: 'static>(
        &mut self,
    ) -> impl Iterator<Item = T> + '_ {
        self.column_mut::<T>()
            .into_iter()
            .flat_map(SparseMap::drain)
    }

    /// Removes every value in the column for `T`, leaving it empty.
    ///
    /// Returns `true` if a column for `T` exists, or `false` if one
    /// has not been created yet.
    pub fn clear<T: 'static>(&mut self) -> bool {
        if let Some(column) = self.column_mut::<T>() {
            column.clear();
            return true;
        }
        false
    }

    /// Returns the [`SparseMap`] column for `T`, if one exists.
    fn column<T: 'static>(&self) -> Option<&SparseMap<T>> {
        self.column_ids
            .get(&TypeId::of::<T>())
            .and_then(|col| self.columns.get(col.index()))
            .and_then(|c| c.downcast_ref::<T>())
    }

    /// Returns a mutable [`SparseMap`] column for `T`, if one exists.
    fn column_mut<T: 'static>(
        &mut self,
    ) -> Option<&mut SparseMap<T>> {
        let col = *self.column_ids.get(&TypeId::of::<T>())?;
        self.columns
            .get_mut(col.index())
            .and_then(|c| c.downcast_mut::<T>())
    }
}

impl Default for TypePool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use super::{PoolKey, TypePool};

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct Velocity(f32);

    #[derive(Debug, PartialEq, Clone)]
    struct Name(String);

    #[test]
    fn insert_and_get() {
        let mut pool = TypePool::new();
        let key = pool.insert(Velocity(10.5));
        assert_eq!(pool.get::<Velocity>(&key), Some(&Velocity(10.5)));
    }

    #[test]
    fn heterogeneous_storage() {
        let mut pool = TypePool::new();
        let kv = pool.insert(Velocity(20.0));
        let kn = pool.insert(Name(String::from("Entity_1")));
        // Each type occupies a different column.
        assert_ne!(kv.col_id(), kn.col_id());
        assert_eq!(pool.get::<Velocity>(&kv), Some(&Velocity(20.0)));
        assert_eq!(
            pool.get::<Name>(&kn),
            Some(&Name(String::from("Entity_1")))
        );
    }

    #[test]
    fn type_isolation() {
        let mut pool = TypePool::new();
        let key = pool.insert(50u64);
        assert!(pool.get::<u32>(&key).is_none());
        assert!(pool.get::<i64>(&key).is_none());
        assert_eq!(pool.get::<u64>(&key), Some(&50u64));
    }

    #[test]
    fn multiple_values_same_type() {
        let mut pool = TypePool::new();
        let k1 = pool.insert(Velocity(1.0));
        let k2 = pool.insert(Velocity(2.0));
        assert_eq!(pool.get::<Velocity>(&k1), Some(&Velocity(1.0)));
        assert_eq!(pool.get::<Velocity>(&k2), Some(&Velocity(2.0)));
    }

    #[test]
    fn get_mut_modifies_value() {
        let mut pool = TypePool::new();
        let key = pool.insert(Velocity(0.0));
        pool.get_mut::<Velocity>(&key).unwrap().0 = 9.9;
        assert_eq!(pool.get::<Velocity>(&key), Some(&Velocity(9.9)));
    }

    #[test]
    fn remove_returns_value() {
        let mut pool = TypePool::new();
        let key = pool.insert(Velocity(5.0));
        assert_eq!(
            pool.remove::<Velocity>(&key),
            Some(Velocity(5.0))
        );
        assert!(pool.get::<Velocity>(&key).is_none());
        assert!(pool.remove::<Velocity>(&key).is_none());
    }

    #[test]
    fn dyn_remove_present() {
        let mut pool = TypePool::new();
        let key = pool.insert(Velocity(1.0));
        assert!(pool.dyn_remove(&key));
        assert!(pool.get::<Velocity>(&key).is_none());
    }

    #[test]
    fn dyn_remove_absent() {
        let mut pool = TypePool::new();
        let key = pool.insert(Velocity(1.0));
        pool.dyn_remove(&key);
        assert!(!pool.dyn_remove(&key));
    }

    #[test]
    fn placeholder_key_is_absent() {
        let mut pool = TypePool::new();
        assert!(!pool.dyn_remove(&PoolKey::PLACEHOLDER));
    }

    #[test]
    fn clear_empties_column() {
        let mut pool = TypePool::new();
        let key = pool.insert(Velocity(1.0));
        pool.insert(Velocity(2.0));

        assert!(pool.clear::<Velocity>());
        assert_eq!(pool.len::<Velocity>(), 0);
        assert!(pool.get::<Velocity>(&key).is_none());
    }

    #[test]
    fn clear_absent_column_returns_false() {
        let mut pool = TypePool::new();
        assert!(!pool.clear::<Velocity>());
    }

    #[test]
    fn insert_with_key_receives_correct_key() {
        let mut pool = TypePool::new();
        let mut seen = None;
        let returned = pool.insert_with_key(|k| {
            seen = Some(k);
            Velocity(1.0)
        });
        assert_eq!(seen, Some(returned));
    }

    #[test]
    fn ensure_column_is_idempotent() {
        let mut pool = TypePool::new();
        let c1 = pool.ensure_column::<Velocity>();
        let c2 = pool.ensure_column::<Velocity>();
        assert_eq!(c1, c2);
    }

    #[test]
    fn different_types_get_different_columns() {
        let mut pool = TypePool::new();
        let cv = pool.ensure_column::<Velocity>();
        let cn = pool.ensure_column::<Name>();
        assert_ne!(cv, cn);
    }

    #[test]
    fn scope_temporarily_removes_value() {
        let mut pool = TypePool::new();
        let key = pool.insert(Velocity(5.0));
        let result = pool.scope::<Velocity, _>(&key, |val, inner| {
            // Value is absent during the closure.
            assert!(inner.get::<Velocity>(&key).is_none());
            val.0 * 2.0
        });
        assert_eq!(result, Some(10.0));
        // Value is restored after scope.
        assert_eq!(pool.get::<Velocity>(&key), Some(&Velocity(5.0)));
    }

    #[test]
    fn scope_restores_mutation() {
        let mut pool = TypePool::new();
        let key = pool.insert(Velocity(5.0));
        pool.scope::<Velocity, _>(&key, |val, _| {
            val.0 = 99.0;
        });
        assert_eq!(pool.get::<Velocity>(&key), Some(&Velocity(99.0)));
    }

    #[test]
    fn scope_absent_key_returns_none() {
        let mut pool = TypePool::new();
        let key = pool.insert(Velocity(1.0));
        pool.remove::<Velocity>(&key);
        assert!(pool.scope::<Velocity, _>(&key, |_, _| ()).is_none());
    }

    #[test]
    fn scope_can_insert_into_pool() {
        let mut pool = TypePool::new();
        let key = pool.insert(Velocity(1.0));
        pool.scope::<Velocity, _>(&key, |_, inner| {
            inner.insert(Name(String::from("side value")));
        });
        assert_eq!(pool.get::<Velocity>(&key), Some(&Velocity(1.0)));
    }
}
