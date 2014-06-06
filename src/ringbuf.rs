// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A double-ended queue implemented as a circular buffer
//!
//! RingBuf implements the trait Deque. It should be imported with
//! `use collections::Deque`.

extern crate alloc;
extern crate core;
extern crate debug;
extern crate test;

extern crate quickcheck;

use core::fmt;
use core::num;
use core::mem;
use core::raw::Slice;
use core::uint;
use std::iter::Chain;
use std::ptr;
use std::rt::heap::{allocate, deallocate};
use std::slice;
use std::collections::Deque;

static INITIAL_CAPACITY: uint = 8u; // 2^3
static MINIMUM_CAPACITY: uint = 2u;

/// RingBuf is a circular buffer that implements Deque.
///
/// # Examples
///
/// ```rust
/// use collections::RingBuf;
/// use collections::Deque;
/// let mut ringbuf = RingBuf::new();
/// ringbuf.push_front(1);
/// ringbuf.push_back(2);
///
/// assert_eq!(ringbuf.len(), 2);
/// assert_eq!(ringbuf.get(0), &1);
/// assert_eq!(ringbuf.front(), &1);
/// assert_eq!(ringbuf.back(), &2);
///
/// assert_eq!(vec.pop_back(), Some(2));
/// assert_eq!(vec.len(), 1);
/// ```
#[unsafe_no_drop_flag]
pub struct RingBuf<T> {

    /// The index of the 0th element
    /// invariant: `0 <= lo < cap`
    lo: uint,

    /// The number of elements currently in the ring.
    /// invariant: `0 <= len <= cap`
    len: uint,

    /// Capacity of the buffer.
    cap: uint,

    /// Pointer to the start of the buffer
    ptr: *mut T
}

/// RingBuf iterator.
pub type Items<'a, T> = Chain<slice::Items<'a, T>, slice::Items<'a, T>>;

/// RingBuf mutable iterator.
pub type MutItems<'a, T> = Chain<slice::MutItems<'a, T>, slice::MutItems<'a, T>>;

impl<T> RingBuf<T> {

    /// Construct a new, empty `RingBuf`.
    ///
    /// The ring buffer will allocate an initial capacity.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use collections::RingBuf;
    /// # use collections::Deque;
    ///
    /// let mut ringbuf: RingBuf<int> = RingBuf::new();
    /// ```
    pub fn new() -> RingBuf<T> {
        RingBuf::with_capacity(INITIAL_CAPACITY)
    }

    /// Constructs a new, empty `RingBuf` with the specified capacity.
    ///
    /// The ring will be able to hold exactly `capacity` elements without
    /// reallocating.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use collections::ringbuf::RingBuf;
    /// let ring: RingBuf<int> = RingBuf::with_capacity(10);
    /// ```
    pub fn with_capacity(capacity: uint) -> RingBuf<T> {
        if mem::size_of::<T>() == 0 {
            RingBuf { lo: 0, len: 0, cap: uint::MAX, ptr: 0 as *mut T }
        } else if capacity < MINIMUM_CAPACITY {
            RingBuf::with_capacity(MINIMUM_CAPACITY)
        } else {
            let size = capacity.checked_mul(&mem::size_of::<T>())
                               .expect("capacity overflow");
            let ptr = unsafe { allocate(size, mem::min_align_of::<T>()) };
            RingBuf { lo: 0, len: 0, cap: capacity, ptr: ptr as *mut T }
        }
    }

    /// Constructs a new `RingBuf` from the elements in a `Vec`.
    ///
    /// No copying will be done, and the new ring buffer will have the same
    /// capacity as the provided vec.
    ///
    /// # Example
    ///
    /// ```rust
    /// #use collections.ringbuf.RingBuf;
    /// let mut vec = vec!(1, 2, 3);
    /// let ringbuf = RingBuf::from_vec(vec);
    /// ```
    pub fn from_vec(mut vec: Vec<T>) -> RingBuf<T> {
        let len = vec.len();
        let cap = vec.capacity();
        let ptr = vec.as_mut_ptr();
        let ringbuf = RingBuf { lo: 0, len: len, cap: cap, ptr: ptr };
        unsafe { mem::forget(vec); }
        ringbuf
    }

    /// Constructs a new `Vec` from the elements in a `RingBuf`.
    ///
    /// May require copying and temporary allocation.
    ///
    /// # Example
    ///
    /// ```rust
    /// #use collections.ringbuf.RingBuf;
    /// let mut ringbuf = RingBuf::new();
    /// ringbuf.push_front(1);
    /// ringbuf.push_back(2);
    /// let vec = ringbuf.into_vec();
    /// ```
    pub fn into_vec(mut self) -> Vec<T> {
        self.reset();

        let vec;
        unsafe {
            vec = Vec::from_raw_parts(self.len, self.cap, self.ptr);
            mem::forget(self);
        }
        vec
    }

