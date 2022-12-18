use std::sync::{Mutex, MutexGuard};

use super::{NodeRepr, Set};

pub struct FineGrainedSet<T> {
    head: Node<T>,
}

impl<T> Default for FineGrainedSet<T> {
    fn default() -> Self {
        Self {
            head: Node::new_head(),
        }
    }
}

impl<T> Set for FineGrainedSet<T>
where
    T: PartialOrd + PartialEq + Eq,
{
    type Elem = T;

    fn add(&self, elem: Self::Elem) -> bool {
        let mut head_ref = self.head.locked();
        if head_ref.is_empty() {
            head_ref.set_value_on_empty_head(elem);
            return true;
        }

        let mut curr = head_ref;
        loop {
            let curr_elem = curr.elem().unwrap();
            if *curr_elem == elem {
                // found existing, do not insert
                return false;
            } else if *curr_elem > elem {
                // insert elem before `curr`
                curr.replace_existing(|rest| LockedNode::new_intermediate(elem, rest));
                return true;
            } else if !curr.has_next() {
                // insert elem after `curr`
                curr.replace_existing(|node| {
                    let (curr, _) = node.into_parts();
                    LockedNode::new_intermediate(curr, LockedNode::new_tail(elem))
                });
                return true;
            } else {
                curr = curr.into_next().expect("next node should exist");
            }
        }
    }

    fn remove(&self, elem: &Self::Elem) -> bool {
        let mut head_ref = self.head.locked();
        if head_ref.is_empty() {
            return false;
        }

        let head_elem = head_ref.elem().unwrap();
        if head_elem == elem {
            // move the next node's content to the head node.

            let next = head_ref.next().map(|n| {
                // note: unlocks the next node here, taking ownership of its content
                n.into_parts()
                    .expect("sentinel node should only be at the front")
            });
            match next {
                Some((elem, rest)) => {
                    head_ref.replace_existing(move |_| match rest {
                        Some(rest) => LockedNode::new_intermediate(elem, rest),
                        None => LockedNode::new_tail(elem),
                    });
                }
                None => {
                    head_ref.clear();
                }
            }
            return true;
        } else if head_elem > elem {
            return false;
        }

        // Otherwise, search for deletion in the rest of the list
        let mut curr = head_ref;
        loop {
            {
                let next = {
                    let next = curr.next();
                    let Some(next) = next else {
                        return false;
                    };

                    let next_elem = next.elem();
                    let Some(next_elem) = next_elem else {
                        return false;
                    };

                    if next_elem > elem {
                        return false;
                    }
                    next
                };

                let next_elem = next.elem().unwrap();
                if next_elem == elem {
                    let next_of_next = next.into_next();
                    match next_of_next {
                        Some(rest) => curr.replace_existing(|n| {
                            let elem = n.into_elem();
                            let parts = rest.into_parts().unwrap();
                            LockedNode::new_intermediate(elem, LockedNode::from_parts(parts))
                        }),
                        None => curr.replace_existing(|n| LockedNode::new_tail(n.into_elem())),
                    }
                    return true;
                }
            }
            // current node is smaller than the target, advance to the next node
            curr = curr.into_next().expect("next node should exist");
        }
    }

    fn contains(&self, elem: &Self::Elem) -> bool {
        let head_ref = self.head.locked();
        if head_ref.is_empty() {
            return false;
        }

        let mut curr_ref = Some(head_ref);
        while let Some(curr) = curr_ref {
            let curr_elem = curr.elem().unwrap();
            if curr_elem == elem {
                return true;
            } else if curr_elem > elem {
                return false;
            } else {
                curr_ref = curr.into_next();
            }
        }

        false
    }
}

struct LockedNodeRef<'a, T>(MutexGuard<'a, Option<LockedNode<T>>>);

impl<'a, T> LockedNodeRef<'a, T> {
    fn is_empty(&self) -> bool {
        (*self.0).is_none()
    }

