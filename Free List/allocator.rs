use std::alloc::{GlobalAlloc, Layout};
use std::ptr::{self, NonNull};
use std::sync::Mutex

#[repr(C)]

struct FreeBlock {
   size: usize,
   next: *mut FreeBlock,
}

pub struct FreeListAllocator {
   free_list: Mutex<*mut FreeBlock>,
   heap_start: usize,
   heap_end: usize,
}

impl FreeListAllocator {
   pub const fn new() -> Self {
      Self {
         free_list: Mutex::new(ptr::null_mut()),
         heap_start: 0,
         heap_size: 0,
      }
   }

   pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
      let mut free_list = self.free_list.lock().unwrap();
      
      let initial_block = heap_start as *mut FreeBlock;
      (*initial_block).size = heap_size;
      (*initial_block).next = ptr::null_mut();
      
      *free_list = initial_block;

      self.heap_start = heap_start;
      self.heap_end = heap_start + heap_size;
   }

   fn align_up(addr: usize, align: usize) -> usize {
      (addr + align - 1) & !(align - 1)
   }

   unsafe fn find_free_block(
      &self,
      size: usize,
      align: usize,
      
   ) -> Option<*mut u8> {
      let mut free_list = self.free_list.lock().unwrap();
      let mut current = *free_list;
      let mut prev: *mut *mut FreeBlock = &mut free_list;
      
      while !current.is_null() {
         let block = &mut *current;
         let block_start = current as usize;
         let aligned_start = Self::align_up(block_start, align);

         let aligned_end = aligned_start + size;
         
         if aligned_end <= block_start + block.size {
            *prev = block.next;
            
            let leftover_size = (block_start + block.size) - aligned_end;

            if leftover_size > std::mem::size_of::<FreeBlock>() {
               let new_free_block = aligned_end as *mut FreeBlock;
               (*new_free_block).size = leftover_size;
               (*new_free_block).next = *free_list;
               *free_list = new_free_block;
            }
            return Some(aligned_start as *mut u8);
         }

         prev = &mut block.next;
         current = block.next;
      }

      None
   }

   unsafe fn add_to_free_list(&self, ptr: *mut u8, size: usize) {
      let block = ptr as *mut FreeBlock;
      (*block).size = size;

      let mut free_list = self.free_list.lock().unwrap();
      (*block).next = *free_list;
      *free_list = block;
   }
}

unsafe impl GlobalAlloc for FreeListAllocator {
   unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
      let size = layout.size().max(std::mem::size_of::<FreeBlock>());
      let align = layout.align();
      
      match self.find_free_block(size, align) {
         Some(ptr) => {
            println!("Allocated {} bytes at {:p}", size, ptr);
            ptr 
         }
         None => {
            println!("Allocation of {} bytes failed ", size);
            ptr::null_mut()
         }
      }
   }

   unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
      let size = layout.size().max(std::mem::size_of::<FreeBlock>());

      println!("Deallocated {} bytes at {:p}", size, ptr);
      self.add_to_free_list(ptr, size);
   }
}