    /// Returns a reference to the value at index `index`.
    ///
    /// # Failure
    ///
    /// Fails if `index` is out of bounds.
    ///
    /// ```rust
    /// let ringbuf = RingBuff::from_vec(vec!(1, 2, 3));
    /// assert!(ringbuf.get(1) == &2);
    /// ```
    pub fn get<'a>(&'a self, index: uint) -> &'a T {
        assert!(index < self.len);
        let offset = self.get_offset(index) as int;
        unsafe { &*self.ptr.offset(offset) }
    }

    /// Returns a mutable reference to the value at index `index`.
    ///
    /// # Failure
    ///
    /// Fails if `index` is out of bounds
    ///
    /// ```rust
    /// let ringbuf = RingBuff::from_vec(vec!(1, 2, 3));
    /// *ringbuf.get_mut(1) = 4;
    /// assert_eq!(ringbuf.get(1), 4);
    /// ```
    pub fn get_mut<'a>(&'a mut self, index: uint) -> &'a mut T {
        assert!(index < self.len);
        let offset = self.get_offset(index) as int;
        unsafe { &mut *self.ptr.offset(offset) }
    }

    /// Swap elements at indices `i` and `j`
    ///
    /// `i` and `j` may be equal.
    ///
    /// # Failure
    ///
    /// Fails if there is no element with the given index
    pub fn swap(&mut self, i: uint, j: uint) {
        assert!(i < self.len());
        assert!(j < self.len());
        let i_offset = self.get_offset(i) as int;
        let j_offset = self.get_offset(j) as int;
        unsafe {
            ptr::swap(self.ptr.offset(i_offset), self.ptr.offset(j_offset));
        }
    }

    /// Shorten a ring buffer, dropping excess elements.
    ///
    /// If `len` is greater than the ring buffer's current length, this has no
    /// effect.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut ringbuf = RingBuf::from_vec(vec!(1, 2, 3, 4));
    /// vec.truncate(2);
    /// assert_eq!(vec, vec!(1, 2));
    /// ```
    pub fn truncate(&mut self, len: uint) {
        unsafe {
            // drop any extra elements
            while len < self.len {
                // decrement len before the read(), so a failure on Drop doesn't
                // re-drop the just-failed value.
                self.len -= 1;
                let offset = self.get_offset(self.len) as int;
                ptr::read(self.ptr.offset(offset) as *T);
            }
        }
    }

    /// Work with `self` as a pair of slices.
    ///
    /// Either or both slices may be empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// fn foo(slice: &[int]) {}
    ///
    /// let rb = RingBuf::new();
    /// let (slice1, slice2) = rb.as_slices();
    /// ```
    #[inline]
    pub fn as_slices<'a>(&'a self) -> (&'a [T], &'a [T]) {
        let slice1;
        let slice2;
        if self.lo > self.cap - self.len {
            unsafe {
                let len1 = self.cap - self.lo;
                slice1 = mem::transmute(
                    Slice {
                        data: (self.ptr as *T).offset(self.lo as int),
                        len: len1
                    });
                slice2 = mem::transmute(
                    Slice {
                        data: self.ptr as *T,
                        len: self.len - len1
                    });
            }
        } else {
            unsafe {
                slice1 = mem::transmute(
                    Slice {
                        data: (self.ptr as *T).offset(self.lo as int),
                        len: self.len
                    });
                slice2 = mem::transmute(
                    Slice {
                        data: self.ptr as *T,
                        len: 0
                    })
            }
        }
        (slice1, slice2)
    }

    /// Work with `self` as a pair of mutable slices.
    ///
    /// Either or both slices may be empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// let rb = RingBuf::new();
    /// rb.push_front(1);
    /// rb.push_back(2);
    /// let (slice1, slice2) = rb.as_mut_slices();
    /// ```
    #[inline]
    pub fn as_mut_slices<'a>(&'a mut self) -> (&'a mut [T], &'a mut [T]) {
        let slice1;
        let slice2;
        if self.lo > self.cap - self.len {
            unsafe {
                let len1 = self.cap - self.lo;
                slice1 = mem::transmute(
                    Slice {
                        data: (self.ptr as *T).offset(self.lo as int),
                        len: len1
                    });
                slice2 = mem::transmute(
                    Slice {
                        data: self.ptr as *T,
                        len: self.len - len1
                    });
            }
        } else {
            unsafe {
                slice1 = mem::transmute(
                    Slice {
                        data: (self.ptr as *T).offset(self.lo as int),
                        len: self.len
                    });
                slice2 = mem::transmute(
                    Slice {
                        data: self.ptr as *T,
                        len: 0
                    });
            }
        }
        (slice1, slice2)
    }

    /// Returns an iterator over references to the elements of the ring buffer
    /// in order.
    ///
    /// # Example
    ///
    /// ```rust
    /// let ringbuf = RingBuff::from_vec(vec!(1, 2, 3));
    /// for num in ringbuf.iter() {
    ///     println!("{}", *num);
    /// }
    /// ```
    #[inline]
    pub fn iter<'a>(&'a self) -> Items<'a, T> {
        let (slice1, slice2) = self.as_slices();
        slice1.iter().chain(slice2.iter())
    }

    /// Returns an iterator over mutable references to the elements of the
    /// ring buffer in order.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut ringbuf = RingBuff::from_vec(vec!(1, 2, 3));
    /// for num in ringbuf.mut_iter() {
    ///     *num = 0;
    /// }
    /// ```
    #[inline]
    pub fn mut_iter<'a>(&'a mut self) -> MutItems<'a,T> {
        let (slice1, slice2) = self.as_mut_slices();
        slice1.mut_iter().chain(slice2.mut_iter())
    }
}

impl<T> Mutable for RingBuf<T> {
    #[inline]
    fn clear(&mut self) {
        self.truncate(0)
    }
}

impl<T> Deque<T> for RingBuf<T> {