    fn elem(&self) -> Option<&T> {
        (&*self.0).as_ref().map(|node| node.elem())
    }

    fn replace_existing<F>(&mut self, replace_fn: F)
    where
        F: FnOnce(LockedNode<T>) -> LockedNode<T>,
    {
        let curr = (*self.0)
            .take()
            .expect("the API should only be called on non-sentinel nodes");

        let next = replace_fn(curr);
        *self.0 = Some(next);
    }

    fn set_value_on_empty_head(&mut self, elem: T) {
        *self.0 = Some(LockedNode::new_tail(elem));
    }

    fn clear(&mut self) {
        *self.0 = None;
    }

    fn into_parts(mut self) -> Option<(T, Option<Box<Node<T>>>)> {
        self.0.take().map(|n| n.into_parts())
    }

    fn has_next(&self) -> bool {
        self.0.as_ref().map(|n| n.has_next()).unwrap_or(false)
    }

    fn next(&self) -> Option<LockedNodeRef<'_, T>> {
        if self.is_empty() {
            return None;
        }
        let curr = (*self.0).as_ref().unwrap();
        curr.next()
    }

    fn into_next<'n>(self) -> Option<LockedNodeRef<'n, T>> {
        // hand-over-hand locking:
        // The current node is locked now (because this struct contains its lock
        // guard). Below we acquire the lock guard of the next node, if it exists.
        // When this function returns, the destructor of this struct will be called,
        // at which point this node's lock is released.
        let next = self.next();

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

impl<'a, T> From<MutexGuard<'a, Option<LockedNode<T>>>> for LockedNodeRef<'a, T> {
    fn from(guard: MutexGuard<'a, Option<LockedNode<T>>>) -> Self {
        LockedNodeRef(guard)
    }
}

struct LockedNode<T> {
    inner: NodeRepr<T, Node<T>>,
}

impl<T> LockedNode<T> {
    fn new_tail(elem: T) -> Self {
        Self {
            inner: NodeRepr::Tail(elem),
        }
    }

    fn new_intermediate<R>(elem: T, rest: R) -> Self
    where
        R: Into<Box<Node<T>>>,
    {
        Self {
            inner: NodeRepr::Elem((elem, rest.into())),
        }
    }

    fn from_parts(parts: (T, Option<Box<Node<T>>>)) -> Self {
        let (elem, maybe_rest) = parts;
        let inner = match maybe_rest {
            Some(rest) => NodeRepr::Elem((elem, rest)),
            None => NodeRepr::Tail(elem),
        };
        Self { inner }
    }

    fn elem(&self) -> &T {
        self.inner.elem()
    }

    fn has_next(&self) -> bool {
        matches!(self.inner, NodeRepr::Elem(_))
    }

    fn next(&self) -> Option<LockedNodeRef<'_, T>> {
        match &self.inner {
            NodeRepr::Elem((_, rest)) => Some(rest.as_ref().locked()),
            NodeRepr::Tail(_) => None,
        }
    }

    fn into_parts(self) -> (T, Option<Box<Node<T>>>) {
        self.inner.into_parts()
    }

    fn into_elem(self) -> T {
        self.inner.into_parts().0
    }
}

#[allow(clippy::from_over_into)]
impl<T> Into<Box<Node<T>>> for LockedNode<T> {
    fn into(self) -> Box<Node<T>> {
        Box::new(self.into())
    }
}

struct Node<T> {
    node: Mutex<Option<LockedNode<T>>>,
}

impl<T> Node<T> {
    fn new_head() -> Self {
        Self {
            node: Mutex::new(None),
        }
    }

    fn locked(&self) -> LockedNodeRef<T> {
        self.node.lock().unwrap().into()
    }
}

impl<T> From<LockedNode<T>> for Node<T> {
    fn from(inner: LockedNode<T>) -> Self {
        Self {
            node: Mutex::new(Some(inner)),
        }
    }
}
