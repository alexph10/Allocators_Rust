use std::alloc::{GlobalAlloc, Layout};
use std::ptr;
use std::sync::Mutex;

pub struct BumpAllocator{
    memory: Mutex<BumpMemory>,
}

struct BumpMemory {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    pub const fn new() -> Self {
        Self {
            memory: Mutex::new(BumpMemory {
                heap_start: 0,
                heap_end: 0, 
                next: 0,
                allocations: 0,
            }),
        }
    }

    pub unsafe fn init(&self, heap_start: usize, heap_end: usize) {
        let mut memory = self.memory.lock().unwrap();
        memory.heap_start = heap_start;
        memory.heap_end = heap_start + heap_size;
        memory.next = heap_start;
        memory.allocations = 0;
    }

    fn align_up(addr: usize, align:usize) -> usize {
        (addr + align - 1) & !(align - 1)
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: layout) -> *mut u8 {
        let mut memory = self.memory.lock().unwrap();

        let alloc_start = Self::align_up(memory.next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end;
            None => return ptr::null_mut(),
        };

        if alloc_end > memory.heap_end {
            ptr::null_mut()
        } else {
            memory.next = alloc_end;
            memory.allocation += 1;
            println!("Allocated {} bytes at {:p}", layout.size(), alloc_start as *mut u8);
            alloc_start as *mut u8
        }
    } 
}