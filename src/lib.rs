// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
#![feature(unsafe_destructor, macro_rules)]


extern crate core;
extern crate debug;
extern crate test;
extern crate alloc;
extern crate core;
extern crate debug;
extern crate test;

extern crate quickcheck;

/// A double-ended queue implemented as a circular buffer
///
/// RingBuf implements the trait Deque. It should be imported with
/// `use collections::Deque`.
pub mod ringbuf {
  use std::fmt;
  use std::mem;
  use std::num;
  use core::raw::Slice;
  use std::slice::raw;
  use std::uint;
  use std::collections::Deque;
  use std::iter::Chain;
  use std::ptr;
  use std::rt::heap::{allocate, deallocate};
  use std::slice;

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

  /// Create a `std::collections::ringbuf::RingBuf` containing the arguments.
  #[macro_export]
  macro_rules! ringbuf(
      ($($e:expr),*) => ({
          // leading _ to allow empty construction without a warning.
          let mut _temp = RingBuf::new();
          $(_temp.push_back($e);)*
          _temp
      });
      ($($e:expr),+,) => (ringbuf!($($e),+))
  )


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
      /// let mut rb = RingBuf::new();
      /// rb.push_back(1);
      /// rb.push_front(0);
      /// let (slice1, slice2) = rb.as_slices();
      /// ```
      #[inline]
      pub fn as_slices<'a>(&'a self) -> (&'a [T], &'a [T]) {
          unsafe {
              let ptr1;
              let ptr2;
              let len1;
              let len2;
              if self.lo > self.cap - self.len {
                  ptr1 = (self.ptr as *T).offset(self.lo as int);
                  ptr2 = self.ptr as *T;
                  len1 = self.cap - self.lo;
                  len2 = self.len - len1;
              } else {
                  ptr1 = (self.ptr as *T).offset(self.lo as int);
                  ptr2 = self.ptr as *T;
                  len1 = self.len;
                  len2 = 0;
              }
              (raw::buf_as_slice(ptr1, len1, |x| mem::transmute(x)),
               raw::buf_as_slice(ptr2, len2, |x| mem::transmute(x)))
          }
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
      fn test_deque_macro() {
        let mut deque = ringbuf!(1i, 5, 9);
        assert_eq!(Some(1), deque.pop_front());
        assert_eq!(Some(5), deque.pop_front());
        assert_eq!(Some(9), deque.pop_front());
        assert_eq!(None, deque.pop_front());
      }

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
}

