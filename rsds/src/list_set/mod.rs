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

impl<T> From<NodeInner<T, OrderedNode<T>>> for OrderedNode<T> {
    fn from(inner: NodeInner<T, OrderedNode<T>>) -> Self {
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

trait ListNode {
    type Elem;

    fn new_tail(elem: Self::Elem) -> Self;

    fn new_intermediate(elem: Self::Elem, rest: Self) -> Self;

    fn get(&self) -> &Self::Elem;

    fn next(&self) -> Option<&Self>;

    fn next_mut(&mut self) -> Option<&mut Self>;

    fn add(&mut self, elem: Self::Elem);

    fn find(&self, target: &Self::Elem) -> bool;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

struct Node<T> {
    node: MaybeUninit<NodeInner<T, Node<T>>>,
}

impl<T> ListNode for Node<T>
where
    T: PartialEq + Eq,
{
    type Elem = T;

    fn new_tail(elem: T) -> Self {
        NodeInner::Tail(elem).into()
    }

    fn new_intermediate(elem: Self::Elem, rest: Self) -> Self {
        NodeInner::Elem((elem, Box::new(rest))).into()
    }

    fn get(&self) -> &Self::Elem {
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
        let node = self.get_node_mut();
        match node {
            NodeInner::Tail(_) => {
                // SAFETY: we only swap init node with uninit memory, so the
                // node being swapped out is initialized.
                let node = unsafe {
                    std::mem::replace(&mut self.node, MaybeUninit::uninit()).assume_init()
                };
                let new_node = match node {
                    NodeInner::Tail(my_elem) => {
                        NodeInner::Elem((my_elem, Box::new(NodeInner::Tail(elem).into())))
                    }
                    _ => unreachable!(),
                };
                self.node.write(new_node);
            }
            NodeInner::Elem((_, rest)) => {
                rest.add(elem);
            }
        }
    }

    fn find(&self, target: &T) -> bool {
        let node = self.get_node_ref();
        match node {
            NodeInner::Elem((curr, rest)) => curr == target || rest.find(target),
            NodeInner::Tail(curr) => curr == target,
        }
    }

    fn len(&self) -> usize {
        let node = self.get_node_ref();
        match node {
            NodeInner::Tail(_) => 1,
            NodeInner::Elem(n) => 1 + n.1.len(),
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

struct OrderedNode<T> {
    node: MaybeUninit<NodeInner<T, OrderedNode<T>>>,
}

impl<T> ListNode for OrderedNode<T>
where
    T: PartialOrd + PartialEq + Eq,
{
    type Elem = T;

    fn new_tail(elem: Self::Elem) -> Self {
        NodeInner::Tail(elem).into()
    }

    fn new_intermediate(elem: Self::Elem, rest: Self) -> Self {
        NodeInner::Elem((elem, Box::new(rest))).into()
    }

    fn get(&self) -> &Self::Elem {
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
        let node = self.get_node_mut();
        match node {
            NodeInner::Tail(curr) => {
                if *curr <= elem {
                    self._add_after_tail(elem);
                } else {
                    self._add_before_self(elem);
                }
            }
            NodeInner::Elem((curr, rest)) => {
                if *curr <= elem {
                    rest.add(elem);
                } else {
                    self._add_before_self(elem);
                }
            }
        }
    }

    fn find(&self, target: &T) -> bool {
        let node = self.get_node_ref();
        match node {
            NodeInner::Elem((curr, rest)) => *curr == *target || rest.find(target),
            NodeInner::Tail(curr) => *curr == *target,
        }
    }

    fn len(&self) -> usize {
        let node = self.get_node_ref();
        match node {
            NodeInner::Tail(_) => 1,
            NodeInner::Elem(n) => 1 + n.1.len(),
        }
    }
}

impl<T> OrderedNode<T>
where
    T: PartialOrd + PartialEq + Eq,
{
    fn _add_before_self(&mut self, elem: T) {
        // SAFETY: this node is guaranteed to be initialized except
        // for the following section, where it is moved to self.next.
        let next_node =
            unsafe { std::mem::replace(&mut self.node, MaybeUninit::uninit()).assume_init() };
        let curr_node = NodeInner::Elem((elem, Box::new(next_node.into())));
        // Inserted `elem` before myself.
        // The list becomes: ... -> elem -> myself -> rest...
        self.node.write(curr_node);
    }

    fn _add_after_tail(&mut self, elem: T) {
        assert!(matches!(self.get_node_ref(), NodeInner::Tail(_)));

        let old_tail = {
            // SAFETY: this node is guaranteed to be initialized except
            // for the following section, where it is moved to after elem.
            let tail_node =
                unsafe { std::mem::replace(&mut self.node, MaybeUninit::uninit()).assume_init() };

            match tail_node {
                NodeInner::Tail(e) => e,
                _ => unreachable!(),
            }
        };

        let curr_node = NodeInner::Elem((old_tail, Box::new(NodeInner::Tail(elem).into())));

        // The list becomes: ... -> old_tail -> elem
        self.node.write(curr_node);
    }

    fn get_node_mut(&mut self) -> &mut NodeInner<T, OrderedNode<T>> {
        // SAFETY: we guarantee node to be initialized except for node swapping
        unsafe { self.node.assume_init_mut() }
    }
    fn get_node_ref(&self) -> &NodeInner<T, OrderedNode<T>> {
        // SAFETY: we guarantee node to be initialized except for node swapping
        unsafe { self.node.assume_init_ref() }
    }
}

struct ListIterInner<'a, N> {
    curr: Option<&'a N>,
}

impl<'a, N> Iterator for ListIterInner<'a, N>
where
    N: ListNode,
{
    type Item = &'a N::Elem;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(curr) = self.curr {
            let next = curr.get();
            self.curr = curr.next();
            return Some(next);
        }
        None
    }
}

pub struct ListIter<'a, T>(ListIterInner<'a, Node<T>>);

impl<'a, T> Iterator for ListIter<'a, T>
where
    T: PartialEq + Eq,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<'a, T> From<ListIterInner<'a, Node<T>>> for ListIter<'a, T> {
    fn from(iter: ListIterInner<'a, Node<T>>) -> Self {
        ListIter(iter)
    }
}

pub struct OrderedListIter<'a, T>(ListIterInner<'a, OrderedNode<T>>);

