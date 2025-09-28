use std::alloc::{GlobalAlloc, Layout, System};


#[global_allocator]

static ALLOCATOR: System = System;

fn main() {
    let vec = vec![1, 2, 3, 4, 5];
    let boxed = Box::new(42);
    println!("vec: {:?}, Boxed: {}, vec, boxed");
}