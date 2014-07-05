// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
#![feature(unsafe_destructor)]

extern crate quickcheck;

///! A double-ended queue implemented as a circular buffer
///!
///! RingBuf implements the trait Deque. It should be imported with
///! `use collections::Deque`.

use std::cmp;
use std::collections::Deque;
use std::default::Default;
use std::fmt;
use std::iter::Chain;
use std::iter::FromIterator;
use std::mem;
use std::num;
use std::ptr;
use std::raw::Slice;
use std::rt::heap::{allocate, deallocate};
use std::slice;
use std::uint;

/// RingBuf is a circular buffer that implements Deque.
///
/// # Examples
///
/// ```rust
/// # use std::collections::{RingBuf, Deque};
/// let mut ringbuf = RingBuf::new();
/// ringbuf.push_front(1i);
/// ringbuf.push_back(2);
///
/// assert_eq!(ringbuf.len(), 2);
/// assert_eq!(ringbuf.get(0), &1);
/// assert_eq!(ringbuf.front(), Some(&1));
/// assert_eq!(ringbuf.back(), Some(&2));
///
/// assert_eq!(ringbuf.pop_back(), Some(2));
/// assert_eq!(ringbuf.len(), 1);
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
    /// The ring buffer will not allocate until elements are pushed onto it.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::collections::RingBuf;
    /// let mut ringbuf: RingBuf<int> = RingBuf::new();
    /// ```
    pub fn new() -> RingBuf<T> {
        RingBuf::with_capacity(0)
    }

    /// Constructs a new, empty `RingBuf` with the specified capacity.
    ///
    /// The ring will be able to hold exactly `capacity` elements without
    /// reallocating. If `capacity` is 0, the ringbuf will not allocate.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::collections::RingBuf;
    /// let ring: RingBuf<int> = RingBuf::with_capacity(10);
    /// ```
    pub fn with_capacity(capacity: uint) -> RingBuf<T> {
        if mem::size_of::<T>() == 0 {
            RingBuf { lo: 0, len: 0, cap: uint::MAX, ptr: 0 as *mut T }
        } else if capacity == 0 {
            RingBuf { lo: 0, len: 0, cap: 0, ptr: 0 as *mut T }
        } else {
            let ptr: *mut T = unsafe { alloc(capacity) };
            RingBuf { lo: 0, len: 0, cap: capacity, ptr: ptr }
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
    /// # use std::collections::RingBuf;
    /// let mut vec = vec![1i, 2, 3];
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
    /// # use std::collections::{RingBuf, Deque};
    /// let mut ringbuf = RingBuf::new();
    /// ringbuf.push_front(1i);
    /// ringbuf.push_back(2);
    /// let vec = ringbuf.into_vec();
    /// assert_eq!(&[1, 2], vec.as_slice());
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
    /// # use std::collections::RingBuf;
    /// let ringbuf = RingBuf::from_vec(vec![1i, 2, 3]);
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
    /// # use std::collections::RingBuf;
    /// let mut ringbuf = RingBuf::from_vec(vec![1i, 2, 3]);
    /// *ringbuf.get_mut(1) = 4;
    /// assert_eq!(ringbuf.get(1), &4);
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

    /// Shorten a ring buffer, dropping excess elements from the end.
    ///
    /// If `len` is greater than the ring buffer's current length, this has no
    /// effect.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::collections::RingBuf;
    /// let mut ringbuf = RingBuf::from_vec(vec![1i, 2, 3, 4]);
    /// ringbuf.truncate(2);
    /// assert_eq!(ringbuf.into_vec(), vec![1i, 2]);
    /// ```
    pub fn truncate(&mut self, len: uint) {
        for _ in range(len, self.len) { self.pop_back(); }
    }

    /// Work with `self` as a pair of slices.
    ///
    /// Either or both slices may be empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::collections::{RingBuf, Deque};
    /// let mut rb = RingBuf::new();
    /// rb.push_back(1i);
    /// rb.push_front(0);
    /// let (slice1, slice2) = rb.as_slices();
    /// assert_eq!(slice1, &[0]);
    /// assert_eq!(slice2, &[1]);
    /// ```
    #[inline]
    pub fn as_slices<'a>(&'a self) -> (&'a [T], &'a [T]) {
        let (ptr1, len1, ptr2, len2) = self.get_slice_ptrs();
        unsafe {
            (mem::transmute(Slice { data: ptr1, len: len1 }),
             mem::transmute(Slice { data: ptr2, len: len2 }))
        }
    }

    /// Work with `self` as a pair of mutable slices.
    ///
    /// Either or both slices may be empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::collections::{RingBuf, Deque};
    /// let mut rb = RingBuf::new();
    /// rb.push_front(1i);
    /// rb.push_back(2);
    /// let (slice1, slice2) = rb.as_mut_slices();
    /// assert_eq!(slice1, &[1]);
    /// assert_eq!(slice2, &[2]);
    /// ```
    #[inline]
    pub fn as_mut_slices<'a>(&'a mut self) -> (&'a mut [T], &'a mut [T]) {
        let (ptr1, len1, ptr2, len2) = self.get_slice_ptrs();
        unsafe {
            (mem::transmute(Slice { data: ptr1, len: len1 }),
             mem::transmute(Slice { data: ptr2, len: len2 }))
        }
    }

    /// Returns an iterator over references to the elements of the ring buffer
    /// in order.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::collections::RingBuf;
    /// let ringbuf = RingBuf::from_vec(vec![1i, 2, 3]);
    /// for &num in ringbuf.iter() {
    ///     println!("{}", num);
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
    /// # use std::collections::RingBuf;
    /// let mut ringbuf = RingBuf::from_vec(vec![1i, 2, 3]);
    /// for num in ringbuf.mut_iter() {
    ///     *num = 0;
    /// }
    /// ```
    #[inline]
    pub fn mut_iter<'a>(&'a mut self) -> MutItems<'a,T> {
        let (slice1, slice2) = self.as_mut_slices();
        slice1.mut_iter().chain(slice2.mut_iter())
    }


    /// Creates a consuming iterator, that is, one that moves each
    /// value out of the ringbuf (from front to back).
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::collections::{RingBuf, Deque};
    /// let mut rb = RingBuf::new();
    /// rb.push_back("a".to_string());
    /// rb.push_back("b".to_string());
    /// for s in rb.move_iter() {
    ///     // s has type String, not &String
    ///     println!("{}", s);
    /// }
    /// ```
    #[inline]
    pub fn move_iter(self) -> MoveItems<T> {
        unsafe {
            let iter = mem::transmute(self.iter());
            let ptr = self.ptr;
            let cap = self.cap;
            mem::forget(self);
            MoveItems { allocation: ptr, cap: cap, iter: iter }
        }
    }

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
    /// # use std::collections::RingBuf;
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
    /// # use std::collections::RingBuf;
    /// let mut ringbuf = RingBuf::<int>::new();
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
    /// # use std::collections::RingBuf;
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
    /// # use std::collections::{RingBuf, Deque};
    /// let mut ringbuf = RingBuf::new();
    /// ringbuf.push_back(1i);
    /// ringbuf.shrink_to_fit();
    /// ```
    pub fn shrink_to_fit(&mut self) {
        let len = self.len;
        self.resize(len);
    }
}

