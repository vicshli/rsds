use std::mem::MaybeUninit;

enum NodeInner<T, N> {
    Elem((T, Box<N>)),
    Tail(T),
}

impl<T> From<NodeInner<T, Node<T>>> for Node<T> {
    fn from(inner: NodeInner<T, Node<T>>) -> Self {
        Self {
            node: MaybeUninit::new(inner),
        }
    }
}

impl<T, N> NodeInner<T, N> {
    fn elem(&self) -> &T {
        match self {
            NodeInner::Elem((e, _)) => e,
            NodeInner::Tail(e) => e,
        }
    }
}

struct Node<T> {
    node: MaybeUninit<NodeInner<T, Node<T>>>,
}

impl<T> Node<T> {
    fn new_tail(elem: T) -> Self {
        NodeInner::Tail(elem).into()
    }

    fn get(&self) -> &T {
        self.get_node_ref().elem()
    }

    fn next(&self) -> Option<&Self> {
        let node = self.get_node_ref();
        match node {
            NodeInner::Tail(_) => None,
            NodeInner::Elem((_, rest)) => Some(rest.as_ref()),
        }
    }

    fn next_mut(&mut self) -> Option<&mut Self> {
        let node = self.get_node_mut();
        match node {
            NodeInner::Tail(_) => None,
            NodeInner::Elem((_, rest)) => Some(rest.as_mut()),
        }
    }

    fn add(&mut self, elem: T) {
        // SAFETY: we only swap init node with uninit memory, so the
        // node being swapped out is initialized.
        let node =
            unsafe { std::mem::replace(&mut self.node, MaybeUninit::uninit()).assume_init() };
        let new_node = match node {
            NodeInner::Tail(curr) => {
                NodeInner::Elem((curr, Box::new(NodeInner::Tail(elem).into())))
            }
            NodeInner::Elem((curr, rest)) => {
                let next = NodeInner::Elem((elem, rest));
                NodeInner::Elem((curr, Box::new(next.into())))
            }
        };
        self.node.write(new_node);
    }

    fn find(&self, target: &T) -> bool
    where
        T: PartialEq,
    {
        let node = self.get_node_ref();
        match node {
            NodeInner::Elem((curr, rest)) => curr == target || rest.find(target),
            NodeInner::Tail(curr) => curr == target,
        }
    }
}

impl<T> Node<T> {
    fn get_node_mut(&mut self) -> &mut NodeInner<T, Node<T>> {
        // SAFETY: we guarantee node to be initialized except for during the
        // Tail -> Elem transition.
        unsafe { self.node.assume_init_mut() }
    }
    fn get_node_ref(&self) -> &NodeInner<T, Node<T>> {
        // SAFETY: we guarantee node to be initialized except for during the
        // Tail -> Elem transition.
        unsafe { self.node.assume_init_ref() }
    }
}

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
        self.head.as_ref().map(|h| h.find(target)).unwrap_or(false)
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

#[derive(Default)]
pub struct List<T> {
    inner: ListInner<T>,
}

impl<T> List<T>
where
    T: PartialEq + Eq,
{
    pub fn add(&mut self, elem: T) {
        self.inner.add(elem)
    }

    pub fn find(&self, target: &T) -> bool {
        self.inner.find(target)
    }

    pub fn iter(&self) -> ListIter<'_, T> {
        self.inner.iter()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[derive(Default)]
pub struct OrderedList<T> {
    inner: ListInner<T>,
}

impl<T> OrderedList<T>
where
    T: PartialOrd + PartialEq + Eq,
{
    pub fn add(&mut self, elem: T) {
        self.inner.add_ordered(elem)
    }

    pub fn find(&self, target: &T) -> bool {
        self.inner.find_ordered(target)
    }

    pub fn iter(&self) -> ListIter<'_, T> {
        self.inner.iter()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[cfg(test)]
mod tests {
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
}
