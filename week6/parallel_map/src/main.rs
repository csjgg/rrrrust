use crossbeam_channel;
use std::{thread, time};

struct Resul<T> {
    value: T,
    index: usize,
}

fn parallel_map<T, U, F>(mut input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
where
    F: FnOnce(T) -> U + Send + Copy + 'static,
    T: Send + 'static,
    U: Send + 'static + Default,
{
    let mut output_vec: Vec<U> = Vec::with_capacity(input_vec.len());
    // TODO: implement parallel map!
    let (sender_to, receiver_to) = crossbeam_channel::unbounded();
    let (sender_back, receiver_back) = crossbeam_channel::unbounded();
    while !input_vec.is_empty() {
        let push = Resul {
            index: input_vec.len() - 1,
            value: input_vec.pop().unwrap(),
        };
        sender_to.send(push).unwrap();
    }
    drop(sender_to);

    let mut handlers = Vec::new();
    for _ in 0..num_threads {
        let receiver = receiver_to.clone();
        let sender = sender_back.clone();
        let handler = thread::spawn(move || {
            while let Ok(rec) = receiver.recv() {
                let result = f(rec.value);
                sender
                    .send(Resul {
                        value: result,
                        index: rec.index,
                    })
                    .unwrap();
            }
            drop(receiver);
            drop(sender);
        });
        handlers.push(handler);
    }
    drop(sender_back);
    drop(receiver_to);
    for handler in handlers {
        handler.join().unwrap();
    }
    let mut recvs = Vec::new();
    while let Ok(rec) = receiver_back.recv() {
        if recvs.len() == 0 {
            recvs.push(rec.index);
            output_vec.push(rec.value);
        } else {
            let mut i = 0;
            while i < recvs.len() && recvs[i] < rec.index {
                i += 1;
            }
            recvs.insert(i, rec.index);
            output_vec.insert(i, rec.value);
        }
    }
    drop(receiver_back);
    output_vec
}

fn main() {
    let v = vec![6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 12, 18, 11, 5, 20];
    let squares = parallel_map(v, 10, |num| {
        println!("{} squared is {}", num, num * num);
        thread::sleep(time::Duration::from_millis(500));
        num * num
    });
    println!("squares: {:?}", squares);
}