impl<T> Collection for RingBuf<T> {
    #[inline]
    fn len(&self) -> uint {
        self.len
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
    /// # use std::collections::{RingBuf, Deque};
    /// let mut ringbuf = RingBuf::new();
    /// ringbuf.push_back(1i);
    /// assert_eq!(Some(&1), ringbuf.back());
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
            let capacity = cmp::max(self.len, 1) * 2;
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
    /// # use std::collections::{RingBuf, Deque};
    /// let mut ringbuf = RingBuf::new();
    /// ringbuf.push_back(1i);
    /// assert_eq!(Some(&1), ringbuf.front());
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
            let capacity = cmp::max(self.len, 1) * 2;
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
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::collections::{RingBuf, Deque};
    /// let mut ringbuf = RingBuf::new();
    /// ringbuf.push_back(1i);
    /// assert_eq!(Some(1), ringbuf.pop_back());
    /// assert_eq!(None, ringbuf.pop_back());
    /// ```
    #[inline]
    fn pop_back(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            unsafe {
                let offset = self.get_offset(self.len - 1) as int;
                self.len -= 1;
                Some(ptr::read(self.ptr.offset(offset) as *const T))
            }
        }
    }

    /// Remove the first element from a ring buffer and return it, or `None` if
    /// it is empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::collections::{RingBuf, Deque};
    /// let mut ringbuf = RingBuf::new();
    /// ringbuf.push_back(1i);
    /// assert_eq!(Some(1), ringbuf.pop_front());
    /// assert_eq!(None, ringbuf.pop_front());
    /// ```
    #[inline]
    fn pop_front(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            unsafe {
                let offset = self.get_offset(0) as int;
                self.lo = self.get_offset(1);
                self.len -= 1;
                Some(ptr::read(self.ptr.offset(offset) as *const T))
            }
        }
    }
}

impl<T> Default for RingBuf<T> {
    #[inline]
    fn default() -> RingBuf<T> { RingBuf::new() }
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
        RingBuf::from_vec(iterator.collect())
    }
}