    /// Return a reference to the first element in the `RingBuf`.
    fn front<'a>(&'a self) -> Option<&'a T> {
        if self.len > 0 { Some(self.get(0)) } else { None }
    }

    /// Return a mutable reference to the first element in the `RingBuf`.
    fn front_mut<'a>(&'a mut self) -> Option<&'a mut T> {
        if self.len > 0 { Some(self.get_mut(0)) } else { None }
    }

    /// Return a reference to the last element in the `RingBuf`.
    fn back<'a>(&'a self) -> Option<&'a T> {
        if self.len > 0 { Some(self.get(self.len - 1)) } else { None }
    }

    /// Return a mutable reference to the last element in the `RingBuf`.
    fn back_mut<'a>(&'a mut self) -> Option<&'a mut T> {
        let len = self.len;
        if len > 0 { Some(self.get_mut(len - 1)) } else { None }
    }

    /// Append an element to a ring buffer.
    ///
    /// # Failure
    ///
    /// Fails if the number of elements in the ring buffer overflows a `uint`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut ringbuf = RingBuf::new();
    /// ringbuf.push_back(1);
    /// ```
    #[inline]
    fn push_back(&mut self, value: T) {
        if mem::size_of::<T>() == 0 {
            // zero-size types consume no memory, so we can't rely on the
            // address space running out
            self.len = self.len.checked_add(&1).expect("length overflow");
            unsafe { mem::forget(value); }
            return
        }
        if self.len == self.cap {
            let capacity = self.len * 2;
            self.resize(capacity);
        }

        unsafe {
            let offset = self.get_back_offset() as int;
            let slot = self.ptr.offset(offset);
            ptr::write(slot, value);
            self.len += 1;
        }
    }

    /// Prepend an element to a ring buffer.
    ///
    /// # Failure
    ///
    /// Fails if the number of elements in the ring buffer overflows a `uint`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut ringbuf = RingBuf::new();
    /// ringbuf.push_back(1);
    /// ```
    #[inline]
    fn push_front(&mut self, value: T) {
        if mem::size_of::<T>() == 0 {
            // zero-size types consume no memory,
            // so we can't rely on the address space running out
            self.len = self.len.checked_add(&1).expect("length overflow");
            unsafe { mem::forget(value); }
            return;
        }
        if self.len == self.cap {
            let capacity = self.len * 2;
            self.resize(capacity);
        }

        unsafe {
            let offset = self.get_front_offset();
            let slot = self.ptr.offset(offset as int);
            ptr::write(slot, value);
            self.len += 1;
            self.lo = offset;
        }
    }

    /// Remove the last element from a ring buffer and return it, or `None` if
    /// it is empty.
    #[inline]
    fn pop_back(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            unsafe {
                let offset = self.get_offset(self.len - 1) as int;
                self.len -= 1;
                Some(ptr::read(self.ptr.offset(offset) as *T))
            }
        }
    }

    /// Remove the first element from a ring buffer and return it, or `None` if
    /// it is empty.
    #[inline]
    fn pop_front(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            unsafe {
                let offset = self.get_offset(0) as int;
                self.lo = self.get_offset(1);
                self.len -= 1;
                Some(ptr::read(self.ptr.offset(offset) as *T))
            }
        }
    }
}

impl<T:Clone> Clone for RingBuf<T> {
    fn clone(&self) -> RingBuf<T> {
        let mut ringbuf = RingBuf::with_capacity(self.len);
        // Unsafe code so this can be optimised to a memcpy (or something
        // similarly fast) when T is Copy. LLVM is easily confused, so any
        // extra operations during the loop can prevent this optimisation
        {
            let (slice1, slice2) = self.as_slices();
            while ringbuf.len < slice1.len() {
                unsafe {
                    let len = ringbuf.len;
                    ptr::write(
                        ringbuf.ptr.offset(len as int),
                        slice1.unsafe_ref(len).clone());
                }
                ringbuf.len += 1;
            }
            while ringbuf.len < slice1.len() + slice2.len() {
                unsafe {
                    let len = ringbuf.len;
                    ptr::write(
                        ringbuf.ptr.offset(len as int),
                        slice2.unsafe_ref(len - slice1.len()).clone());
                }
                ringbuf.len += 1;
            }
        }
        ringbuf
    }

    fn clone_from(&mut self, source: &RingBuf<T>) {
        // drop anything in self that will not be overwritten
        if self.len() > source.len() {
            self.truncate(source.len())
        }

        // reuse the contained values' allocations/resources.
        for (place, thing) in self.mut_iter().zip(source.iter()) {
            place.clone_from(thing)
        }

        // self.len <= source.len due to the truncate above, so the
        // slice here is always in-bounds.
        let len = self.len();
        self.extend(source.iter().skip(len).map(|x| x.clone()));
    }
}

impl<T> FromIterator<T> for RingBuf<T> {
    fn from_iter<I:Iterator<T>>(mut iterator: I) -> RingBuf<T> {
        let (lower, _) = iterator.size_hint();
        let mut ringbuf = RingBuf::with_capacity(lower);
        for element in iterator {
            ringbuf.push_back(element)
        }
        ringbuf
    }
}

impl<T> Extendable<T> for RingBuf<T> {
    fn extend<I: Iterator<T>>(&mut self, mut iterator: I) {
        let len = self.len;
        let (lower, _) = iterator.size_hint();
        self.reserve_additional(len + lower);
        for element in iterator {
            self.push_back(element)
        }
    }
}

impl<T> Collection for RingBuf<T> {
    #[inline]
    fn len(&self) -> uint {
        self.len
    }
}

