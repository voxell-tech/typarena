use alloc::boxed::Box;
use core::any::TypeId;

use sparse_map::{Key, SparseMap};

use crate::{MaybeSend, MaybeSync};

pub(crate) type DynSparseMap = Box<dyn AnySparseMap>;

trait Seal {}
impl<T: 'static> Seal for SparseMap<T> {}

#[expect(private_bounds)]
pub trait AnySparseMap: Seal + MaybeSend + MaybeSync {
    /// Returns the [`TypeId`] of the stored value type.
    fn value_type_id(&self) -> TypeId;

    /// Removes the value at `key`.
    ///
    /// Returns `true` if an entry was present and removed.
    fn dyn_remove(&mut self, key: &Key) -> bool;

    /// Returns `true` if an entry was present.
    fn dyn_contains(&self, key: &Key) -> bool;
}

impl<T: 'static> AnySparseMap for SparseMap<T>
where
    Self: MaybeSend + MaybeSync,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn dyn_remove(&mut self, key: &Key) -> bool {
        self.remove(key).is_some()
    }

    fn dyn_contains(&self, key: &Key) -> bool {
        self.contains(key)
    }
}

impl dyn AnySparseMap {
    #[inline]
    pub fn type_id_of(&self) -> TypeId {
        self.value_type_id()
    }

    #[inline]
    pub fn element_is<T: 'static>(&self) -> bool {
        self.value_type_id() == TypeId::of::<T>()
    }

    #[inline]
    pub fn downcast_ref<T: 'static>(&self) -> Option<&SparseMap<T>> {
        if self.element_is::<T>() {
            unsafe { Some(self.downcast_unchecked_ref()) }
        } else {
            None
        }
    }

    #[inline]
    pub fn downcast_mut<T: 'static>(
        &mut self,
    ) -> Option<&mut SparseMap<T>> {
        if self.element_is::<T>() {
            unsafe { Some(self.downcast_unchecked_mut()) }
        } else {
            None
        }
    }

    /// # Safety
    ///
    /// Calling this with the wrong type is *undefined behavior*.
    #[inline]
    pub unsafe fn downcast_unchecked_ref<T: 'static>(
        &self,
    ) -> &SparseMap<T> {
        debug_assert!(self.element_is::<T>());
        unsafe { &*(self as *const Self as *const SparseMap<T>) }
    }

    /// # Safety
    ///
    /// Calling this with the wrong type is *undefined behavior*.
    #[inline]
    pub unsafe fn downcast_unchecked_mut<T: 'static>(
        &mut self,
    ) -> &mut SparseMap<T> {
        debug_assert!(self.element_is::<T>());
        unsafe { &mut *(self as *mut Self as *mut SparseMap<T>) }
    }
}