impl<T> Extendable<T> for RingBuf<T> {
    fn extend<I: Iterator<T>>(&mut self, mut iterator: I) {
        let (lower, _) = iterator.size_hint();
        self.reserve_additional(lower);
        for element in iterator {
            self.push_back(element)
        }
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

    /// Calculates the start and length of the slices in this ringbuf.
    #[inline]
    fn get_slice_ptrs(&self) -> (*const T, uint, *const T, uint) {
        let ptr1;
        let ptr2;
        let len1;
        let len2;
        unsafe {
            if self.lo > self.cap - self.len {
                ptr1 = self.ptr.offset(self.lo as int);
                ptr2 = self.ptr;
                len1 = self.cap - self.lo;
                len2 = self.len - len1;
            } else {
                ptr1 = self.ptr.offset(self.lo as int);
                ptr2 = self.ptr;
                len1 = self.len;
                len2 = 0;
            }
        }
        (ptr1 as *const T, len1, ptr2 as *const T, len2)
    }

    /// Resize the `RingBuf` to the specified capacity.
    ///
    /// # Failure
    ///
    /// Fails if the number of elements in the ring buffer is greater than
    /// the requested capacity.
    fn resize(&mut self, capacity: uint) {
        assert!(capacity >= self.len, "capacity underflow");

        if capacity == self.cap { return }
        if mem::size_of::<T>() == 0 { return }

        let ptr;
        unsafe {
            if capacity == 0 {
                ptr = 0 as *mut T;
            } else {
                let (slice1, slice2) = self.as_slices();
                ptr = alloc::<T>(capacity) as *mut T;
                let len1 = slice1.len();
                ptr::copy_nonoverlapping_memory(ptr, slice1.as_ptr(), len1);
                ptr::copy_nonoverlapping_memory(ptr.offset(len1 as int),
                slice2.as_ptr(),
                slice2.len());
            }
            if self.cap != 0 {
                dealloc(self.ptr, self.cap);
            }
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
                                     self.ptr.offset(self.lo as int) as *const T,
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
                                                    tmp as *const T,
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
                    tmp as *const T,
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
    fn partial_cmp(&self, other: &RingBuf<T>) -> Option<Ordering> {
        for (a, b) in self.iter().zip(other.iter()) {
            let cmp = a.partial_cmp(b);
            if cmp != Some(Equal) {
                return cmp;
            }
        }

        Some(self.len.cmp(&other.len))
    }
}

impl<T: Eq> Eq for RingBuf<T> {}

impl<T: Ord> Ord for RingBuf<T> {
    #[inline]
    fn cmp(&self, other: &RingBuf<T>) -> Ordering {
        self.partial_cmp(other).expect("No ordering for Ord elements.")
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

/// An iterator that moves out of a RingBuf.
pub struct MoveItems<T> {
    allocation: *mut T, // the block of memory allocated for the ringbuf
    cap: uint, // the capacity of the ringbuf
    iter: Items<'static, T>
}

impl<T> Iterator<T> for MoveItems<T> {
    #[inline]
    fn next(&mut self) -> Option<T> {
        unsafe {
            self.iter.next().map(|x| ptr::read(x))
        }
    }

    #[inline]
    fn size_hint(&self) -> (uint, Option<uint>) {
        self.iter.size_hint()
    }
}

impl<T> DoubleEndedIterator<T> for MoveItems<T> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        unsafe {
            self.iter.next_back().map(|x| ptr::read(x))
        }
    }
}

#[unsafe_destructor]
impl<T> Drop for MoveItems<T> {
    fn drop(&mut self) {
        // destroy the remaining elements
        if self.cap != 0 {
            for _x in *self {}
            unsafe {
                dealloc(self.allocation, self.cap);
            }
        }
    }
}

#[cfg(test)]
mod checks {
    use std::collections::Deque;
    use std::iter::FromIterator;
    use std::rand::Rand;

    use quickcheck::Arbitrary;
    use quickcheck::Gen;
    use quickcheck::Shrinker;
    use quickcheck::quickcheck;

    use super::RingBuf;

    /// Creates a new ringbuf with a provided initial capacity and offset, and
    /// copied elements from the provided slice. This is a convenience for
    /// creating a ringbuf with a `lo` offset other than the default buffer start.
    fn create_ringbuf_with_offset<T: Copy>(items: &[T],
                                           capacity: uint,
                                           lo: uint)
                                           -> RingBuf<T> {
            let mut ringbuf = RingBuf::with_capacity(capacity);
            ringbuf.lo = if capacity == 0 { 0 } else { lo % capacity };
            for &i in items.iter() {
                ringbuf.push_back(i);
            }
            ringbuf
        }

    impl<A: Copy + Arbitrary> Arbitrary for RingBuf<A> {
        fn arbitrary<G: Gen>(g: &mut G) -> RingBuf<A> {
            let vec: Vec<A> = Arbitrary::arbitrary(g);
            let cap = vec.capacity();
            let lo = if cap == 0 { 0 } else { g.gen_range(0, cap) };

            create_ringbuf_with_offset(vec.as_slice(), cap, lo)
        }

        fn shrink(&self) -> Box<Shrinker<RingBuf<A>>> {
            let mut xs: Vec<RingBuf<A>> = vec![];

            // Add versions with varying offsets
            let mut lo = self.lo;
            while lo > 0 {
                let shrinks = self.clone()
                                  .into_vec()
                                  .shrink()
                                  .map(|x| create_ringbuf_with_offset(x.as_slice(),
                                                                      self.cap,
                                                                      lo))
                                  .collect();
                xs.push_all_move(shrinks);
                lo = lo / 2;
            }

            // Add version with 0 offset
            let shrinks = self.clone()
                              .into_vec()
                              .shrink()
                              .map(|x| RingBuf::from_vec(x))
                              .collect();
            xs.push_all_move(shrinks);
            box xs.move_iter() as Box<Shrinker<RingBuf<A>>>
        }
    }

    #[test]
    fn check_vec_bijection() {
        fn prop(rb: RingBuf<int>) -> bool {
            rb == RingBuf::from_vec(rb.clone().into_vec())
        }

        quickcheck(prop);
    }

    #[test]
    fn check_iter_bijection() {
        fn prop(rb: RingBuf<int>) -> bool {
            rb == FromIterator::from_iter(rb.clone().move_iter())
        }

        quickcheck(prop);
    }

    #[test]
    fn check_clone_equivalence() {
        fn prop(rb: RingBuf<int>) -> bool {
            rb == rb.clone()
        }

        quickcheck(prop);
    }

    #[test]
    fn test_shrink_to_fit_equivalence() {
        fn prop(rb: RingBuf<int>) -> bool {
            let mut stf = rb.clone();
            stf.shrink_to_fit();
            rb == stf
        }

        quickcheck(prop);
    }

    #[test]
    fn check_back_push_pop_get() {
        fn prop(rb: RingBuf<int>, item: int) -> bool {
            let mut copy = rb.clone();
            copy.push_back(item);

            copy.back() == Some(&item)
                && copy.pop_back() == Some(item)
                && copy == rb
        }
        quickcheck(prop);
    }

    #[test]
    fn check_front_push_pop_get() {
        fn prop(rb: RingBuf<int>, item: int) -> bool {
            let mut copy = rb.clone();
            copy.push_front(item);

            copy.front() == Some(&item)
                && copy.pop_front() == Some(item)
                && copy == rb
        }
        quickcheck(prop);
    }

    #[test]
    fn check_get() {
        fn prop(rb: RingBuf<int>) -> bool {
            let vec = rb.clone().into_vec();
            rb.len() == vec.len()
                && rb.iter().zip(vec.iter()).all(|(a, b)| a == b)
        }
        quickcheck(prop);
    }

    #[test]
    fn check_cmp() {
        fn prop(rb1: RingBuf<int>, rb2: RingBuf<int>) -> bool {
            let vec1 = rb1.clone().into_vec();
            let vec2 = rb2.clone().into_vec();

            rb1.cmp(&rb2) == vec1.cmp(&vec2)
        }
        quickcheck(prop);
    }

    #[test]
    fn check_extendable() {
        fn prop(vec: Vec<int>) -> bool {
            let mut rb = RingBuf::new();
            rb.extend(vec.clone().move_iter());
            vec == rb.into_vec()
        }
        quickcheck(prop);
    }

    #[test]
    fn check_iter() {
        fn prop(rb: RingBuf<int>) -> bool {
            rb.clone().iter().zip(rb.into_vec().iter()).all(|(a, b)| a == b)
        }

        quickcheck(prop);
    }

    #[test]
    fn check_mut_iter() {
        fn prop(rb: RingBuf<int>) -> bool {
            rb.clone().mut_iter().zip(rb.into_vec().mut_iter()).all(|(a, b)| a == b)
        }

        quickcheck(prop);
    }

    #[test]
    fn check_move_iter() {
        fn prop(rb: RingBuf<int>) -> bool {
            rb.clone().move_iter().zip(rb.into_vec().move_iter()).all(|(a, b)| a == b)
        }

        quickcheck(prop);
    }

    #[test]
    fn check_truncate() {
        fn prop(mut rb: RingBuf<int>, len: uint) -> bool {
            let mut vec = rb.clone().into_vec();
            vec.truncate(len);
            rb.truncate(len);
            RingBuf::from_vec(vec) == rb
        }

        quickcheck(prop);
    }
}