/// Allocate a buffer with the provided capacity.
// FIXME: #13996: need a way to mark the return value as `noalias`
#[inline(never)]
unsafe fn alloc<T>(capacity: uint) -> *mut T {
    let size = capacity.checked_mul(&mem::size_of::<T>())
                       .expect("capacity overflow");
    allocate(size, mem::min_align_of::<T>()) as *mut T
}

/// Deallocate a buffer of the provided capacity.
#[inline]
unsafe fn dealloc<T>(ptr: *mut T, capacity: uint) {
    if mem::size_of::<T>() != 0 {
        deallocate(ptr as *mut u8,
                   capacity * mem::size_of::<T>(),
                   mem::min_align_of::<T>())
    }
}

impl<T> RingBuf<T> {
    /// Returns the number of elements the ringbuf can hold without
    /// reallocating.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::collections::RingBuf;
    /// let ringbuf: RingBuf<int> = RingBuf::with_capacity(10);
    /// assert_eq!(ringbuf.capacity(), 10);
    /// ```
    #[inline]
    pub fn capacity(&self) -> uint {
        self.cap
    }

    /// Reserves capacity for at least `n` additional elements in the given
    /// ring buffer.
    ///
    /// # Failure
    ///
    /// Fails if the new capacity overflows `uint`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use collections::RingBuf;
    /// let mut ringbuf: RingBuf<int> = RingBuf::with_capacity(1);
    /// ringbuf.reserve_additional(10);
    /// assert!(ringbuf.capacity() >= 11);
    /// ```
    pub fn reserve_additional(&mut self, extra: uint) {
        if self.cap - self.len < extra {
            let size = self.len.checked_add(&extra).expect("length overflow");
            self.reserve(size);
        }
    }

    /// Reserves capacity for at least `n` elements in the given ring buffer.
    ///
    /// This function will over-allocate in order to amortize the allocation
    /// costs in scenarios where the caller may need to repeatedly reserve
    /// additional space.
    ///
    /// If the capacity for `self` is already equal to or greater than the
    /// requested capacity, then no action is taken.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use collections::RingBuf;
    /// let mut ringbuf = RingBuf::new();
    /// ringbuf.reserve(10);
    /// assert!(ringbuf.capacity() >= 10);
    /// ```
    pub fn reserve(&mut self, capacity: uint) {
        self.reserve_exact(num::next_power_of_two(capacity))
    }

    /// Reserves capacity for exactly `capacity` elements in the given ring
    /// buffer.
    ///
    /// If the capacity for `self` is already equal to or greater than the
    /// requested capacity, then no action is taken.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use collections::RingBuf;
    /// let mut ringbuf: RingBuf<int> = RingBuf::with_capacity(10);
    /// ringbuf.reserve_exact(11);
    /// assert_eq!(ringbuf.capacity(), 11);
    /// ```
    pub fn reserve_exact(&mut self, capacity: uint) {
        if capacity > self.cap {
            self.resize(capacity);
        }
    }

    /// Shrink the capacity of the ring buffer as much as possible
    ///
    /// # Example
    ///
    /// ```rust
    /// # use collections::RingBuf;
    /// let mut ringbuf = RingBuf::new();
    /// ringbuf.push_back(1);
    /// ringbuf.shrink_to_fit();
    /// ```
    pub fn shrink_to_fit(&mut self) {
        let len = self.len;
        self.resize(len);
    }
}

impl<T> RingBuf<T> {
    /// Resize the `RingBuf` to the specified capacity.
    ///
    /// # Failure
    ///
    /// Fails if the requested capacity overflows a `uint`, or if
    /// the number of elements in the ring buffer is greater than the
    /// requested capacity.
    fn resize(&mut self, capacity: uint) {
        debug_assert!(capacity >= self.len, "capacity underflow");
        if capacity == self.cap { return }
        if mem::size_of::<T>() == 0 { return }

        let ptr;
        unsafe {
            let (slice1, slice2) = self.as_slices();
            ptr = alloc::<T>(capacity) as *mut T;
            let len1 = slice1.len();
            ptr::copy_nonoverlapping_memory(ptr, slice1.as_ptr(), len1);
            ptr::copy_nonoverlapping_memory(ptr.offset(len1 as int),
                                            slice2.as_ptr(),
                                            slice2.len());
            dealloc(self.ptr, self.cap);
        }

        self.ptr = ptr;
        self.cap = capacity;
        self.lo = 0;
    }

    /// Return the offset of the next back slot
    #[inline]
    fn get_back_offset(&self) -> uint {
        self.get_offset(self.len)
    }

    /// Return the offset of the next front slot
    #[inline]
    fn get_front_offset(&self) -> uint {
        if self.lo == 0 {
            self.cap - 1
        } else {
            self.lo - 1
        }
    }

    /// Return the offset of the given index in the underlying buffer.
    #[inline]
    fn get_offset(&self, index: uint) -> uint {
        // The order of these operations preserves numerical stability
        if self.lo >= self.cap - index {
            index - (self.cap - self.lo)
        } else {
            self.lo + index
        }
    }

