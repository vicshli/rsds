use std::{
    mem::MaybeUninit,
    sync::{LockResult, Mutex, MutexGuard},
};

use super::Set;

/// A linked-list-based concurrent set that implements optimistic concurrency
/// control.
pub struct OptimisticSet<T> {
    // the head node is a sentinel and never contains user-inserted value
    head: Node<T>,
}

impl<T> Set for OptimisticSet<T> {
    type Elem = T;

    fn add(&self, elem: Self::Elem) -> bool {
        if self.is_empty() {
            self.head.insert_after_self(elem);
            return true;
        }
        let prev = &self.head;
        let curr = &self.head.next();
        if curr.is_none() {
            // try insert by locking
        } else {
            // TOCTTOU race condition: while curr had value at the time of
            // check, it may have been deallocated in the mean time (by another
            // thread successfully removing `curr`).
            //
            // If we access `curr` to get its `next` ptr, we will cause a
            // segfault. To prevent premature deallocation, we should impl
            // ref-count somehow. But at that point, we are not that different
            // from using a RWlock.
            //
            // The main difference from RWlock is: RWlock blocks readers when
            // there's a writer, but we may make readers non-blocking in our
            // implementation.
        }

        todo!()
    }

    fn remove(&self, elem: &Self::Elem) -> bool {
        todo!()
    }

    fn contains(&self, elem: &Self::Elem) -> bool {
        if self.is_empty() {
            return false;
        }

        todo!()
    }
}

impl<T> OptimisticSet<T> {
    fn is_empty(&self) -> bool {
        !self.head.has_next()
    }
}

struct Node<T> {
    data: T,
    next: PeekableMutex<Box<PeekableOptional<Node<T>>>>,
}

impl<T: PartialEq> PartialEq for Node<T> {
    fn eq(&self, other: &Self) -> bool {
        self.data.eq(&other.data)
    }
}

impl<T: PartialOrd> PartialOrd for Node<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.data.partial_cmp(&other.data)
    }
}

impl<T: Default> Default for Node<T> {
    fn default() -> Self {
        Self {
            data: T::default(),
            next: PeekableMutex::new(Box::new(PeekableOptional::none())),
        }
    }
}

impl<T> Node<T> {
    fn new(elem: T) -> Self {
        Self {
            data: elem,
            next: PeekableMutex::new(Box::new(PeekableOptional::none())),
        }
    }

    fn next(&self) -> &PeekableOptional<Node<T>> {
        unsafe { self.next.unprotected_read() }
    }

    fn has_next(&self) -> bool {
        unsafe { self.next.unprotected_read().is_some() }
    }

    fn insert_after_self(&self, next: T) {
        let mut next_guard = self.next.lock().unwrap();
        let next_node = Node::new(next);
        next_guard.none_to_some(next_node);
    }
}

struct PeekableOptional<T> {
    has_value: bool,
    data: MaybeUninit<T>,
}

impl<T> PeekableOptional<T> {
    fn none() -> Self {
        Self {
            has_value: false,
            data: MaybeUninit::uninit(),
        }
    }

    fn some(data: T) -> Self {
        Self {
            has_value: true,
            data: MaybeUninit::new(data),
        }
    }

    fn is_some(&self) -> bool {
        self.has_value
    }

    fn is_none(&self) -> bool {
        !self.has_value
    }

    fn none_to_some(&mut self, val: T) {
        assert!(!self.has_value);
        self.data = MaybeUninit::new(val);
        self.has_value = true;
    }
}

struct PeekableMutex<T> {
    data: Mutex<T>,
    ptr: *const T,
}

impl<T> PeekableMutex<T> {
    pub fn new(data: T) -> Self {
        let data = Mutex::new(data);
        let ptr = {
            let guard = data.lock().unwrap();
            &*guard as *const T
        };
        Self { data, ptr }
    }

    pub unsafe fn unprotected_read(&self) -> &T {
        &*self.ptr
    }

    pub fn lock(&self) -> LockResult<MutexGuard<'_, T>> {
        self.data.lock()
    }
}
