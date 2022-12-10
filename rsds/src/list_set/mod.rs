use std::mem::MaybeUninit;

#[derive(Default)]
pub struct LinkedList<T> {
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

impl<T> Node<T> {
    fn add(&mut self, elem: T) {
        // SAFETY: we guarantee node to be initialized except for during the
        // Tail -> Elem transition.
        let node = unsafe { self.node.assume_init_mut() };
        match node {
            NodeInner::Tail(_) => {
                let m = MaybeUninit::uninit();
                // SAFETY: we only swap init node with uninit memory, so the
                // node being swapped out is initialized
                let node = unsafe { std::mem::replace(&mut self.node, m).assume_init() };
                let new_node = match node {
                    NodeInner::Tail(my_elem) => {
                        NodeInner::Elem((my_elem, Box::new(NodeInner::Tail(elem).into())))
                    }
                    _ => unreachable!(),
                };
                self.node.write(new_node);
            }
            NodeInner::Elem(node) => {
                node.1.add(elem);
            }
        }
    }

    fn len(&self) -> usize {
        // SAFETY: we guarantee node to be initialized except for during the
        // Tail -> Elem transition.
        let node = unsafe { self.node.assume_init_ref() };
        match node {
            NodeInner::Tail(_) => 1,
            NodeInner::Elem(n) => 1 + n.1.len(),
        }
    }
}

impl<T> LinkedList<T> {
    pub fn add(&mut self, elem: T) {
        if self.head.is_some() {
            self.head.as_mut().unwrap().add(elem);
        } else {
            self.head = Some(NodeInner::Tail(elem).into());
        }
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

    #[test]
    fn linked_list() {
        let mut list = LinkedList::default();
        assert!(list.is_empty());
        for i in 0..100 {
            list.add(i);
        }
        assert!(list.len() == 100);
    }
}