    /// Reset the `lo` index to 0. This may require copying and temporary
    /// allocation.
    fn reset(&mut self) {
        if self.lo == 0 { return }

        // Shift elements to start of buffer
        {
            // `slice1` begins at the `lo` index.
            // `slice2` begins at the `0` index.
            let (slice1, slice2) = self.as_slices();
            let len1 = slice1.len();
            let len2 = slice2.len();

            if len1 == 0 {
                // Nothing to do
            } if len2 == 0 {
                // The buffer does not wrap. Move slice1.
                //
                //   lo
                //    V
                // +-+-+-+-+-+-+-+
                // | |x|x|x|x|x| |
                // +-+-+-+-+-+-+-+
                unsafe {
                    ptr::copy_memory(self.ptr,
                                     self.ptr.offset(self.lo as int) as *T,
                                     self.len);
                }

            } if len1 <= (self.cap - len1) - len2 {
                // There is sufficient space to move slice2 without overwriting
                // slice1.
                //
                //           lo
                //            V
                // +-+-+-+-+-+-+-+
                // |x|x|x| | |x|x|
                // +-+-+-+-+-+-+-+
                unsafe {
                    ptr::copy_memory(self.ptr.offset(slice1.len() as int),
                                     slice2.as_ptr(),
                                     slice2.len());
                    ptr::copy_memory(self.ptr,
                                     slice1.as_ptr(),
                                     slice1.len());
                }
            } else if len1 < len2 {
                // Copy slice1 and move slice2.
                //
                //           lo
                //            V
                // +-+-+-+-+-+-+-+
                // |x|x|x|x| |x|x|
                // +-+-+-+-+-+-+-+
                unsafe {
                    let tmp = alloc(len1);
                    ptr::copy_nonoverlapping_memory(tmp,
                                                    slice1.as_ptr(),
                                                    len1);
                    ptr::copy_memory(self.ptr.offset(len1 as int),
                                     slice2.as_ptr(),
                                     len2);
                    ptr::copy_nonoverlapping_memory(self.ptr,
                                                    tmp as *T,
                                                    len1);
                    dealloc(tmp, len1);
                }
            } else {
                // Copy slice2 and move slice1.
                //
                //         lo
                //          V
                // +-+-+-+-+-+-+-+
                // |x|x| | |x|x|x|
                // +-+-+-+-+-+-+-+
                unsafe {
                    let tmp = alloc(len2);
                    ptr::copy_nonoverlapping_memory(tmp,
                                                    slice2.as_ptr(),
                                                    len2);
                    ptr::copy_memory(self.ptr,
                                     slice1.as_ptr(),
                                     len1);
                    ptr::copy_nonoverlapping_memory(self.ptr.offset(len1 as int),
                                                    tmp as *T,
                                                    len2);
                    dealloc(tmp, len2);
                }
            }
        }
        self.lo = 0;
    }
}

impl<T: PartialEq> PartialEq for RingBuf<T> {
    #[inline]
    fn eq(&self, other: &RingBuf<T>) -> bool {
        self.len == other.len
            && self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl<T: PartialOrd> PartialOrd for RingBuf<T> {
    #[inline]
    fn lt(&self, other: &RingBuf<T>) -> bool {
        self.iter().zip(other.iter()).all(|(a, b)| a.lt(b))
            && self.len < other.len
    }
}

impl<T: Eq> Eq for RingBuf<T> {}

impl<T: Ord> Ord for RingBuf<T> {
    #[inline]
    fn cmp(&self, other: &RingBuf<T>) -> Ordering {
        for (a, b) in self.iter().zip(other.iter()) {
            let cmp = a.cmp(b);
            if cmp != Equal {
                return cmp;
            }
        }
        self.len.cmp(&other.len)
    }
}

impl<T: fmt::Show> fmt::Show for RingBuf<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "["));

        for (i, e) in self.iter().enumerate() {
            if i != 0 { try!(write!(f, ", ")); }
            try!(write!(f, "{}", *e));
        }

        write!(f, "]")
    }
}

#[unsafe_destructor]
impl<T> Drop for RingBuf<T> {
    fn drop(&mut self) {
        if self.cap != 0 {
            unsafe {
                for x in self.iter() {
                    ptr::read(x);
                }

                dealloc(self.ptr, self.cap)
            }
        }
    }
}


#[cfg(test)]
mod checks {
    extern crate quickcheck;

    use quickcheck::quickcheck;
    use std::collections::Deque;
    use super::RingBuf;

    #[test]
    fn check_from_into_vec() {
        fn prop(items: Vec<int>) -> bool {
            items == RingBuf::from_vec(items.clone()).into_vec()
        }

        quickcheck(prop);
    }

    #[test]
    fn check_clone() {
        fn prop(items: Vec<int>) -> bool {
            let rb = RingBuf::from_vec(items);

            rb == rb.clone()
        }

        quickcheck(prop);
    }

    #[test]
    fn check_push_back() {
        fn prop(items: Vec<int>, cap: uint) -> bool {
            let mut rb = RingBuf::with_capacity(cap);

            for &i in items.iter() {
                rb.push_back(i);
            }

            RingBuf::from_vec(items) == rb
        }

        quickcheck(prop);
    }

    #[test]
    fn check_push_front() {
        fn prop(mut items: Vec<int>, cap: uint) -> bool {
            let mut rb = RingBuf::with_capacity(cap);

            for &i in items.iter() {
                rb.push_front(i);
            }
            items.reverse();

            RingBuf::from_vec(items) == rb
        }

        quickcheck(prop);
    }

