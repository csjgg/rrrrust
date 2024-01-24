use linked_list::LinkedList;
pub mod linked_list;

fn main() {
    let mut list: LinkedList<String> = LinkedList::new();
    assert!(list.is_empty());
    assert_eq!(list.get_size(), 0);
    for i in 1..12 {
        list.push_front(i.to_string());
    }
    println!("{}", list);
    println!("list size: {}", list.get_size());
    println!("top element: {}", list.pop_front().unwrap());
    println!("{}", list);
    println!("size: {}", list.get_size());
    let mut list2 = list.clone();
    println!("list1:{}\nlist2:{}",list, list2);
    let _ = list2.pop_front().unwrap();
    if list == list2{
        println!("yes");
    }else{
        println!("no");
    }
    println!("list1:{}\nlist2:{}",list, list2);
    // println!("{}", list.to_string()); // ToString impl for anything impl Display

    // If you implement iterator trait:
    //for val in &list {
    //    println!("{}", val);
    //}
}
