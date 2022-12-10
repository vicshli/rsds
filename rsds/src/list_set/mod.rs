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

trait ListNode {
    type Elem;

    fn new_tail(elem: Self::Elem) -> Self;

    fn new_intermediate(elem: Self::Elem, rest: Self) -> Self;

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
        // SAFETY: this node is guaranteed to be initialized except
        // for the following section, where it is moved to after elem.
        let tail_node =
            unsafe { std::mem::replace(&mut self.node, MaybeUninit::uninit()).assume_init() };
        let curr_node = NodeInner::Elem((elem, Box::new(tail_node.into())));
        // Inserted `elem` before tail.
        // The list becomes: ... -> elem -> tail
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

struct ListInner<N> {
    head: Option<N>,
}

impl<N> Default for ListInner<N> {
    fn default() -> Self {
        Self { head: None }
    }
}

impl<N> ListInner<N>
where
    N: ListNode,
{
    pub fn add(&mut self, elem: N::Elem) {
        if self.head.is_some() {
            self.head.as_mut().unwrap().add(elem);
        } else {
            self.head = Some(N::new_tail(elem));
        }
    }

    pub fn find(&self, target: &N::Elem) -> bool {
        self.head.as_ref().map(|h| h.find(target)).unwrap_or(false)
    }

    pub fn len(&self) -> usize {
        match &self.head {
            Some(node) => node.len(),
            None => 0,
        }
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
        self.inner.add(elem)
    }

    pub fn find(&self, target: &T) -> bool {
        self.inner.find(target)
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
        let mut list = List::default();
        assert!(list.is_empty());
        for i in 0..100 {
            list.add(i);
        }
        assert!(list.len() == 100);
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
}
