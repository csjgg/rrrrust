use std::fmt;
use std::option::Option;

pub struct LinkedList<T> {
    head: Option<Box<Node<T>>>,
    size: usize,
}

struct Node<T> {
    value: T,
    next: Option<Box<Node<T>>>,
}

impl<T> Node<T> {
    pub fn new(value: T, next: Option<Box<Node<T>>>) -> Node<T> {
        Node {
            value: value,
            next: next,
        }
    }
}

impl<T> LinkedList<T> {
    pub fn new() -> LinkedList<T> {
        LinkedList {
            head: None,
            size: 0,
        }
    }

    pub fn get_size(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.get_size() == 0
    }

    pub fn push_front(&mut self, value: T) {
        let new_node: Box<Node<T>> = Box::new(Node::new(value, self.head.take()));
        self.head = Some(new_node);
        self.size += 1;
    }

    pub fn pop_front(&mut self) -> Option<T> {
        let node: Box<Node<T>> = self.head.take()?;
        self.head = node.next;
        self.size -= 1;
        Some(node.value)
    }
}

impl<T:Clone> Clone for LinkedList<T>{
    fn clone(&self) -> Self {
        let mut new_list = LinkedList::new();
        let mut new_list2 = LinkedList::new();
        let mut current: &Option<Box<Node<T>>> = &self.head;
        loop{
            match current{
                Some(node) => {
                    new_list2.push_front(node.value.clone());
                    current = &node.next;
                }
                None => break,
            }
        }
        let mut t = new_list2.pop_front();
        loop{
            match t{
                Some(val) => {
                    new_list.push_front(val.clone());
                    t = new_list2.pop_front();
                }
                None => break,
            }
        }
        new_list
    }
}

impl<T:PartialEq> PartialEq for LinkedList<T>{
    fn eq(&self, other: &Self) -> bool{
        let mut current1: &Option<Box<Node<T>>> = &self.head;
        let mut current2: &Option<Box<Node<T>>> = &other.head;
        loop{
            match (current1, current2){
                (Some(node1), Some(node2)) => {
                    if node1.value != node2.value{
                        return false;
                    }
                    current1 = &node1.next;
                    current2 = &node2.next;
                }
                (None, None) => break,
                _ => return false,
            }
        }
        true
    }
}

impl<T: fmt::Display> fmt::Display for LinkedList<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut current: &Option<Box<Node<T>>> = &self.head;
        let mut result = String::new();
        loop {
            match current {
                Some(node) => {
                    result = format!("{} {}", result, node.value);
                    current = &node.next;
                }
                None => break,
            }
        }
        write!(f, "{}", result)
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        let mut current = self.head.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
    }
}
