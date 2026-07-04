use alloc::boxed::Box;
use core::any::TypeId;
use core::hash::{BuildHasher, Hash};

use indexmap::IndexMap;

pub(crate) type DynIndexMap<K, S> = Box<dyn AnyIndexMap<K, S>>;

trait Seal {}
impl<K, V: 'static, S> Seal for IndexMap<K, V, S> {}

#[expect(private_bounds)]
pub trait AnyIndexMap<K, S>: Seal {
    /// Returns the [`TypeId`] of the stored value type.
    fn value_type_id(&self) -> TypeId;

    /// Removes the value at `key` by swapping it with the last entry.
    ///
    /// This is [`IndexMap::swap_remove`]: it is *O(1)* but perturbs
    /// the position of whatever used to be the last entry.
    ///
    /// Returns `true` if an entry was present and removed.
    fn dyn_swap_remove(&mut self, key: &K) -> bool;

    /// Returns `true` if an entry is present for `key`.
    fn dyn_contains(&self, key: &K) -> bool;
}

impl<K, V, S> AnyIndexMap<K, S> for IndexMap<K, V, S>
where
    K: Eq + Hash + 'static,
    V: 'static,
    S: BuildHasher,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<V>()
    }

    fn dyn_swap_remove(&mut self, key: &K) -> bool {
        self.swap_remove(key).is_some()
    }

    fn dyn_contains(&self, key: &K) -> bool {
        self.contains_key(key)
    }
}

impl<K, S> dyn AnyIndexMap<K, S> {
    #[inline]
    pub fn value_is<V: 'static>(&self) -> bool {
        self.value_type_id() == TypeId::of::<V>()
    }

    #[inline]
    pub fn downcast_ref<V: 'static>(
        &self,
    ) -> Option<&IndexMap<K, V, S>> {
        if self.value_is::<V>() {
            unsafe { Some(self.downcast_unchecked_ref()) }
        } else {
            None
        }
    }

    #[inline]
    pub fn downcast_mut<V: 'static>(
        &mut self,
    ) -> Option<&mut IndexMap<K, V, S>> {
        if self.value_is::<V>() {
            unsafe { Some(self.downcast_unchecked_mut()) }
        } else {
            None
        }
    }

    /// # Safety
    ///
    /// Calling this with the wrong type is *undefined behavior*.
    #[inline]
    pub unsafe fn downcast_unchecked_ref<V: 'static>(
        &self,
    ) -> &IndexMap<K, V, S> {
        debug_assert!(self.value_is::<V>());
        let ptr = self as *const Self as *const IndexMap<K, V, S>;
        unsafe { &*ptr }
    }

    /// # Safety
    ///
    /// Calling this with the wrong type is *undefined behavior*.
    #[inline]
    pub unsafe fn downcast_unchecked_mut<V: 'static>(
        &mut self,
    ) -> &mut IndexMap<K, V, S> {
        debug_assert!(self.value_is::<V>());
        let ptr = self as *mut Self as *mut IndexMap<K, V, S>;
        unsafe { &mut *ptr }
    }
}
