use std::sync::RwLock;

use super::{Node, Set};

/// A linked list-based set implemented with coarse-grained locking.
#[derive(Default)]
pub struct CoarseSet<T> {
    head: RwLock<Option<Node<T>>>,
}

impl<T> Set for CoarseSet<T>
where
    T: PartialOrd + PartialEq + Eq,
{
    type Elem = T;

    fn add(&self, elem: Self::Elem) -> bool {
        let mut head_guard = self.head.write().unwrap();

        if (*head_guard).is_none() {
            *head_guard = Some(Node::new_tail(elem));
            return true;
        }

        let head_val = (*head_guard).as_ref().unwrap().get();
        if *head_val == elem {
            false
        } else if *head_val > elem {
            let head = (*head_guard).take().unwrap();
            let new_head = Node::new_intermediate(elem, head);
            *head_guard = Some(new_head);
            true
        } else {
            let mut curr = (*head_guard).as_mut().unwrap();
            loop {
                // SAFETY: `curr.add()` would not invalidate the reference
                // returned by `curr.next_mut()`.
                let c = unsafe { &mut *(curr as *mut Node<T>) };
                match c.next_mut() {
                    Some(next) => {
                        let next_val = next.get();
                        if *next_val == elem {
                            return false;
                        } else if *next_val < elem {
                            curr = next;
                        } else {
                            curr.add(elem);
                            return true;
                        }
                    }
                    None => {
                        curr.add(elem);
                        return true;
                    }
                }
            }
        }
    }

    fn remove(&self, elem: &Self::Elem) -> bool {
        let mut head_guard = self.head.write().unwrap();

        if (*head_guard).is_none() {
            return false;
        }

        let head_val = (*head_guard).as_ref().unwrap().get();
        if head_val == elem {
            let (_, maybe_rest) = (*head_guard).take().unwrap().into_parts();
            if let Some(rest) = maybe_rest {
                *head_guard = Some(Box::into_inner(rest));
            }
            true
        } else if head_val > elem {
            false
        } else {
            let mut curr = (*head_guard).as_mut().unwrap();
            loop {
                // SAFETY: `curr.add()` would not invalidate the reference
                // returned by `curr.next_mut()`.
                let c = unsafe { &mut *(curr as *mut Node<T>) };
                match c.next_mut() {
                    Some(next) => {
                        let next_val = next.get();
                        if next_val == elem {
                            curr.set_next(next.take_next());
                            return true;
                        } else if next_val < elem {
                            curr = next;
                        } else {
                            return false;
                        }
                    }
                    None => {
                        return false;
                    }
                }
            }
        }
    }

    fn contains(&self, elem: &Self::Elem) -> bool {
        let head_guard = self.head.read().unwrap();
        match &*head_guard {
            None => false,
            Some(head) => {
                let head_val = head.get();
                if head_val == elem {
                    true
                } else if head_val > elem {
                    false
                } else {
                    let mut curr = head;
                    loop {
                        match curr.next() {
                            Some(next) => {
                                let next_val = next.get();
                                if next_val == elem {
                                    return true;
                                } else if next_val > elem {
                                    return false;
                                } else {
                                    curr = next;
                                }
                            }
                            None => return false,
                        }
                    }
                }
            }
        }
    }
}
