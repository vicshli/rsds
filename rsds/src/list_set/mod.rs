//! A module implementing set as linked lists.

use std::mem::MaybeUninit;

mod coarse_set;
mod fine_grained_set;

pub use coarse_set::CoarseSet;
pub use fine_grained_set::FineGrainedSet;

/// Defines common behavior for a set.
pub trait Set {
    /// Type of element contained in a set.
    type Elem;

    /// Attempts to add an element to the set.
    ///
    /// Returns `true` if the element is successfully added, or `false` if the
    /// element already exists in the set.
    fn add(&self, elem: Self::Elem) -> bool;

    /// Attempts to remove an element from the set.
    ///
    /// Returns `true` if the element is found and removed, or `false` if the
    /// element could not be found.
    fn remove(&self, elem: &Self::Elem) -> bool;

    /// Searches an element in the set, returning whether it is found.
    fn contains(&self, elem: &Self::Elem) -> bool;
}

enum NodeRepr<T, N> {
    Elem((T, Box<N>)),
    Tail(T),
}

impl<T, N> NodeRepr<T, N> {
    fn into_elem(self) -> T {
        match self {
            NodeRepr::Elem((e, _)) => e,
            NodeRepr::Tail(e) => e,
        }
    }

    fn into_parts(self) -> (T, Option<Box<N>>) {
        match self {
            NodeRepr::Elem((elem, rest)) => (elem, Some(rest)),
            NodeRepr::Tail(elem) => (elem, None),
        }
    }
}

impl<T> From<NodeRepr<T, Node<T>>> for Node<T> {
    fn from(inner: NodeRepr<T, Node<T>>) -> Self {
        Self {
            node: MaybeUninit::new(inner),
        }
    }
}

impl<T, N> NodeRepr<T, N> {
    fn elem(&self) -> &T {
        match self {
            NodeRepr::Elem((e, _)) => e,
            NodeRepr::Tail(e) => e,
        }
    }
}

struct Node<T> {
    node: MaybeUninit<NodeRepr<T, Node<T>>>,
}

impl<T> Node<T> {
    pub fn new_tail(elem: T) -> Self {
        NodeRepr::Tail(elem).into()
    }

    pub fn new_intermediate(elem: T, rest: Node<T>) -> Self {
        NodeRepr::Elem((elem, Box::new(rest))).into()
    }

    fn get(&self) -> &T {
        self.get_node_ref().elem()
    }

    fn next(&self) -> Option<&Self> {
        let node = self.get_node_ref();
        match node {
            NodeRepr::Tail(_) => None,
            NodeRepr::Elem((_, rest)) => Some(rest.as_ref()),
        }
    }

    fn next_mut(&mut self) -> Option<&mut Self> {
        let node = self.get_node_mut();
        match node {
            NodeRepr::Tail(_) => None,
            NodeRepr::Elem((_, rest)) => Some(rest.as_mut()),
        }
    }

    /// Transforms a Node into a Tail, returning the rest of the list if exists.
    fn take_next(&mut self) -> Option<Box<Node<T>>> {
        let node = self.get_node_mut();
        match node {
            NodeRepr::Tail(_) => None,
            NodeRepr::Elem(_) => {
                self.replace_node_with_ret(|node| {
                    // Downgrade Elem to Tail, returning the rest of the list
                    match node {
                        NodeRepr::Elem((elem, rest)) => {
                            let new_node = NodeRepr::Tail(elem);
                            (new_node, Some(rest))
                        }
                        _ => unreachable!(),
                    }
                })
            }
        }
    }

    fn set_next(&mut self, new_next: Option<Box<Node<T>>>) {
        self.replace_node_with(move |node| match new_next {
            Some(rest) => NodeRepr::Elem((node.into_elem(), rest)),
            None => NodeRepr::Tail(node.into_elem()),
        });
    }

    fn add(&mut self, elem: T) {
        self.replace_node_with(|node| match node {
            NodeRepr::Tail(curr) => NodeRepr::Elem((curr, Box::new(NodeRepr::Tail(elem).into()))),
            NodeRepr::Elem((curr, rest)) => {
                let next = NodeRepr::Elem((elem, rest));
                NodeRepr::Elem((curr, Box::new(next.into())))
            }
        })
    }

    fn into_parts(self) -> (T, Option<Box<Node<T>>>) {
        // SAFETY: we guarantee node to be initialized between method invocations.
        let node = unsafe { self.node.assume_init() };
        node.into_parts()
    }

