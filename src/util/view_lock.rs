use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::util::DynIter;
use crate::util::Id;
use crate::util::MapAccess;

pub trait ViewLock<'a, T, C>
where
    C: DynIter<'a, Item = Arc<RwLock<T>>> + MapAccess<Arc<RwLock<T>>>
{
    fn lock_view(&'a self) -> LockedView<'a, T, C>;
    fn lock_view_mut(&'a self) -> LockedViewMut<'a, T, C>;
}

pub struct LockedView<'a, T, C>
where
    C: DynIter<'a, Item = Arc<RwLock<T>>> + MapAccess<Arc<RwLock<T>>>
{
    locked_container: RwLockReadGuard<'a, C>,
}

impl<'a, T, C> LockedView<'a, T, C>
where
    C: DynIter<'a, Item = Arc<RwLock<T>>> + MapAccess<Arc<RwLock<T>>>
{
    pub fn new(locked_container: RwLockReadGuard<'a, C>) -> Self {
        Self {
            locked_container,
        }
    }

    pub fn get(&'a self, id: Id) -> Result<impl Deref<Target = T> + 'a, String> {
        Ok(LockedReference::new(self.locked_container.get(id)?))
    }

    pub fn iter(&'a self) -> impl Iterator<Item = impl Deref<Target = T> + 'a> + 'a {
        LockedViewIter::new(self.locked_container.dyn_iter())
    }
}

pub struct LockedViewMut<'a, T, C>
where
    C: DynIter<'a, Item = Arc<RwLock<T>>> + MapAccess<Arc<RwLock<T>>>
{
    locked_container: RwLockWriteGuard<'a, C>,
}

impl<'a, T, C> LockedViewMut<'a, T, C>
where
    C: DynIter<'a, Item = Arc<RwLock<T>>> + MapAccess<Arc<RwLock<T>>>
{
    pub fn new(locked_container: RwLockWriteGuard<'a, C>) -> Self {
        Self {
            locked_container,
        }
    }

    pub fn get(&'a self, id: Id) -> Result<impl Deref<Target = T> + 'a, String> {
        Ok(LockedReference::new(self.locked_container.get(id)?))
    }

    pub fn get_mut(&'a self, id: Id) -> Result<impl DerefMut<Target = T> + 'a, String> {
        Ok(LockedReferenceMut::new(self.locked_container.get(id)?))
    }

    pub fn iter(&'a self) -> impl Iterator<Item = impl Deref<Target = T> + 'a> + 'a {
        LockedViewIter::new(self.locked_container.dyn_iter())
    }

    pub fn iter_mut(&'a self) -> impl Iterator<Item = impl DerefMut<Target = T> + 'a> + 'a {
        LockedViewIterMut::new(self.locked_container.dyn_iter())
    }
}

pub struct LockedReference<'a, T> {
    _reference: &'a Arc<RwLock<T>>,
    guard: RwLockReadGuard<'a, T>,
}

impl<'a, T> LockedReference<'a, T> {
    fn new(reference: &'a Arc<RwLock<T>>) -> Self {
        Self {
            _reference: reference,
            guard: reference.read().unwrap(),
        }
    }
}

impl<T> Deref for LockedReference<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.guard
    }
}

pub struct LockedReferenceMut<'a, T> {
    _reference: &'a Arc<RwLock<T>>,
    guard: RwLockWriteGuard<'a, T>,
}

impl<'a, T> LockedReferenceMut<'a, T> {
    fn new(reference: &'a Arc<RwLock<T>>) -> Self {
        Self {
            _reference: reference,
            guard: reference.write().unwrap(),
        }
    }
}

impl<T> Deref for LockedReferenceMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.guard
    }
}

impl<T> DerefMut for LockedReferenceMut<'_, T> {
    fn deref_mut(& mut self) -> &mut T {
        &mut *self.guard
    }
}

pub struct LockedViewIter<'a, T> {
    iter: Box<dyn Iterator<Item = &'a Arc<RwLock<T>>> + 'a>,
}

impl<'a, T> LockedViewIter<'a, T> {
    fn new(iter: Box<dyn Iterator<Item = &'a Arc<RwLock<T>>> + 'a>) -> Self {
        Self {
            iter,
        }
    }
}

impl<'a, T> Iterator for LockedViewIter<'a, T> {
    type Item = LockedReference<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let next_item = self.iter.next();
        next_item.map(|x| LockedReference::new(x))
    }
}

pub struct LockedViewIterMut<'a, T> {
    iter: Box<dyn Iterator<Item = &'a Arc<RwLock<T>>> + 'a>,
}

impl<'a, T> LockedViewIterMut<'a, T> {
    fn new(iter: Box<dyn Iterator<Item = &'a Arc<RwLock<T>>> + 'a>) -> Self {
        Self {
            iter,
        }
    }
}

impl<'a, T> Iterator for LockedViewIterMut<'a, T> {
    type Item = LockedReferenceMut<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let next_item = self.iter.next();
        next_item.map(|x| LockedReferenceMut::new(x))
    }
}