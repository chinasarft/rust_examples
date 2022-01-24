//#![feature(rustc_private)]

use std::alloc::{GlobalAlloc, Layout};
use std::sync::atomic::{AtomicUsize, Ordering};

extern crate libc;

struct MyAllocator {
    offset: AtomicUsize,
    alloc_count: AtomicUsize,
    free_count: AtomicUsize,
}

unsafe impl GlobalAlloc for MyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = libc::malloc(layout.size() as libc::size_t) as *mut u8;
        self.offset
            .fetch_add(layout.size() as usize, Ordering::SeqCst);
        self.alloc_count.fetch_add(1, Ordering::SeqCst);
        ret
    }
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        libc::free(ptr as *mut libc::c_void);
        self.free_count.fetch_add(1, Ordering::SeqCst);
    }
}

#[global_allocator]
static A: MyAllocator = MyAllocator {
    offset: AtomicUsize::new(0),
    alloc_count: AtomicUsize::new(0),
    free_count: AtomicUsize::new(0),
};

fn main() {
    println!("it runs!{} ac:{} fc:{}", A.offset.load(Ordering::SeqCst),
        A.alloc_count.load(Ordering::SeqCst), A.free_count.load(Ordering::SeqCst));
    println!("it runs again!{} ac:{} fc:{}", A.offset.load(Ordering::SeqCst),
        A.alloc_count.load(Ordering::SeqCst), A.free_count.load(Ordering::SeqCst));
}