    fn replace_node_with<F>(&mut self, node_replacer: F)
    where
        F: FnOnce(NodeRepr<T, Node<T>>) -> NodeRepr<T, Node<T>>,
    {
        // SAFETY: we guarantee node to be initialized between method invocations.
        let old_node =
            unsafe { std::mem::replace(&mut self.node, MaybeUninit::uninit()).assume_init() };
        let new_node = node_replacer(old_node);
        self.node.write(new_node);
    }

    fn replace_node_with_ret<F, Ret>(&mut self, node_replacer: F) -> Ret
    where
        F: FnOnce(NodeRepr<T, Node<T>>) -> (NodeRepr<T, Node<T>>, Ret),
    {
        // SAFETY: we guarantee node to be initialized between method invocations.
        let old_node =
            unsafe { std::mem::replace(&mut self.node, MaybeUninit::uninit()).assume_init() };
        let (new_node, ret) = node_replacer(old_node);
        self.node.write(new_node);
        ret
    }

    fn get_node_mut(&mut self) -> &mut NodeRepr<T, Node<T>> {
        // SAFETY: we guarantee node to be initialized between method invocations.
        unsafe { self.node.assume_init_mut() }
    }

    fn get_node_ref(&self) -> &NodeRepr<T, Node<T>> {
        // SAFETY: we guarantee node to be initialized between method invocations.
        unsafe { self.node.assume_init_ref() }
    }
}

/// Linked list iterator.
pub struct ListIter<'a, T> {
    curr: Option<&'a Node<T>>,
}

impl<'a, T> Iterator for ListIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(curr) = self.curr {
            let next = curr.get();
            self.curr = curr.next();
            return Some(next);
        }
        None
    }
}

struct ListInner<T> {
    head: Option<Node<T>>,
    tail: Option<*mut Node<T>>,
    len: usize,
}

impl<T> Default for ListInner<T> {
    fn default() -> Self {
        Self {
            head: None,
            tail: None,
            len: 0,
        }
    }
}

impl<T> ListInner<T> {
    pub fn add(&mut self, elem: T) {
        if self.head.is_none() {
            self.head = Some(Node::new_tail(elem));
            self.tail = Some(self.head.as_mut().unwrap());
        } else {
            // SAFETY: `tail` is guaranteed to be pointing to the list tail
            // and is guaranteed to be alive.
            let old_tail = unsafe { &mut *self.tail.unwrap() };
            old_tail.add(elem);
            let new_tail: *mut Node<T> = old_tail.next_mut().unwrap();
            self.tail = Some(new_tail);
        }
        self.len += 1;
    }

    pub fn add_ordered(&mut self, elem: T)
    where
        T: PartialOrd + PartialEq + Eq,
    {
        if self.head.is_none() {
            self.head = Some(Node::new_tail(elem));
            self.tail = Some(self.head.as_mut().unwrap());
        } else {
            let mut curr = self.head.as_mut().unwrap();
            loop {
                // SAFTETY: allow forming two mutable borrows (one in
                // `c.next_mut()`, and another in `curr.add(...)`).
                //
                // This is Ok because `curr.add(...)` would not invalidate the
                // reference returned by `c.next_mut()`. Plus, if `curr.add(..)`
                // were invoked, the return value of `c.next_mut()` isn't used.
                let c = unsafe { &mut *(curr as *mut Node<T>) };
                match c.next_mut() {
                    Some(next) => {
                        if *next.get() > elem {
                            curr.add(elem);
                            break;
                        } else {
                            curr = next;
                        }
                    }
                    None => {
                        curr.add(elem);
                        break;
                    }
                }
            }
        }
        self.len += 1;
    }

    pub fn find(&self, target: &T) -> bool
    where
        T: PartialEq + Eq,
    {
        let Some(mut curr) = self.head.as_ref() else {
            return false;
        };

        loop {
            let curr_elem = curr.get();
            if curr_elem == target {
                return true;
            } else {
                match curr.next() {
                    Some(next) => curr = next,
                    None => return false,
                }
            }
        }
    }

    pub fn find_ordered(&self, target: &T) -> bool
    where
        T: PartialOrd + PartialEq + Eq,
    {
        let Some(mut curr) = self.head.as_ref() else {
            return false;
        };

        loop {
            let curr_elem = curr.get();
            if curr_elem > target {
                return false;
            } else if curr_elem == target {
                return true;
            } else {
                match curr.next() {
                    Some(next) => curr = next,
                    None => return false,
                }
            }
        }
    }