    #[test]
    fn check_push_mixed() {
        fn prop(items: Vec<(bool, int)>, cap: uint) -> bool {
            let mut rb = RingBuf::with_capacity(cap);

            let mut fronts = Vec::new();
            let mut backs = Vec::new();

            for &(is_front, i) in items.iter() {
                if is_front {
                    fronts.push(i);
                    rb.push_front(i);
                } else {
                    backs.push(i);
                    rb.push_back(i);
                }
            }
            fronts.reverse();
            fronts.push_all(backs.as_slice());

            RingBuf::from_vec(fronts) == rb
        }

        quickcheck(prop);
    }

    #[test]
    fn check_pop_front() {
        fn prop(items: Vec<int>) -> bool {
            let mut rb = RingBuf::from_vec(items.clone());
            items.iter().all(|i| i == rb.front().unwrap() && i == &rb.pop_front().unwrap())
        }

        quickcheck(prop);
    }

    #[test]
    fn check_pop_back() {
        fn prop(mut items: Vec<int>) -> bool {
            let mut rb = RingBuf::from_vec(items.clone());
            items.reverse();
            items.iter().all(|i| i == rb.back().unwrap() && i == &rb.pop_back().unwrap())
        }

        quickcheck(prop);
    }
}


#[cfg(test)]
mod tests {

    use std::collections::Deque;
    use super::RingBuf;

    #[test]
    fn test_push_back() {
        let mut rb = RingBuf::new();
        let items = vec!(1, 2, 3, 4, 5, 6, 7, 8);

        for &i in items.iter() {
            rb.push_back(i);
        }

        let (slice1, slice2) = rb.as_slices();

        assert!(slice1 == items.as_slice());
        assert!(slice2.is_empty());
    }

    #[test]
    fn test_push_front() {
        let mut rb = RingBuf::with_capacity(0);
        let mut items = vec!(1, 2, 3);

        for &i in items.iter() {
            rb.push_front(i);
        }
        items.reverse();

        assert!(items == rb.iter().map(|&x| x).collect());
    }

    #[test]
    fn test_push_mixed() {
        let mut rb = RingBuf::with_capacity(1);

        rb.push_back(4);
        println!("rb: {}, {:?}", rb, rb);
        rb.push_back(5);
        println!("rb: {}, {:?}", rb, rb);
        rb.push_front(3);
        println!("rb: {}, {:?}", rb, rb);
        rb.push_back(6);
        println!("rb: {}, {:?}", rb, rb);
        rb.push_front(2);
        println!("rb: {}, {:?}", rb, rb);
        rb.push_front(1);
        println!("rb: {}, {:?}", rb, rb);

        let vec: Vec<int> = rb.iter().map(|&x| x).collect();
        assert!(vec == vec!(1, 2, 3, 4, 5, 6));
    }
}

#[cfg(test)]
mod libtests {
    extern crate log;

    use std::fmt::Show;
    use std::gc::{GC, Gc};
    use test::Bencher;
    use test;

    use std::collections::{Deque, Mutable};
    use super::RingBuf;
    use std::vec::Vec;

    #[test]
    fn test_simple() {
        let mut d = RingBuf::new();
        assert_eq!(d.len(), 0u);
        d.push_front(17);
        d.push_front(42);
        d.push_back(137);
        assert_eq!(d.len(), 3u);
        d.push_back(137);
        assert_eq!(d.len(), 4u);
        assert_eq!(*d.front().unwrap(), 42);
        assert_eq!(*d.back().unwrap(), 137);
        let mut i = d.pop_front();
        assert_eq!(i, Some(42));
        i = d.pop_back();
        assert_eq!(i, Some(137));
        i = d.pop_back();
        assert_eq!(i, Some(137));
        i = d.pop_back();
        assert_eq!(i, Some(17));
        assert_eq!(d.len(), 0u);
        d.push_back(3);
        assert_eq!(d.len(), 1u);
        d.push_front(2);
        assert_eq!(d.len(), 2u);
        d.push_back(4);
        assert_eq!(d.len(), 3u);
        d.push_front(1);
        assert_eq!(d.len(), 4u);
        assert_eq!(*d.get(0), 1);
        assert_eq!(*d.get(1), 2);
        assert_eq!(*d.get(2), 3);
        assert_eq!(*d.get(3), 4);
    }

    #[test]
    fn test_boxes() {
        let a: Gc<int> = box(GC) 5;
        let b: Gc<int> = box(GC) 72;
        let c: Gc<int> = box(GC) 64;
        let d: Gc<int> = box(GC) 175;

        let mut deq = RingBuf::new();
        assert_eq!(deq.len(), 0);
        deq.push_front(a);
        deq.push_front(b);
        deq.push_back(c);
        assert_eq!(deq.len(), 3);
        deq.push_back(d);
        assert_eq!(deq.len(), 4);
        assert_eq!(deq.front(), Some(&b));
        assert_eq!(deq.back(), Some(&d));
        assert_eq!(deq.pop_front(), Some(b));
        assert_eq!(deq.pop_back(), Some(d));
        assert_eq!(deq.pop_back(), Some(c));
        assert_eq!(deq.pop_back(), Some(a));
        assert_eq!(deq.len(), 0);
        deq.push_back(c);
        assert_eq!(deq.len(), 1);
        deq.push_front(b);
        assert_eq!(deq.len(), 2);
        deq.push_back(d);
        assert_eq!(deq.len(), 3);
        deq.push_front(a);
        assert_eq!(deq.len(), 4);
        assert_eq!(*deq.get(0), a);
        assert_eq!(*deq.get(1), b);
        assert_eq!(*deq.get(2), c);
        assert_eq!(*deq.get(3), d);
    }

