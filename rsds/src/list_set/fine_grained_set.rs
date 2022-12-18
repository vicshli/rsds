use std::sync::{Mutex, MutexGuard};

use super::{NodeInner, Set};

pub struct FineGrainedSet<T> {
    head: LockedNode<T>,
}

impl<T> FineGrainedSet<T> {
    pub fn new() -> Self {
        Self {
            head: LockedNode::new_head(),
        }
    }
}

impl<T> Set for FineGrainedSet<T>
where
    T: PartialOrd + PartialEq + Eq,
{
    type Elem = T;

    fn add(&self, elem: Self::Elem) -> bool {
        let mut head_ref = self.head.curr();
        if head_ref.is_empty() {
            head_ref.set_value_on_empty_head(elem);
            return true;
        }

        let mut curr_ref = Some(head_ref);
        while let Some(mut curr) = curr_ref {
            let curr_elem = curr.elem().unwrap();
            if *curr_elem == elem {
                return false;
            } else if *curr_elem > elem {
                curr.replace_existing(|rest| LockedNodeInner::new_intermediate(elem, rest));
                return true;
            } else {
                curr_ref = curr.next();
            }
        }

        false
    }

    fn remove(&self, elem: &Self::Elem) -> bool {
        todo!()
    }

    fn contains(&self, elem: &Self::Elem) -> bool {
        todo!()
    }
}

struct LockedNodeRef<'a, T>(MutexGuard<'a, Option<LockedNodeInner<T>>>);

impl<'a, T> LockedNodeRef<'a, T> {
    fn is_empty(&self) -> bool {
        (*self.0).is_none()
    }

    fn elem(&self) -> Option<&T> {
        (&*self.0).as_ref().map(|node| node.elem())
    }

    fn replace_existing<F>(&mut self, replace_fn: F)
    where
        F: FnOnce(LockedNodeInner<T>) -> LockedNodeInner<T>,
    {
        let curr = (*self.0)
            .take()
            .expect("the API should only be called on non-sentinel nodes");

        let next = replace_fn(curr);
        *self.0 = Some(next);
    }

    fn set_value_on_empty_head(&mut self, elem: T) {
        *self.0 = Some(LockedNodeInner::new_tail(elem));
    }

    fn next<'n>(self) -> Option<LockedNodeRef<'n, T>> {
        if self.is_empty() {
            return None;
        }
        let curr = (*self.0).as_ref().unwrap();

        // hand-over-hand locking:
        // The current node is locked now (because this struct contains its lock
        // guard). Below we acquire the lock guard of the next node, if it exists.
        // When this function returns, the destructor of this struct will be called,
        // at which point this node's lock is released.
        let next = curr.next();

        // Extend the lifetime of the returned value to the caller's requirements.
        //
        // SAFETY:
        // `next` currently has exclusive access to the next node, which means the
        // caller is the only party that can delete it.
        //
        // Care should be taken to ensure whoever holds `curr`'s lock in the future
        // doesn't delete its successor without first obtaining the next node's lock.
        //
        // `next`'s original lifetime was tied to `self` because we could only
        // obtain the next node from the current node's reference to its successor.
        unsafe { std::mem::transmute(next) }
    }
}

impl<'a, T> From<MutexGuard<'a, Option<LockedNodeInner<T>>>> for LockedNodeRef<'a, T> {
    fn from(guard: MutexGuard<'a, Option<LockedNodeInner<T>>>) -> Self {
        LockedNodeRef(guard)
    }
}

struct LockedNodeInner<T> {
    inner: NodeInner<T, LockedNode<T>>,
}

impl<T> LockedNodeInner<T> {
    fn new_tail(elem: T) -> Self {
        Self {
            inner: NodeInner::Tail(elem),
        }
    }

    fn new_intermediate(elem: T, rest: LockedNodeInner<T>) -> Self {
        Self {
            inner: NodeInner::Elem((
                elem,
                Box::new(LockedNode {
                    node: Mutex::new(Some(rest)),
                }),
            )),
        }
    }

    fn elem(&self) -> &T {
        self.inner.elem()
    }

    fn next(&self) -> Option<LockedNodeRef<'_, T>> {
        match &self.inner {
            NodeInner::Elem((_, rest)) => Some(rest.as_ref().curr()),
            NodeInner::Tail(_) => None,
        }
    }
}

struct LockedNode<T> {
    node: Mutex<Option<LockedNodeInner<T>>>,
}

impl<T> LockedNode<T> {
    fn new_head() -> Self {
        Self {
            node: Mutex::new(None),
        }
    }

    fn curr(&self) -> LockedNodeRef<T> {
        self.node.lock().unwrap().into()
    }
}
