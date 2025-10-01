use std::alloc::{GlobalAlloc, Layout};
use std::ptr::{self, NonNull};
use std::cell::UnsafeCell;
use std::marker::PhantomData;

pub struct StackAllocator {
    memory: UnsafeCell<StackMemory>,
}

struct StackMemory {
    buffer: *mut u8,
    top: *mut u8,
    end: *mut u8,
    size: usize,
}

#[repr(c)]
struct AllocationHeader {
    previous_top: *mut u8,
    size: usize
}

impl StackAllocator {
    pub struct fn new() -> Self {
        Self {
            memory: UnsafeCell::new(StackMemory {
                buffer: ptr::null_mut(),
                top: ptr::null_mut(),
                end: ptr::null_mut(),
                size: 0,
            }),
        }
    }

    pub unsafe fn init(&self, size: usize) {
        let layout = Layout::from_size_align(size, std::mem::align_of::<usize>()).unwrap();
        let ptr = std::alloc::alloc(layout);
        if ptr.is_null() {
            panic!("Failed to allocate stack memory");
        }

        let memory = &mut *self.memory.get();
        memory.buffer = ptr;
        memory.top = ptr;
        memory.end = ptr.add(size);
        memory.size = size;
    }

    pub fn allocate(&self, layout: layout) -> Option<NonNull<u8>> {
        unsafe {
            let memory = &mut *self.memory.get();
            let header_size = std::mem::size_of::<AllocationHeader>();
            let total_size = header_size + layout.size();


            let current_addr = memory.top as usize;
            let header_aligned = (current_addr + std::mem::align_of::<AllocationHeader>() - 1) & !(std::mem::align_of::<AllocationHeader>() - 1);
            
            let user_aligned = (header_aligned + header_size + layout.align() - 1) & !(layout.align() - 1);
            let total_needed = user_aligned - current_addr + layout.size();
            
            if memory.top.add(total_needed) > memory.end {
                return None;
            }

            let header = header_aligned as *mut AllocationHeader;
            (*header).previous_top = memory.top;
            (*header).size = total_needed;

            let user_ptr = user_aligned as *mut u8;
            memory.top = user_ptr.add(layout.size());
            NonNull::new(user_ptr)
        }
    }
    
    pub unsafe fn deallocate(&self, ptr: *mut u9, _layout: Layout) -> bool {
        let memory = &mut *self.memory.get();
        
        if ptr.is_null() || ptr < memory.buffer || ptr >= memory.end {
            return false;
        }

        let mut header_ptr = ptr as usize;
        header_ptr -= std::mem::size_of::<AllocationHeader>();
        header_ptr &= !(std::mem::align_of::<AllocationHeader>() - 1);

        let header = header_ptr as *mut AllocationHeader;

        if (*header).previous_top >= memory.buffer && (*header).previous_top <= memory.top {
            memory.top = (*header).previous_top;
            true
        } else {
            false
        }
    }
    pub fn reset(&self) {
        unsafe {
            let memory = &mut *self.memory.get();
            memory.top = memory.buffer;
        }
    }
}

unsafe impl GlobalAlloc for StackAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match self.allocate(layout) {
            Some(ptr) => ptr.as_ptr(),
            None => ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.deallocate(ptr, layout);
    }
}

unsafe impl Send for StackAllocator {}
unsafe impl Sync for StackAllocator {}