#![crate_name = "ringbuf-bench"]
#![crate_type = "bin"]

extern crate debug;
extern crate test;
extern crate core;
extern crate collections;

extern crate ringbuf;
extern crate criterion;

// Switch from benchmarking the stdlib implementation to the new
// implementation by changing the import
//use collections::RingBuf;
use ringbuf::RingBuf;

use collections::Deque;
use std::rand::Rng;
use std::rand::SeedableRng;
use std::rand::StdRng;

use criterion::{Bencher, Criterion};

#[allow(dead_code)]
fn main() {
    let mut b = Criterion::default();
    let capacities = &[8u, 1024, 32 * 1024];
    //b.bench_group("push_back_default_allocate", capacities, push_back_default_allocate);
    //b.bench_group("push_back_pre_allocate", capacities, push_back_pre_allocate);
    //b.bench_group("push_pre_default_allocate", capacities, push_front_default_allocate);
    //b.bench_group("push_pre_allocate", capacities, push_front_pre_allocate);
    b.bench_family("iterate", iterate, capacities);
    //b.bench_group("get", capacities, get);
    b.bench_family("move_iterator", move_iterator, capacities);
    b.bench_family("safe_move_iterator", safe_move_iterator, capacities);
}

fn get_rng() -> StdRng {
    SeedableRng::from_seed(&[1, 2, 3, 4])
}

#[allow(dead_code)]
fn allocate(b: &mut Bencher, capacity: &uint) {
    b.iter(|| {
        let mut rb = RingBuf::<int>::with_capacity(*capacity);
        test::black_box(&mut rb);
    })
}

#[allow(dead_code)]
fn push_back_pre_allocate(b: &mut Bencher, capacity: &uint) {
    let items: Vec<int> = get_rng().gen_iter::<int>().take(*capacity).collect();
    b.iter(|| {
        let mut rb: RingBuf<int> = RingBuf::with_capacity(*capacity);
        for &item in items.iter() {
            rb.push_back(item);
        }
        test::black_box(&mut rb);
    })
}

#[allow(dead_code)]
fn push_back_default_allocate(b: &mut Bencher, capacity: &uint) {
    let items: Vec<int> = get_rng().gen_iter::<int>().take(*capacity).collect();
    b.iter(|| {
        let mut rb: RingBuf<int> = RingBuf::with_capacity(8);
        for &item in items.iter() {
            rb.push_back(item);
        }
        test::black_box(&mut rb);
    })
}

#[allow(dead_code)]
fn push_front_pre_allocate(b: &mut Bencher, capacity: &uint) {
    let items: Vec<int> = get_rng().gen_iter::<int>().take(*capacity).collect();
    b.iter(|| {
        let mut rb: RingBuf<int> = RingBuf::with_capacity(*capacity);
        for &item in items.iter() {
            rb.push_front(item);
        }
        test::black_box(&mut rb);
    })
}

#[allow(dead_code)]
fn push_front_default_allocate(b: &mut Bencher, capacity: &uint) {
    let items: Vec<int> = get_rng().gen_iter::<int>().take(*capacity).collect();
    b.iter(|| {
        let mut rb: RingBuf<int> = RingBuf::with_capacity(8);
        for &item in items.iter() {
            rb.push_front(item);
        }
        test::black_box(&mut rb);
    })
}

#[allow(dead_code)]
fn iterate(b: &mut Bencher, capacity: &uint) {
    let mut rb = RingBuf::with_capacity(*capacity);
    for element in get_rng().gen_iter::<int>().take(*capacity) {
        rb.push_back(element);
    }

    b.iter(|| {
        for &element in rb.clone().iter() {
            test::black_box(&element);
        }
    })
}

#[allow(dead_code)]
fn move_iterator(b: &mut Bencher, capacity: &uint) {
    let mut rb = RingBuf::with_capacity(*capacity);
    for element in get_rng().gen_iter::<int>().take(*capacity) {
        rb.push_back(element);
    }

    b.iter(|| {
        for element in rb.clone().move_iter() {
            test::black_box(element);
        }
    })
}

#[allow(dead_code)]
fn safe_move_iterator(b: &mut Bencher, capacity: &uint) {
    let mut rb = RingBuf::with_capacity(*capacity);
    for element in get_rng().gen_iter::<int>().take(*capacity) {
        rb.push_back(element);
    }

    b.iter(|| {
        for element in (SafeMoveItems{ ringbuf: rb.clone() }) {
            test::black_box(element);
        }
    })
}


#[allow(dead_code)]
fn get(b: &mut Bencher, capacity: &uint) {
    let mut rb = RingBuf::with_capacity(*capacity);
    for element in get_rng().gen_iter::<int>().take(*capacity) {
        rb.push_back(element);
    }

    b.iter(|| {
        for i in range(0, *capacity) {
            test::black_box(rb.get(i));
        }
    })
}

/// An iterator that moves out of a RingBuf.
pub struct SafeMoveItems<T> {
    ringbuf: RingBuf<T>
}

impl<T> Iterator<T> for SafeMoveItems<T> {
    #[inline]
    fn next(&mut self) -> Option<T> {
        self.ringbuf.pop_front()
    }

    #[inline]
    fn size_hint(&self) -> (uint, Option<uint>) {
        (self.ringbuf.len(), Some(self.ringbuf.len()))
    }
}

impl<T> DoubleEndedIterator<T> for SafeMoveItems<T> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        self.ringbuf.pop_back()
    }
}