    #[cfg(test)]
    fn test_parameterized<T:Clone + PartialEq + Show>(a: T, b: T, c: T, d: T) {
        let mut deq = RingBuf::new();
        assert_eq!(deq.len(), 0);
        deq.push_front(a.clone());
        deq.push_front(b.clone());
        deq.push_back(c.clone());
        assert_eq!(deq.len(), 3);
        deq.push_back(d.clone());
        assert_eq!(deq.len(), 4);
        assert_eq!((*deq.front().unwrap()).clone(), b.clone());
        assert_eq!((*deq.back().unwrap()).clone(), d.clone());
        assert_eq!(deq.pop_front().unwrap(), b.clone());
        assert_eq!(deq.pop_back().unwrap(), d.clone());
        assert_eq!(deq.pop_back().unwrap(), c.clone());
        assert_eq!(deq.pop_back().unwrap(), a.clone());
        assert_eq!(deq.len(), 0);
        deq.push_back(c.clone());
        assert_eq!(deq.len(), 1);
        deq.push_front(b.clone());
        assert_eq!(deq.len(), 2);
        deq.push_back(d.clone());
        assert_eq!(deq.len(), 3);
        deq.push_front(a.clone());
        assert_eq!(deq.len(), 4);
        assert_eq!((*deq.get(0)).clone(), a.clone());
        assert_eq!((*deq.get(1)).clone(), b.clone());
        assert_eq!((*deq.get(2)).clone(), c.clone());
        assert_eq!((*deq.get(3)).clone(), d.clone());
    }

    #[test]
    fn test_push_front_grow() {
        let mut deq = RingBuf::new();
        for i in range(0u, 66) {
            deq.push_front(i);
        }
        assert_eq!(deq.len(), 66);

        for i in range(0u, 66) {
            assert_eq!(*deq.get(i), 65 - i);
        }

        let mut deq = RingBuf::new();
        for i in range(0u, 66) {
            deq.push_back(i);
        }

        for i in range(0u, 66) {
            assert_eq!(*deq.get(i), i);
        }
    }

    #[bench]
    fn bench_new(b: &mut test::Bencher) {
        b.iter(|| {
            let _: RingBuf<u64> = RingBuf::new();
        })
    }

    #[bench]
    fn bench_push_back(b: &mut test::Bencher) {
        let mut deq = RingBuf::new();
        b.iter(|| {
            deq.push_back(0);
        })
    }

    #[bench]
    fn bench_push_front(b: &mut test::Bencher) {
        let mut deq = RingBuf::new();
        b.iter(|| {
            deq.push_front(0);
        })
    }

    #[bench]
    fn bench_grow(b: &mut test::Bencher) {
        let mut deq = RingBuf::new();
        b.iter(|| {
            for _ in range(0, 65) {
                deq.push_front(1);
            }
        })
    }

    #[deriving(Clone, PartialEq, Show)]
    enum Taggy {
        One(int),
        Two(int, int),
        Three(int, int, int),
    }

    #[deriving(Clone, PartialEq, Show)]
    enum Taggypar<T> {
        Onepar(int),
        Twopar(int, int),
        Threepar(int, int, int),
    }

    #[deriving(Clone, PartialEq, Show)]
    struct RecCy {
        x: int,
        y: int,
        t: Taggy
    }

    #[test]
    fn test_param_int() {
        test_parameterized::<int>(5, 72, 64, 175);
    }

    #[test]
    fn test_param_at_int() {
        test_parameterized::<Gc<int>>(box(GC) 5, box(GC) 72,
                                      box(GC) 64, box(GC) 175);
    }

    #[test]
    fn test_param_taggy() {
        test_parameterized::<Taggy>(One(1), Two(1, 2), Three(1, 2, 3), Two(17, 42));
    }

    #[test]
    fn test_param_taggypar() {
        test_parameterized::<Taggypar<int>>(Onepar::<int>(1),
                                            Twopar::<int>(1, 2),
                                            Threepar::<int>(1, 2, 3),
                                            Twopar::<int>(17, 42));
    }

    #[test]
    fn test_param_reccy() {
        let reccy1 = RecCy { x: 1, y: 2, t: One(1) };
        let reccy2 = RecCy { x: 345, y: 2, t: Two(1, 2) };
        let reccy3 = RecCy { x: 1, y: 777, t: Three(1, 2, 3) };
        let reccy4 = RecCy { x: 19, y: 252, t: Two(17, 42) };
        test_parameterized::<RecCy>(reccy1, reccy2, reccy3, reccy4);
    }

