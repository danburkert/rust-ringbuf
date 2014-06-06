#![crate_id = "sort-bench"]
#![crate_type = "bin"]

extern crate debug;
extern crate test;
extern crate core;

extern crate sort;
extern crate criterion;

use std::rt::heap::{allocate, reallocate, deallocate, reallocate_inplace};
use std::mem;
use std::ptr;

use criterion::{Bencher, Criterion};

fn main() {
  let mut b = Criterion::new();
  let sizes = &[131072];
  b.bench_group("realloc", sizes, realloc);
  b.bench_group("realloc_manual", sizes, realloc_manual);
  b.bench_group("realloc_manual_on_fail", sizes, realloc_manual_on_fail);
}

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
      ptr::copy_nonoverlapping_memory(ptr2, ptr as *u8, n);

      deallocate(ptr, n, alignment);

      test::black_box(&mut ptr);
      deallocate(ptr2, n * 2, alignment);
    }
  })
}

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
        ptr::copy_nonoverlapping_memory(temp, ptr as *u8, n);

        deallocate(ptr, n, alignment);
        ptr = temp;
      }

      test::black_box(&mut ptr);
      deallocate(ptr, n * 2, alignment);
    }
  })
}
