//! Generational IDs, independent of any value storage.

use alloc::vec::Vec;
use core::cmp::Ordering;
use core::fmt::{Debug, Display, Formatter, Result};
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;

/// A generational ID with a phantom type parameter `T`.
///
/// The `generation` counter is bumped each time a raw `id` slot is
/// recycled, preventing stale handles from accidentally matching a
/// newly allocated ID (ABA problem). The phantom `T` makes IDs for
/// different domains incompatible at the type level, even if their
/// raw `id`/`generation` values collide.
pub struct GenId<T> {
    id: u32,
    generation: u32,
    _marker: PhantomData<T>,
}

impl<T> GenId<T> {
    pub const PLACEHOLDER: Self = Self::from_raw(u32::MAX, u32::MAX);

    const fn from_raw(id: u32, generation: u32) -> Self {
        Self {
            id,
            generation,
            _marker: PhantomData,
        }
    }

    const fn next_generation(mut self) -> Self {
        self.generation = self.generation.wrapping_add(1);
        self
    }
}

impl<T> Display for GenId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}v{}", self.id, self.generation)
    }
}

impl<T> Debug for GenId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("GenId")
            .field("id", &self.id)
            .field("generation", &self.generation)
            .finish()
    }
}

impl<T> Hash for GenId<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.generation.hash(state);
        self._marker.hash(state);
    }
}

impl<T> Eq for GenId<T> {}

impl<T> PartialEq for GenId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.generation == other.generation
            && self._marker == other._marker
    }
}

impl<T> Ord for GenId<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id
            .cmp(&other.id)
            .then(self.generation.cmp(&other.generation))
            .then(self._marker.cmp(&other._marker))
    }
}

impl<T> PartialOrd for GenId<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Copy for GenId<T> {}

impl<T> Clone for GenId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

/// Allocates and recycles [`GenId<T>`] values.
///
/// Recycled IDs are returned with a bumped generation so they are
/// never equal to any previously issued ID for the same raw slot.
pub struct IdGenerator<T> {
    next_id: u32,
    unused_ids: Vec<GenId<T>>,
}

impl<T> IdGenerator<T> {
    pub const fn new() -> Self {
        Self {
            next_id: 0,
            unused_ids: Vec::new(),
        }
    }

    /// Returns a recycled ID (generation bumped) if one is available,
    /// otherwise allocates a fresh one.
    pub fn new_id(&mut self) -> GenId<T> {
        self.unused_ids
            .pop()
            .map(|id| id.next_generation())
            .unwrap_or_else(|| {
                assert!(
                    self.next_id < u32::MAX,
                    "`IdGenerator` ran out of ids (u32::MAX limit \
                     reached)"
                );
                let id = GenId::from_raw(self.next_id, 0);
                self.next_id += 1;
                id
            })
    }

    /// Returns `id` to the pool for future reuse.
    pub fn recycle(&mut self, id: GenId<T>) {
        self.unused_ids.push(id);
    }
}

impl<T> Default for IdGenerator<T> {
    fn default() -> Self {
        Self::new()
    }
}