    pub fn iter(&self) -> ListIter<'_, T> {
        match self.head {
            Some(ref h) => ListIter { curr: Some(h) },
            None => ListIter { curr: None },
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A linked list.
#[derive(Default)]
pub struct List<T> {
    inner: ListInner<T>,
}

impl<T> List<T>
where
    T: PartialEq + Eq,
{
    /// Appends an element to the end of the linked list.
    pub fn add(&mut self, elem: T) {
        self.inner.add(elem)
    }

    /// Checks whether the given element is part of the linked list.
    pub fn find(&self, target: &T) -> bool {
        self.inner.find(target)
    }

    /// Returns the linked list's iterator.
    pub fn iter(&self) -> ListIter<'_, T> {
        self.inner.iter()
    }

    /// Returns the number of elements contained in this linked list.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Checks whether the linked list is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

/// A sorted linked list.
#[derive(Default)]
pub struct OrderedList<T> {
    inner: ListInner<T>,
}

impl<T> OrderedList<T>
where
    T: PartialOrd + PartialEq + Eq,
{
    /// Appends an element to the end of the linked list.
    pub fn add(&mut self, elem: T) {
        self.inner.add_ordered(elem)
    }

    /// Checks whether the given element is part of the linked list.
    pub fn find(&self, target: &T) -> bool {
        self.inner.find_ordered(target)
    }

    /// Returns the linked list's iterator.
    pub fn iter(&self) -> ListIter<'_, T> {
        self.inner.iter()
    }

    /// Returns the number of elements contained in this linked list.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Checks whether the linked list is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use quickcheck_macros::quickcheck;

    #[test]
    fn linked_list() {
        let len = 5_000_000;
        let mut list = List::default();
        assert!(list.is_empty());
        for i in 0..len {
            list.add(i);
        }
        assert!(list.len() == len);
        assert!(list.iter().copied().eq(0..len));
    }

    #[quickcheck]
    fn linked_list_search_existing(elem: usize) -> bool {
        let mut list = List::default();
        list.add(elem);
        list.find(&elem)
    }

    #[quickcheck]
    fn linked_list_search_nonexisting(elem: usize) -> bool {
        let list = List::default();
        !list.find(&elem)
    }

    #[test]
    fn ordered_list() {
        let min = 0;
        let max = 10_000;

        let mut list = OrderedList::<usize>::default();
        for i in min..max {
            list.add(i);
        }
        assert_eq!(list.len(), max - min);
        assert!(list.iter().copied().eq(min..max));

        let mut rev_list = OrderedList::<usize>::default();
        for i in (min..max).rev() {
            rev_list.add(i);
        }
        assert_eq!(list.len(), max - min);
        assert!(list.iter().copied().eq(min..max));
    }

    #[test]
    fn ordered_list_find() {
        let min = 0;
        let max = 10_000;

        let mut list = OrderedList::<usize>::default();
        for i in min..max {
            list.add(i);
        }

        assert!(list.find(&min));
        assert!(!list.find(&max));
        assert!(list.find(&((min + max) / 2)));
    }

    fn insert_contains_delete<S>(s: Arc<S>, elems: Arc<Vec<S::Elem>>, min: usize, max: usize)
    where
        S: Set + Send,
        S::Elem: Clone,
    {
        let elems = &elems[min..max];

        for v in elems {
            assert!(s.add(v.clone()));
        }
        for v in elems {
            assert!(s.contains(v));
        }
        for v in elems {
            assert!(s.remove(v));
        }
        for v in elems {
            assert!(!s.contains(v));
        }
    }

    fn test_set<S>(elems: Vec<S::Elem>, num_thrs: usize)
    where
        S: Set + Send + Sync + Default + 'static,
        S::Elem: Sync + Send + Clone,
    {
        let num_inserts = elems.len() / num_thrs;
        let elems = Arc::new(elems);

        let set = Arc::new(S::default());
        let handles: Vec<_> = (0..num_thrs)
            .map(|i| {
                let s = set.clone();
                let elems = elems.clone();
                let start = i * num_inserts;
                let end = start + num_inserts;
                std::thread::spawn(move || insert_contains_delete(s, elems, start, end))
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
    }

    #[cfg(test)]
    mod coarse_set {
        use crate::list_set::coarse_set::CoarseSet;

        #[test]
        fn coarse_set() {
            super::test_set::<CoarseSet<usize>>((0..10_000).collect(), 8);
        }
    }

    #[cfg(test)]
    mod fine_grained_set {
        use crate::list_set::fine_grained_set::FineGrainedSet;

        #[test]
        fn fine_grained_set() {
            super::test_set::<FineGrainedSet<usize>>((0..10_000).collect(), 8);
        }
    }
}