impl<'a, T> Iterator for OrderedListIter<'a, T>
where
    T: PartialOrd + PartialEq + Eq,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}
impl<'a, T> ExactSizeIterator for OrderedListIter<'a, T> where T: PartialOrd + PartialEq + Eq {}

impl<'a, T> From<ListIterInner<'a, OrderedNode<T>>> for OrderedListIter<'a, T> {
    fn from(iter: ListIterInner<'a, OrderedNode<T>>) -> Self {
        OrderedListIter(iter)
    }
}

struct ListInner<N> {
    head: Option<N>,
    tail: Option<*mut N>,
    len: usize,
}

impl<N> Default for ListInner<N> {
    fn default() -> Self {
        Self {
            head: None,
            tail: None,
            len: 0,
        }
    }
}

impl<N> ListInner<N>
where
    N: ListNode,
{
    pub fn add(&mut self, elem: N::Elem) {
        if self.head.is_none() {
            self.head = Some(N::new_tail(elem));
            self.tail = Some(self.head.as_mut().unwrap());
        } else {
            // SAFETY: `tail` is guaranteed to be pointing to the list tail
            // and is guaranteed to be alive.
            let old_tail = unsafe { &mut *self.tail.unwrap() };
            old_tail.add(elem);
            let new_tail: *mut N = old_tail.next_mut().unwrap();
            self.tail = Some(new_tail);
        }
        self.len += 1;
    }

    pub fn add_ordered(&mut self, elem: N::Elem)
    where
        N::Elem: PartialOrd,
    {
        if self.head.is_none() {
            self.head = Some(N::new_tail(elem));
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
                let c = unsafe { &mut *(curr as *mut N) };
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

    pub fn find(&self, target: &N::Elem) -> bool {
        self.head.as_ref().map(|h| h.find(target)).unwrap_or(false)
    }

    pub fn iter(&self) -> ListIterInner<'_, N> {
        match self.head {
            Some(ref h) => ListIterInner { curr: Some(h) },
            None => ListIterInner { curr: None },
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
    inner: ListInner<Node<T>>,
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
        self.inner.iter().into()
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
    inner: ListInner<OrderedNode<T>>,
}

impl<T> OrderedList<T>
where
    T: PartialOrd + PartialEq + Eq,
{
    pub fn add(&mut self, elem: T) {
        self.inner.add_ordered(elem)
    }

    pub fn find(&self, target: &T) -> bool {
        self.inner.find(target)
    }

    pub fn iter(&self) -> OrderedListIter<'_, T> {
        self.inner.iter().into()
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
}