    #[test]
    fn test_with_capacity() {
        let mut d = RingBuf::with_capacity(0);
        d.push_back(1);
        assert_eq!(d.len(), 1);
        let mut d = RingBuf::with_capacity(50);
        d.push_back(1);
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn test_reserve_exact() {
        let mut d = RingBuf::new();
        d.push_back(0u64);
        d.reserve_exact(50);
        assert_eq!(d.cap, 50);
        let mut d = RingBuf::new();
        d.push_back(0u32);
        d.reserve_exact(50);
        assert_eq!(d.cap, 50);
    }

    #[test]
    fn test_reserve() {
        let mut d = RingBuf::new();
        d.push_back(0u64);
        d.reserve(50);
        assert_eq!(d.cap, 64);
        let mut d = RingBuf::new();
        d.push_back(0u32);
        d.reserve(50);
        assert_eq!(d.cap, 64);
    }

    #[test]
    fn test_swap() {
        let mut d: RingBuf<int> = range(0, 5).collect();
        d.pop_front();
        d.swap(0, 3);
        assert_eq!(d.iter().map(|&x|x).collect::<Vec<int>>(), vec!(4, 2, 3, 1));
    }

    #[test]
    fn test_iter() {
        let mut d = RingBuf::new();
        assert_eq!(d.iter().next(), None);
        assert_eq!(d.iter().size_hint(), (0, Some(0)));

        for i in range(0, 5) {
            d.push_back(i);
        }
        assert_eq!(d.iter().collect::<Vec<&int>>().as_slice(), &[&0,&1,&2,&3,&4]);

        for i in range(6, 9) {
            d.push_front(i);
        }
        assert_eq!(d.iter().collect::<Vec<&int>>().as_slice(), &[&8,&7,&6,&0,&1,&2,&3,&4]);

        let mut it = d.iter();
        let mut len = d.len();
        loop {
            match it.next() {
                None => break,
                _ => { len -= 1; assert_eq!(it.size_hint(), (len, Some(len))) }
            }
        }
    }

    #[test]
    fn test_rev_iter() {
        let mut d = RingBuf::new();
        assert_eq!(d.iter().rev().next(), None);

        for i in range(0, 5) {
            d.push_back(i);
        }
        assert_eq!(d.iter().rev().collect::<Vec<&int>>().as_slice(), &[&4,&3,&2,&1,&0]);

        for i in range(6, 9) {
            d.push_front(i);
        }
        assert_eq!(d.iter().rev().collect::<Vec<&int>>().as_slice(), &[&4,&3,&2,&1,&0,&6,&7,&8]);
    }

    #[test]
    fn test_mut_rev_iter_wrap() {
        let mut d = RingBuf::with_capacity(3);
        assert!(d.mut_iter().rev().next().is_none());

        d.push_back(1);
        d.push_back(2);
        d.push_back(3);
        assert_eq!(d.pop_front(), Some(1));
        d.push_back(4);

        assert_eq!(d.mut_iter().rev().map(|x| *x).collect::<Vec<int>>(),
                   vec!(4, 3, 2));
    }

    #[test]
    fn test_mut_iter() {
        let mut d = RingBuf::new();
        assert!(d.mut_iter().next().is_none());

        for i in range(0u, 3) {
            d.push_front(i);
        }

        for (i, elt) in d.mut_iter().enumerate() {
            assert_eq!(*elt, 2 - i);
            *elt = i;
        }

        {
            let mut it = d.mut_iter();
            assert_eq!(*it.next().unwrap(), 0);
            assert_eq!(*it.next().unwrap(), 1);
            assert_eq!(*it.next().unwrap(), 2);
            assert!(it.next().is_none());
        }
    }

    #[test]
    fn test_mut_rev_iter() {
        let mut d = RingBuf::new();
        assert!(d.mut_iter().rev().next().is_none());

        for i in range(0u, 3) {
            d.push_front(i);
        }

        for (i, elt) in d.mut_iter().rev().enumerate() {
            assert_eq!(*elt, i);
            *elt = i;
        }

        {
            let mut it = d.mut_iter().rev();
            assert_eq!(*it.next().unwrap(), 0);
            assert_eq!(*it.next().unwrap(), 1);
            assert_eq!(*it.next().unwrap(), 2);
            assert!(it.next().is_none());
        }
    }

    #[test]
    fn test_from_iter() {
        use std::iter;
        let v = vec!(1,2,3,4,5,6,7);
        let deq: RingBuf<int> = v.iter().map(|&x| x).collect();
        let u: Vec<int> = deq.iter().map(|&x| x).collect();
        assert_eq!(u, v);

        let mut seq = iter::count(0u, 2).take(256);
        let deq: RingBuf<uint> = seq.collect();
        for (i, &x) in deq.iter().enumerate() {
            assert_eq!(2*i, x);
        }
        assert_eq!(deq.len(), 256);
    }

    #[test]
    fn test_clone() {
        let mut d = RingBuf::new();
        d.push_front(17);
        d.push_front(42);
        d.push_back(137);
        d.push_back(137);
        println!("d:         {}", d);
        println!("d.clone(): {}", d.clone());
        assert_eq!(d.len(), 4u);
        let mut e = d.clone();
        assert_eq!(e.len(), 4u);
        while !d.is_empty() {
            assert_eq!(d.pop_back(), e.pop_back());
        }
        assert_eq!(d.len(), 0u);
        assert_eq!(e.len(), 0u);
    }

    #[test]
    fn test_eq() {
        let mut d = RingBuf::new();
        assert!(d == RingBuf::with_capacity(0));
        d.push_front(137);
        d.push_front(17);
        d.push_front(42);
        d.push_back(137);
        let mut e = RingBuf::with_capacity(0);
        e.push_back(42);
        e.push_back(17);
        e.push_back(137);
        e.push_back(137);
        assert!(&e == &d);
        e.pop_back();
        e.push_back(0);
        assert!(e != d);
        e.clear();
        assert!(e == RingBuf::new());
    }

    #[test]
    fn test_show() {
        let ringbuf: RingBuf<int> = range(0, 10).collect();
        assert!(format!("{}", ringbuf).as_slice() == "[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]");

        let ringbuf: RingBuf<&str> = vec!["just", "one", "test", "more"].iter()
                                                                        .map(|&s| s)
                                                                        .collect();
        assert!(format!("{}", ringbuf).as_slice() == "[just, one, test, more]");
    }
}
