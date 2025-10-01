use std::alloc::{GlobalAlloc, Layout};
use std::ptr::{self, NonNull};
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicPtr, Ordering};

pub struct PoolAllocator {
    pool: UnsafeCell<PoolData>,
}

struct PoolData {
    memory: *mut u8,
    free_list: *mut FreeBlock,
    block_size: usize,
    block_count: usize,
    alignment: usize,
}

struct FreeBlock {
    next: *mut FreeBlock,
}


impl PoolAllocator {
    pub const fn new() -> Self {
        Self {
            pool: UnsafeCell::new(PoolData {
                memory: ptr::null_mut(),
                free_list: ptr::null_mut(),
                block_size: 0,
                block_count: 0,
                alignment: 0,
            }),
        }
    }

    pub unsafe fn init(&self, block_size: usize, block_count: usize, alingment: usize) {
        let aligned_block_size = Self::align_size( std::cmp::max(block_size, std::mem::size_of::<FreeBlock>()), alignment);
        
        let total_size = aligned_block_size * block_count;
        let layout = Layout::from_size_align(total_size, alignment).unwrap();
        let memory = std::alloc::alloc(layout);

        if memory.is_null() {
            panic!("Failed to allocate pool memory");
        }

        let pool = &mut *self.pool.get();
        pool.memory = memory;
        pool.block_size = aligned_block_size;
        pool.block_count = block_count;
        pool.alignment = alignment;

        pool.free_list = memory as *mut FreeBlock;
        let mut current = pool.free_list;

        for i in 0..block_count {
            let next_addr = memory.add(i * aligned_block_size) as *mut FreeBlock;
            if i == block_count - 1 {
                (*current).next = ptr::null_mut();
            } else {
                (*current).next = memory.add((i + 1) * aligned_block_size) as *mut FreeBlock;   
            }
            current = next_addr;
        }
    }

    pub fn allocate(&self) -> Option<NonNull<u8>> {
        unsafe {
            let pool = &mut *self.pool.get();

            if pool.free_list.is_null() {
                return None;
            }

            let block = pool.free_list as *mut u8;
            pool.free_list = (*pool.free_list).next;

            NonNull::new(block)
        }
    }

    pub unsafe fn deallocate(&self, ptr: *mut u8) -> bool {
        let pool = &mut *self.pool.get();

        if !self.owns_pointer(ptr) {
            return false;
        }

        let free_block = ptr as *mut FreeBlock;
        (*free_block).next = pool.free_list;
        pool.free_list = free_block;
        
        true
    }

    fn owns_pointer(&self, ptr: *mut u8) -> bool {
        unsafe {
            let pool = &*self.pool.get();
            let start = pool.memory;
            let end = pool.memory.add(pool.block_size * pool.block_count);
            
            ptr >= start && ptr < end && {
                let offset = ptr as usize - start as usize;
                offset % pool.block_size == 0
            }
        }
    }

    fn align_size(size: usize, alignment: usize) -> usize {
        (size + alignment - 1) & !(alignment - 1)
    }
}

unsafe impl Send for PoolAllocator {}
unsafe impl Sync for PoolAllocator {}

pub struct LockFreePoolAllocator {
    memory: *mut u8,
    free_list: AtomicPtr<FreeBlock>,
    block_size: usize,
    block_count: usize,
}

impl LockFreePoolAllocator {
    pub unsafe fn new(block_size: usize, block_count: usize, alignment: usize) -> Self {
        let aligned_block_size = (std::cmp::max(block_size, std::mem::size_of::<FreeBlock>()) + alignment - 1) & !(alignment - 1);
        let total_size = aligned_block_size * block_count;
        let layout = Layout::from_size_align(total_size, alignment).unwrap();
        let memory = std::alloc::alloc(layout);

        if memory.is_null() {
            panic!("Failed to allocate");
        }

        for i in 0..block_count {
            let current = memory.add(i * aligned_block_size) as *mut FreeBlock;
            let next = if i == block_count - 1 {
                ptr::null_mut()
            } else {
                memory.add((i + 1) * aligned_block_size) as *mut FreeBlock
            };
            (*current).next = next;
        }
        Self {
            memory,
            free_list: AtomicPtr::new(memory as *mut FreeBlock),
            block_size: aligned_block_size,
            block_count,
        }
    }

    pub fn allocate(&self) -> Option<NonNull<u8>> {
        let mut head = self.free_list.load(Ordering::Acquire);
        
        loop {
            if head.is_null() {
                return None;
            }
            unsafe {
                let next = (*head).next;
                match self.free_list.compare_exchange_weak (
                    head,
                    next,
                    Ordering::Release,
                    Ordering::Relaxed
                ) {
                    Ok(_)=> return NonNull::new(head as *mut u8),
                    Err(actual) => head = actual,
                }
            }
        }
    }

    pub unsafe fn deallocate(&self, ptr: *mut u8) {
        let free_block = ptr as *mut FreeBlock;
        let mut head = self.free_list.load(Ordering::Relaxed);

        loop {
            (*free_block).next = head;

            match self.free_list.compare_exchange_weak (
                head,
                free_block,
                Ordering::Release,
                Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(actual) => head = actual,
            }
        }
    }

}