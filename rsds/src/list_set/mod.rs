use std::mem::MaybeUninit;

#[derive(Default)]
pub struct List<T> {
    head: Option<Node<T>>,
}

enum NodeInner<T> {
    Elem((T, Box<Node<T>>)),
    Tail(T),
}

impl<T> From<NodeInner<T>> for Node<T> {
    fn from(inner: NodeInner<T>) -> Self {
        Self {
            node: MaybeUninit::new(inner),
        }
    }
}

struct Node<T> {
    node: MaybeUninit<NodeInner<T>>,
}

impl<T> Node<T>
where
    T: PartialEq + Eq,
{
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

    fn get_node_mut(&mut self) -> &mut NodeInner<T> {
        // SAFETY: we guarantee node to be initialized except for during the
        // Tail -> Elem transition.
        unsafe { self.node.assume_init_mut() }
    }
    fn get_node_ref(&self) -> &NodeInner<T> {
        // SAFETY: we guarantee node to be initialized except for during the
        // Tail -> Elem transition.
        unsafe { self.node.assume_init_ref() }
    }
}

impl<T> List<T>
where
    T: PartialEq + Eq,
{
    pub fn add(&mut self, elem: T) {
        if self.head.is_some() {
            self.head.as_mut().unwrap().add(elem);
        } else {
            self.head = Some(NodeInner::Tail(elem).into());
        }
    }

    pub fn find(&self, target: &T) -> bool {
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
