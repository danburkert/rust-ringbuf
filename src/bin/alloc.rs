#![crate_id = "alloc-bench"]
#![crate_type = "bin"]

extern crate debug;
extern crate test;
extern crate core;

extern crate criterion;

use std::rt::heap::{allocate, reallocate, deallocate, reallocate_inplace};
use std::mem;
use std::ptr;

use criterion::{Bencher, Criterion};

fn main() {
  let mut b = Criterion::new();
  let sizes = &[8, 128, 1024, 32 * 1024];
  b.bench_group("alloc", sizes, alloc);
  b.bench_group("alloc", sizes, alloc_vec);
  //b.bench_group("alloc-vec-no-forget", sizes, alloc_vec_no_forget);
  //b.bench_group("alloc_vec", sizes, alloc_vec);
  //b.bench_group("realloc", sizes, realloc);
  //b.bench_group("realloc_manual", sizes, realloc_manual);
  //b.bench_group("realloc_manual_on_fail", sizes, realloc_manual_on_fail);
}

#[allow(dead_code)]
fn alloc(b: &mut Bencher, n: uint) {
    let alignment = mem::min_align_of::<u8>();
    b.iter(|| {
        unsafe {
            let ptr =
                if mem::size_of::<u8>() == 0 {
                    0 as *mut u8
                } else if n == 0 {
                    0 as *mut u8
                } else {
                    let size = n.checked_mul(&mem::size_of::<u8>())
                                .expect("capacity overflow");
                    allocate(size, mem::min_align_of::<u8>())
                };
            for i in range(0, n) {
                let slot = ptr.offset(i as int);
                ptr::write(&mut *slot, i as u8);
            }
            test::black_box(&ptr);
            deallocate(ptr, n, alignment);
        }
    })
}

#[allow(dead_code)]
fn alloc_vec(b: &mut Bencher, n: uint) {
  b.iter(|| {
    unsafe {
      let mut vec = Vec::with_capacity(n);
      let mut ptr = vec.as_mut_ptr();
      mem::forget(vec);

      for i in range(0, n) {
        let slot = ptr.offset(i as int);
        ptr::write(&mut *slot, i as u8);
      }

      test::black_box(&mut ptr);
      Vec::from_raw_parts(0, n, ptr);
    }
  })
}

#[allow(dead_code)]
fn alloc_vec_no_forget(b: &mut Bencher, n: uint) {
  b.iter(|| {
    unsafe {
      let mut vec = Vec::with_capacity(n);
      let mut ptr = vec.as_mut_ptr();

      for i in range(0, n) {
        let slot = ptr.offset(i as int);
        ptr::write(&mut *slot, i as u8);
      }

      test::black_box(&mut ptr);
    }
  })
}

#[allow(dead_code)]
fn realloc(b: &mut Bencher, n: uint) {
  let alignment = mem::min_align_of::<u8>();
  b.iter(|| {
    unsafe {
      let mut ptr = allocate(n, alignment);

      for i in range(0, n) {
        let slot = ptr.offset(i as int);
        ptr::write(&mut *slot, i as u8);
      }

      ptr = reallocate(ptr, 2 * n, alignment, n);

      test::black_box(&mut ptr);
      deallocate(ptr, n * 2, alignment);
    }
  })
}

#[allow(dead_code)]
fn realloc_manual(b: &mut Bencher, n: uint) {
  b.iter(|| {
    let alignment = mem::min_align_of::<u8>();
    unsafe {
      let mut ptr = allocate(n, alignment);

      for i in range(0, n) {
        let slot = ptr.offset(i as int);
        ptr::write(&mut *slot, i as u8);
      }

      let ptr2 = allocate(2 * n, alignment);
      ptr::copy_nonoverlapping_memory(ptr2, ptr as *const u8, n);

      deallocate(ptr, n, alignment);

      test::black_box(&mut ptr);
      deallocate(ptr2, n * 2, alignment);
    }
  })
}

#[allow(dead_code)]
fn realloc_manual_on_fail(b: &mut Bencher, n: uint) {
  b.iter(|| {
    let alignment = mem::min_align_of::<u8>();
    unsafe {
      let mut ptr = allocate(n, alignment);

      for i in range(0, n) {
        let slot = ptr.offset(i as int);
        ptr::write(&mut *slot, i as u8);
      }

      let inplace = reallocate_inplace(ptr, 2 * n, alignment, n);
      if !inplace {
        let temp = allocate(2 * n, alignment);
        ptr::copy_nonoverlapping_memory(temp, ptr as *const u8, n);

        deallocate(ptr, n, alignment);
        ptr = temp;
      }

      test::black_box(&mut ptr);
      deallocate(ptr, n * 2, alignment);
    }
  })
}
