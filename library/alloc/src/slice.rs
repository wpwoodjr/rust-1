//! A dynamically-sized view into a contiguous sequence, `[T]`.
//!
//! *[See also the slice primitive type](slice).*
//!
//! Slices are a view into a block of memory represented as a pointer and a
//! length.
//!
//! ```
//! // slicing a Vec
//! let vec = vec![1, 2, 3];
//! let int_slice = &vec[..];
//! // coercing an array to a slice
//! let str_slice: &[&str] = &["one", "two", "three"];
//! ```
//!
//! Slices are either mutable or shared. The shared slice type is `&[T]`,
//! while the mutable slice type is `&mut [T]`, where `T` represents the element
//! type. For example, you can mutate the block of memory that a mutable slice
//! points to:
//!
//! ```
//! let x = &mut [1, 2, 3];
//! x[1] = 7;
//! assert_eq!(x, &[1, 7, 3]);
//! ```
//!
//! Here are some of the things this module contains:
//!
//! ## Structs
//!
//! There are several structs that are useful for slices, such as [`Iter`], which
//! represents iteration over a slice.
//!
//! ## Trait Implementations
//!
//! There are several implementations of common traits for slices. Some examples
//! include:
//!
//! * [`Clone`]
//! * [`Eq`], [`Ord`] - for slices whose element type are [`Eq`] or [`Ord`].
//! * [`Hash`] - for slices whose element type is [`Hash`].
//!
//! ## Iteration
//!
//! The slices implement `IntoIterator`. The iterator yields references to the
//! slice elements.
//!
//! ```
//! let numbers = &[0, 1, 2];
//! for n in numbers {
//!     println!("{n} is a number!");
//! }
//! ```
//!
//! The mutable slice yields mutable references to the elements:
//!
//! ```
//! let mut scores = [7, 8, 9];
//! for score in &mut scores[..] {
//!     *score += 1;
//! }
//! ```
//!
//! This iterator yields mutable references to the slice's elements, so while
//! the element type of the slice is `i32`, the element type of the iterator is
//! `&mut i32`.
//!
//! * [`.iter`] and [`.iter_mut`] are the explicit methods to return the default
//!   iterators.
//! * Further methods that return iterators are [`.split`], [`.splitn`],
//!   [`.chunks`], [`.windows`] and more.
//!
//! [`Hash`]: core::hash::Hash
//! [`.iter`]: slice::iter
//! [`.iter_mut`]: slice::iter_mut
//! [`.split`]: slice::split
//! [`.splitn`]: slice::splitn
//! [`.chunks`]: slice::chunks
//! [`.windows`]: slice::windows
#![stable(feature = "rust1", since = "1.0.0")]
// Many of the usings in this module are only used in the test configuration.
// It's cleaner to just turn off the unused_imports warning than to fix them.
#![cfg_attr(test, allow(unused_imports, dead_code))]

use core::borrow::{Borrow, BorrowMut};
#[cfg(not(no_global_oom_handling))]
use core::cmp::Ordering::{self, Less};
#[cfg(not(no_global_oom_handling))]
use core::mem;
#[cfg(not(no_global_oom_handling))]
use core::mem::size_of;
#[cfg(not(no_global_oom_handling))]
use core::ptr;

use crate::alloc::Allocator;
#[cfg(not(no_global_oom_handling))]
use crate::alloc::Global;
#[cfg(not(no_global_oom_handling))]
use crate::borrow::ToOwned;
use crate::boxed::Box;
use crate::vec::Vec;

#[unstable(feature = "slice_range", issue = "76393")]
pub use core::slice::range;
#[unstable(feature = "array_chunks", issue = "74985")]
pub use core::slice::ArrayChunks;
#[unstable(feature = "array_chunks", issue = "74985")]
pub use core::slice::ArrayChunksMut;
#[unstable(feature = "array_windows", issue = "75027")]
pub use core::slice::ArrayWindows;
#[stable(feature = "inherent_ascii_escape", since = "1.60.0")]
pub use core::slice::EscapeAscii;
#[stable(feature = "slice_get_slice", since = "1.28.0")]
pub use core::slice::SliceIndex;
#[stable(feature = "from_ref", since = "1.28.0")]
pub use core::slice::{from_mut, from_ref};
#[stable(feature = "rust1", since = "1.0.0")]
pub use core::slice::{from_raw_parts, from_raw_parts_mut};
#[stable(feature = "rust1", since = "1.0.0")]
pub use core::slice::{Chunks, Windows};
#[stable(feature = "chunks_exact", since = "1.31.0")]
pub use core::slice::{ChunksExact, ChunksExactMut};
#[stable(feature = "rust1", since = "1.0.0")]
pub use core::slice::{ChunksMut, Split, SplitMut};
#[unstable(feature = "slice_group_by", issue = "80552")]
pub use core::slice::{GroupBy, GroupByMut};
#[stable(feature = "rust1", since = "1.0.0")]
pub use core::slice::{Iter, IterMut};
#[stable(feature = "rchunks", since = "1.31.0")]
pub use core::slice::{RChunks, RChunksExact, RChunksExactMut, RChunksMut};
#[stable(feature = "slice_rsplit", since = "1.27.0")]
pub use core::slice::{RSplit, RSplitMut};
#[stable(feature = "rust1", since = "1.0.0")]
pub use core::slice::{RSplitN, RSplitNMut, SplitN, SplitNMut};
#[stable(feature = "split_inclusive", since = "1.51.0")]
pub use core::slice::{SplitInclusive, SplitInclusiveMut};

////////////////////////////////////////////////////////////////////////////////
// Basic slice extension methods
////////////////////////////////////////////////////////////////////////////////

// HACK(japaric) needed for the implementation of `vec!` macro during testing
// N.B., see the `hack` module in this file for more details.
#[cfg(test)]
pub use hack::into_vec;

// HACK(japaric) needed for the implementation of `Vec::clone` during testing
// N.B., see the `hack` module in this file for more details.
#[cfg(test)]
pub use hack::to_vec;

// HACK(japaric): With cfg(test) `impl [T]` is not available, these three
// functions are actually methods that are in `impl [T]` but not in
// `core::slice::SliceExt` - we need to supply these functions for the
// `test_permutations` test
pub(crate) mod hack {
    use core::alloc::Allocator;

    use crate::boxed::Box;
    use crate::vec::Vec;

    // We shouldn't add inline attribute to this since this is used in
    // `vec!` macro mostly and causes perf regression. See #71204 for
    // discussion and perf results.
    pub fn into_vec<T, A: Allocator>(b: Box<[T], A>) -> Vec<T, A> {
        unsafe {
            let len = b.len();
            let (b, alloc) = Box::into_raw_with_allocator(b);
            Vec::from_raw_parts_in(b as *mut T, len, len, alloc)
        }
    }

    #[cfg(not(no_global_oom_handling))]
    #[inline]
    pub fn to_vec<T: ConvertVec, A: Allocator>(s: &[T], alloc: A) -> Vec<T, A> {
        T::to_vec(s, alloc)
    }

    #[cfg(not(no_global_oom_handling))]
    pub trait ConvertVec {
        fn to_vec<A: Allocator>(s: &[Self], alloc: A) -> Vec<Self, A>
        where
            Self: Sized;
    }

    #[cfg(not(no_global_oom_handling))]
    impl<T: Clone> ConvertVec for T {
        #[inline]
        default fn to_vec<A: Allocator>(s: &[Self], alloc: A) -> Vec<Self, A> {
            struct DropGuard<'a, T, A: Allocator> {
                vec: &'a mut Vec<T, A>,
                num_init: usize,
            }
            impl<'a, T, A: Allocator> Drop for DropGuard<'a, T, A> {
                #[inline]
                fn drop(&mut self) {
                    // SAFETY:
                    // items were marked initialized in the loop below
                    unsafe {
                        self.vec.set_len(self.num_init);
                    }
                }
            }
            let mut vec = Vec::with_capacity_in(s.len(), alloc);
            let mut guard = DropGuard { vec: &mut vec, num_init: 0 };
            let slots = guard.vec.spare_capacity_mut();
            // .take(slots.len()) is necessary for LLVM to remove bounds checks
            // and has better codegen than zip.
            for (i, b) in s.iter().enumerate().take(slots.len()) {
                guard.num_init = i;
                slots[i].write(b.clone());
            }
            core::mem::forget(guard);
            // SAFETY:
            // the vec was allocated and initialized above to at least this length.
            unsafe {
                vec.set_len(s.len());
            }
            vec
        }
    }

    #[cfg(not(no_global_oom_handling))]
    impl<T: Copy> ConvertVec for T {
        #[inline]
        fn to_vec<A: Allocator>(s: &[Self], alloc: A) -> Vec<Self, A> {
            let mut v = Vec::with_capacity_in(s.len(), alloc);
            // SAFETY:
            // allocated above with the capacity of `s`, and initialize to `s.len()` in
            // ptr::copy_to_non_overlapping below.
            unsafe {
                s.as_ptr().copy_to_nonoverlapping(v.as_mut_ptr(), s.len());
                v.set_len(s.len());
            }
            v
        }
    }
}

#[cfg(not(test))]
impl<T> [T] {
    /// Sorts the slice.
    ///
    /// This sort is stable (i.e., does not reorder equal elements) and *O*(*n* \* log(*n*)) worst-case.
    ///
    /// When applicable, unstable sorting is preferred because it is generally faster than stable
    /// sorting and it doesn't allocate auxiliary memory.
    /// See [`sort_unstable`](slice::sort_unstable).
    ///
    /// # Current implementation
    ///
    /// This is a stable two stage merge sort with pre-sorted prefix optimization.
    /// It is quite fast in cases where the slice is nearly sorted, or consists of
    /// two or more sorted sequences concatenated one after another, while remaining very fast for
    /// randomly sorted sequences.
    ///
    /// Also, it allocates temporary storage half the size of `self`, but for short slices a
    /// non-allocating insertion sort is used instead.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut v = [-5, 4, 1, -3, 2];
    ///
    /// v.sort();
    /// assert!(v == [-5, -3, 1, 2, 4]);
    /// ```
    #[cfg(not(no_global_oom_handling))]
    #[rustc_allow_incoherent_impl]
    #[stable(feature = "rust1", since = "1.0.0")]
    #[inline]
    pub fn sort(&mut self)
    where
        T: Ord,
    {
        merge_sort(self, |a, b| a.lt(b));
    }

    /// Sorts the slice with a comparator function.
    ///
    /// This sort is stable (i.e., does not reorder equal elements) and *O*(*n* \* log(*n*)) worst-case.
    ///
    /// The comparator function must define a total ordering for the elements in the slice. If
    /// the ordering is not total, the order of the elements is unspecified. An order is a
    /// total order if it is (for all `a`, `b` and `c`):
    ///
    /// * total and antisymmetric: exactly one of `a < b`, `a == b` or `a > b` is true, and
    /// * transitive, `a < b` and `b < c` implies `a < c`. The same must hold for both `==` and `>`.
    ///
    /// For example, while [`f64`] doesn't implement [`Ord`] because `NaN != NaN`, we can use
    /// `partial_cmp` as our sort function when we know the slice doesn't contain a `NaN`.
    ///
    /// ```
    /// let mut floats = [5f64, 4.0, 1.0, 3.0, 2.0];
    /// floats.sort_by(|a, b| a.partial_cmp(b).unwrap());
    /// assert_eq!(floats, [1.0, 2.0, 3.0, 4.0, 5.0]);
    /// ```
    ///
    /// When applicable, unstable sorting is preferred because it is generally faster than stable
    /// sorting and it doesn't allocate auxiliary memory.
    /// See [`sort_unstable_by`](slice::sort_unstable_by).
    ///
    /// # Current implementation
    ///
    /// This is a stable two stage merge sort with pre-sorted prefix optimization.
    /// It is quite fast in cases where the slice is nearly sorted, or consists of
    /// two or more sorted sequences concatenated one after another, while remaining very fast for
    /// randomly sorted sequences.
    ///
    /// Also, it allocates temporary storage half the size of `self`, but for short slices a
    /// non-allocating insertion sort is used instead.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut v = [5, 4, 1, 3, 2];
    /// v.sort_by(|a, b| a.cmp(b));
    /// assert!(v == [1, 2, 3, 4, 5]);
    ///
    /// // reverse sorting
    /// v.sort_by(|a, b| b.cmp(a));
    /// assert!(v == [5, 4, 3, 2, 1]);
    /// ```
    #[cfg(not(no_global_oom_handling))]
    #[rustc_allow_incoherent_impl]
    #[stable(feature = "rust1", since = "1.0.0")]
    #[inline]
    pub fn sort_by<F>(&mut self, mut compare: F)
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        merge_sort(self, |a, b| compare(a, b) == Less);
    }

    /// Sorts the slice with a key extraction function.
    ///
    /// This sort is stable (i.e., does not reorder equal elements) and *O*(*m* \* *n* \* log(*n*))
    /// worst-case, where the key function is *O*(*m*).
    ///
    /// For expensive key functions (e.g. functions that are not simple property accesses or
    /// basic operations), [`sort_by_cached_key`](slice::sort_by_cached_key) is likely to be
    /// significantly faster, as it does not recompute element keys.
    ///
    /// When applicable, unstable sorting is preferred because it is generally faster than stable
    /// sorting and it doesn't allocate auxiliary memory.
    /// See [`sort_unstable_by_key`](slice::sort_unstable_by_key).
    ///
    /// # Current implementation
    ///
    /// The current algorithm is an adaptive, iterative merge sort inspired by
    /// [timsort](https://en.wikipedia.org/wiki/Timsort).
    /// It is designed to be very fast in cases where the slice is nearly sorted, or consists of
    /// two or more sorted sequences concatenated one after another.
    ///
    /// Also, it allocates temporary storage half the size of `self`, but for short slices a
    /// non-allocating insertion sort is used instead.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut v = [-5i32, 4, 1, -3, 2];
    ///
    /// v.sort_by_key(|k| k.abs());
    /// assert!(v == [1, 2, -3, 4, -5]);
    /// ```
    #[cfg(not(no_global_oom_handling))]
    #[rustc_allow_incoherent_impl]
    #[stable(feature = "slice_sort_by_key", since = "1.7.0")]
    #[inline]
    pub fn sort_by_key<K, F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        merge_sort(self, |a, b| f(a).lt(&f(b)));
    }

    /// Sorts the slice with a key extraction function.
    ///
    /// During sorting, the key function is called at most once per element, by using
    /// temporary storage to remember the results of key evaluation.
    /// The order of calls to the key function is unspecified and may change in future versions
    /// of the standard library.
    ///
    /// This sort is stable (i.e., does not reorder equal elements) and *O*(*m* \* *n* + *n* \* log(*n*))
    /// worst-case, where the key function is *O*(*m*).
    ///
    /// For simple key functions (e.g., functions that are property accesses or
    /// basic operations), [`sort_by_key`](slice::sort_by_key) is likely to be
    /// faster.
    ///
    /// # Current implementation
    ///
    /// The current algorithm is based on [pattern-defeating quicksort][pdqsort] by Orson Peters,
    /// which combines the fast average case of randomized quicksort with the fast worst case of
    /// heapsort, while achieving linear time on slices with certain patterns. It uses some
    /// randomization to avoid degenerate cases, but with a fixed seed to always provide
    /// deterministic behavior.
    ///
    /// In the worst case, the algorithm allocates temporary storage in a `Vec<(K, usize)>` the
    /// length of the slice.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut v = [-5i32, 4, 32, -3, 2];
    ///
    /// v.sort_by_cached_key(|k| k.to_string());
    /// assert!(v == [-3, -5, 2, 32, 4]);
    /// ```
    ///
    /// [pdqsort]: https://github.com/orlp/pdqsort
    #[cfg(not(no_global_oom_handling))]
    #[rustc_allow_incoherent_impl]
    #[stable(feature = "slice_sort_by_cached_key", since = "1.34.0")]
    #[inline]
    pub fn sort_by_cached_key<K, F>(&mut self, f: F)
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        // Helper macro for indexing our vector by the smallest possible type, to reduce allocation.
        macro_rules! sort_by_key {
            ($t:ty, $slice:ident, $f:ident) => {{
                let mut indices: Vec<_> =
                    $slice.iter().map($f).enumerate().map(|(i, k)| (k, i as $t)).collect();
                // The elements of `indices` are unique, as they are indexed, so any sort will be
                // stable with respect to the original slice. We use `sort_unstable` here because
                // it requires less memory allocation.
                indices.sort_unstable();
                for i in 0..$slice.len() {
                    let mut index = indices[i].1;
                    while (index as usize) < i {
                        index = indices[index as usize].1;
                    }
                    indices[i].1 = index;
                    $slice.swap(i, index as usize);
                }
            }};
        }

        let sz_u8 = mem::size_of::<(K, u8)>();
        let sz_u16 = mem::size_of::<(K, u16)>();
        let sz_u32 = mem::size_of::<(K, u32)>();
        let sz_usize = mem::size_of::<(K, usize)>();

        let len = self.len();
        if len < 2 {
            return;
        }
        if sz_u8 < sz_u16 && len <= (u8::MAX as usize) {
            return sort_by_key!(u8, self, f);
        }
        if sz_u16 < sz_u32 && len <= (u16::MAX as usize) {
            return sort_by_key!(u16, self, f);
        }
        if sz_u32 < sz_usize && len <= (u32::MAX as usize) {
            return sort_by_key!(u32, self, f);
        }
        sort_by_key!(usize, self, f)
    }

    /// Copies `self` into a new `Vec`.
    ///
    /// # Examples
    ///
    /// ```
    /// let s = [10, 40, 30];
    /// let x = s.to_vec();
    /// // Here, `s` and `x` can be modified independently.
    /// ```
    #[cfg(not(no_global_oom_handling))]
    #[rustc_allow_incoherent_impl]
    #[rustc_conversion_suggestion]
    #[stable(feature = "rust1", since = "1.0.0")]
    #[inline]
    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.to_vec_in(Global)
    }

    /// Copies `self` into a new `Vec` with an allocator.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    ///
    /// use std::alloc::System;
    ///
    /// let s = [10, 40, 30];
    /// let x = s.to_vec_in(System);
    /// // Here, `s` and `x` can be modified independently.
    /// ```
    #[cfg(not(no_global_oom_handling))]
    #[rustc_allow_incoherent_impl]
    #[inline]
    #[unstable(feature = "allocator_api", issue = "32838")]
    pub fn to_vec_in<A: Allocator>(&self, alloc: A) -> Vec<T, A>
    where
        T: Clone,
    {
        // N.B., see the `hack` module in this file for more details.
        hack::to_vec(self, alloc)
    }

    /// Converts `self` into a vector without clones or allocation.
    ///
    /// The resulting vector can be converted back into a box via
    /// `Vec<T>`'s `into_boxed_slice` method.
    ///
    /// # Examples
    ///
    /// ```
    /// let s: Box<[i32]> = Box::new([10, 40, 30]);
    /// let x = s.into_vec();
    /// // `s` cannot be used anymore because it has been converted into `x`.
    ///
    /// assert_eq!(x, vec![10, 40, 30]);
    /// ```
    #[rustc_allow_incoherent_impl]
    #[stable(feature = "rust1", since = "1.0.0")]
    #[inline]
    pub fn into_vec<A: Allocator>(self: Box<Self, A>) -> Vec<T, A> {
        // N.B., see the `hack` module in this file for more details.
        hack::into_vec(self)
    }

    /// Creates a vector by repeating a slice `n` times.
    ///
    /// # Panics
    ///
    /// This function will panic if the capacity would overflow.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// assert_eq!([1, 2].repeat(3), vec![1, 2, 1, 2, 1, 2]);
    /// ```
    ///
    /// A panic upon overflow:
    ///
    /// ```should_panic
    /// // this will panic at runtime
    /// b"0123456789abcdef".repeat(usize::MAX);
    /// ```
    #[rustc_allow_incoherent_impl]
    #[cfg(not(no_global_oom_handling))]
    #[stable(feature = "repeat_generic_slice", since = "1.40.0")]
    pub fn repeat(&self, n: usize) -> Vec<T>
    where
        T: Copy,
    {
        if n == 0 {
            return Vec::new();
        }

        // If `n` is larger than zero, it can be split as
        // `n = 2^expn + rem (2^expn > rem, expn >= 0, rem >= 0)`.
        // `2^expn` is the number represented by the leftmost '1' bit of `n`,
        // and `rem` is the remaining part of `n`.

        // Using `Vec` to access `set_len()`.
        let capacity = self.len().checked_mul(n).expect("capacity overflow");
        let mut buf = Vec::with_capacity(capacity);

        // `2^expn` repetition is done by doubling `buf` `expn`-times.
        buf.extend(self);
        {
            let mut m = n >> 1;
            // If `m > 0`, there are remaining bits up to the leftmost '1'.
            while m > 0 {
                // `buf.extend(buf)`:
                unsafe {
                    ptr::copy_nonoverlapping(
                        buf.as_ptr(),
                        (buf.as_mut_ptr() as *mut T).add(buf.len()),
                        buf.len(),
                    );
                    // `buf` has capacity of `self.len() * n`.
                    let buf_len = buf.len();
                    buf.set_len(buf_len * 2);
                }

                m >>= 1;
            }
        }

        // `rem` (`= n - 2^expn`) repetition is done by copying
        // first `rem` repetitions from `buf` itself.
        let rem_len = capacity - buf.len(); // `self.len() * rem`
        if rem_len > 0 {
            // `buf.extend(buf[0 .. rem_len])`:
            unsafe {
                // This is non-overlapping since `2^expn > rem`.
                ptr::copy_nonoverlapping(
                    buf.as_ptr(),
                    (buf.as_mut_ptr() as *mut T).add(buf.len()),
                    rem_len,
                );
                // `buf.len() + rem_len` equals to `buf.capacity()` (`= self.len() * n`).
                buf.set_len(capacity);
            }
        }
        buf
    }

    /// Flattens a slice of `T` into a single value `Self::Output`.
    ///
    /// # Examples
    ///
    /// ```
    /// assert_eq!(["hello", "world"].concat(), "helloworld");
    /// assert_eq!([[1, 2], [3, 4]].concat(), [1, 2, 3, 4]);
    /// ```
    #[rustc_allow_incoherent_impl]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn concat<Item: ?Sized>(&self) -> <Self as Concat<Item>>::Output
    where
        Self: Concat<Item>,
    {
        Concat::concat(self)
    }

    /// Flattens a slice of `T` into a single value `Self::Output`, placing a
    /// given separator between each.
    ///
    /// # Examples
    ///
    /// ```
    /// assert_eq!(["hello", "world"].join(" "), "hello world");
    /// assert_eq!([[1, 2], [3, 4]].join(&0), [1, 2, 0, 3, 4]);
    /// assert_eq!([[1, 2], [3, 4]].join(&[0, 0][..]), [1, 2, 0, 0, 3, 4]);
    /// ```
    #[rustc_allow_incoherent_impl]
    #[stable(feature = "rename_connect_to_join", since = "1.3.0")]
    pub fn join<Separator>(&self, sep: Separator) -> <Self as Join<Separator>>::Output
    where
        Self: Join<Separator>,
    {
        Join::join(self, sep)
    }

    /// Flattens a slice of `T` into a single value `Self::Output`, placing a
    /// given separator between each.
    ///
    /// # Examples
    ///
    /// ```
    /// # #![allow(deprecated)]
    /// assert_eq!(["hello", "world"].connect(" "), "hello world");
    /// assert_eq!([[1, 2], [3, 4]].connect(&0), [1, 2, 0, 3, 4]);
    /// ```
    #[rustc_allow_incoherent_impl]
    #[stable(feature = "rust1", since = "1.0.0")]
    #[deprecated(since = "1.3.0", note = "renamed to join")]
    pub fn connect<Separator>(&self, sep: Separator) -> <Self as Join<Separator>>::Output
    where
        Self: Join<Separator>,
    {
        Join::join(self, sep)
    }
}

#[cfg(not(test))]
impl [u8] {
    /// Returns a vector containing a copy of this slice where each byte
    /// is mapped to its ASCII upper case equivalent.
    ///
    /// ASCII letters 'a' to 'z' are mapped to 'A' to 'Z',
    /// but non-ASCII letters are unchanged.
    ///
    /// To uppercase the value in-place, use [`make_ascii_uppercase`].
    ///
    /// [`make_ascii_uppercase`]: slice::make_ascii_uppercase
    #[cfg(not(no_global_oom_handling))]
    #[rustc_allow_incoherent_impl]
    #[must_use = "this returns the uppercase bytes as a new Vec, \
                  without modifying the original"]
    #[stable(feature = "ascii_methods_on_intrinsics", since = "1.23.0")]
    #[inline]
    pub fn to_ascii_uppercase(&self) -> Vec<u8> {
        let mut me = self.to_vec();
        me.make_ascii_uppercase();
        me
    }

    /// Returns a vector containing a copy of this slice where each byte
    /// is mapped to its ASCII lower case equivalent.
    ///
    /// ASCII letters 'A' to 'Z' are mapped to 'a' to 'z',
    /// but non-ASCII letters are unchanged.
    ///
    /// To lowercase the value in-place, use [`make_ascii_lowercase`].
    ///
    /// [`make_ascii_lowercase`]: slice::make_ascii_lowercase
    #[cfg(not(no_global_oom_handling))]
    #[rustc_allow_incoherent_impl]
    #[must_use = "this returns the lowercase bytes as a new Vec, \
                  without modifying the original"]
    #[stable(feature = "ascii_methods_on_intrinsics", since = "1.23.0")]
    #[inline]
    pub fn to_ascii_lowercase(&self) -> Vec<u8> {
        let mut me = self.to_vec();
        me.make_ascii_lowercase();
        me
    }
}

////////////////////////////////////////////////////////////////////////////////
// Extension traits for slices over specific kinds of data
////////////////////////////////////////////////////////////////////////////////

/// Helper trait for [`[T]::concat`](slice::concat).
///
/// Note: the `Item` type parameter is not used in this trait,
/// but it allows impls to be more generic.
/// Without it, we get this error:
///
/// ```error
/// error[E0207]: the type parameter `T` is not constrained by the impl trait, self type, or predica
///    --> src/liballoc/slice.rs:608:6
///     |
/// 608 | impl<T: Clone, V: Borrow<[T]>> Concat for [V] {
///     |      ^ unconstrained type parameter
/// ```
///
/// This is because there could exist `V` types with multiple `Borrow<[_]>` impls,
/// such that multiple `T` types would apply:
///
/// ```
/// # #[allow(dead_code)]
/// pub struct Foo(Vec<u32>, Vec<String>);
///
/// impl std::borrow::Borrow<[u32]> for Foo {
///     fn borrow(&self) -> &[u32] { &self.0 }
/// }
///
/// impl std::borrow::Borrow<[String]> for Foo {
///     fn borrow(&self) -> &[String] { &self.1 }
/// }
/// ```
#[unstable(feature = "slice_concat_trait", issue = "27747")]
pub trait Concat<Item: ?Sized> {
    #[unstable(feature = "slice_concat_trait", issue = "27747")]
    /// The resulting type after concatenation
    type Output;

    /// Implementation of [`[T]::concat`](slice::concat)
    #[unstable(feature = "slice_concat_trait", issue = "27747")]
    fn concat(slice: &Self) -> Self::Output;
}

/// Helper trait for [`[T]::join`](slice::join)
#[unstable(feature = "slice_concat_trait", issue = "27747")]
pub trait Join<Separator> {
    #[unstable(feature = "slice_concat_trait", issue = "27747")]
    /// The resulting type after concatenation
    type Output;

    /// Implementation of [`[T]::join`](slice::join)
    #[unstable(feature = "slice_concat_trait", issue = "27747")]
    fn join(slice: &Self, sep: Separator) -> Self::Output;
}

#[cfg(not(no_global_oom_handling))]
#[unstable(feature = "slice_concat_ext", issue = "27747")]
impl<T: Clone, V: Borrow<[T]>> Concat<T> for [V] {
    type Output = Vec<T>;

    fn concat(slice: &Self) -> Vec<T> {
        let size = slice.iter().map(|slice| slice.borrow().len()).sum();
        let mut result = Vec::with_capacity(size);
        for v in slice {
            result.extend_from_slice(v.borrow())
        }
        result
    }
}

#[cfg(not(no_global_oom_handling))]
#[unstable(feature = "slice_concat_ext", issue = "27747")]
impl<T: Clone, V: Borrow<[T]>> Join<&T> for [V] {
    type Output = Vec<T>;

    fn join(slice: &Self, sep: &T) -> Vec<T> {
        let mut iter = slice.iter();
        let first = match iter.next() {
            Some(first) => first,
            None => return vec![],
        };
        let size = slice.iter().map(|v| v.borrow().len()).sum::<usize>() + slice.len() - 1;
        let mut result = Vec::with_capacity(size);
        result.extend_from_slice(first.borrow());

        for v in iter {
            result.push(sep.clone());
            result.extend_from_slice(v.borrow())
        }
        result
    }
}

#[cfg(not(no_global_oom_handling))]
#[unstable(feature = "slice_concat_ext", issue = "27747")]
impl<T: Clone, V: Borrow<[T]>> Join<&[T]> for [V] {
    type Output = Vec<T>;

    fn join(slice: &Self, sep: &[T]) -> Vec<T> {
        let mut iter = slice.iter();
        let first = match iter.next() {
            Some(first) => first,
            None => return vec![],
        };
        let size =
            slice.iter().map(|v| v.borrow().len()).sum::<usize>() + sep.len() * (slice.len() - 1);
        let mut result = Vec::with_capacity(size);
        result.extend_from_slice(first.borrow());

        for v in iter {
            result.extend_from_slice(sep);
            result.extend_from_slice(v.borrow())
        }
        result
    }
}

////////////////////////////////////////////////////////////////////////////////
// Standard trait implementations for slices
////////////////////////////////////////////////////////////////////////////////

#[stable(feature = "rust1", since = "1.0.0")]
impl<T> Borrow<[T]> for Vec<T> {
    fn borrow(&self) -> &[T] {
        &self[..]
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<T> BorrowMut<[T]> for Vec<T> {
    fn borrow_mut(&mut self) -> &mut [T] {
        &mut self[..]
    }
}

#[cfg(not(no_global_oom_handling))]
#[stable(feature = "rust1", since = "1.0.0")]
impl<T: Clone> ToOwned for [T] {
    type Owned = Vec<T>;
    #[cfg(not(test))]
    fn to_owned(&self) -> Vec<T> {
        self.to_vec()
    }

    #[cfg(test)]
    fn to_owned(&self) -> Vec<T> {
        hack::to_vec(self, Global)
    }

    fn clone_into(&self, target: &mut Vec<T>) {
        // drop anything in target that will not be overwritten
        target.truncate(self.len());

        // target.len <= self.len due to the truncate above, so the
        // slices here are always in-bounds.
        let (init, tail) = self.split_at(target.len());

        // reuse the contained values' allocations/resources.
        target.clone_from_slice(init);
        target.extend_from_slice(tail);
    }
}

////////////////////////////////////////////////////////////////////////////////
// Sorting
////////////////////////////////////////////////////////////////////////////////

/// Inserts `v[v.len() - 1]` into pre-sorted sequence `v[..v.len() - 1]` so that whole `v[..]` becomes sorted.
///
/// This is the integral subroutine of insertion sort.
#[cfg(not(no_global_oom_handling))]
// benchmarking indicated that inlining makes a substantial improvement, yet only requires a couple of hundred bytes
#[inline(always)]
fn insert_end<T, F>(v: &mut [T], is_less: &mut F)
where
    F: FnMut(&T, &T) -> bool,
{
    let end = v.len().saturating_sub(1);
    if end > 0 && is_less(&v[end], &v[end - 1]) {
        unsafe {
            // There are three ways to implement insertion here:
            //
            // 1. Swap adjacent elements until the last one gets to its final destination.
            //    However, this way we copy data around more than is necessary. If elements are big
            //    structures (costly to copy), this method will be slow.
            //
            // 2. Iterate until the right place for the last element is found. Then shift the
            //    elements preceeding it to make room for it and finally place it into the
            //    remaining hole. This is a good method.
            //
            // 3. Copy the last element into a temporary variable. Iterate until the right place
            //    for it is found. As we go along, copy every traversed element into the slot
            //    succeeding it. Finally, copy data from the temporary variable into the remaining
            //    hole. This method is very good. Benchmarks demonstrated slightly better
            //    performance than with the 2nd method.
            //
            // All methods were benchmarked, and the 3rd showed best results. So we chose that one.
            let tmp = mem::ManuallyDrop::new(ptr::read(v.get_unchecked(end)));

            // Intermediate state of the insertion process is always tracked by `hole`, which
            // serves two purposes:
            // 1. Protects integrity of `v` from panics in `is_less`.
            // 2. Fills the remaining hole in `v` in the end.
            //
            // Panic safety:
            //
            // If `is_less` panics at any point during the process, `hole` will get dropped and
            // fill the hole in `v` with `tmp`, thus ensuring that `v` still holds every object it
            // initially held exactly once.
            let mut hole = InsertionHole { src: &*tmp, dest: v.get_unchecked_mut(end - 1) };
            ptr::copy_nonoverlapping(hole.dest, v.get_unchecked_mut(end), 1);

            let mut i = end - 1;
            while i > 0 && is_less(&*tmp, v.get_unchecked(i - 1)) {
                hole.dest = v.get_unchecked_mut(i - 1);
                ptr::copy_nonoverlapping(hole.dest, v.get_unchecked_mut(i), 1);
                i -= 1;
            }
            // `hole` gets dropped and thus copies `tmp` into the remaining hole in `v`.
        }
    }

    // When dropped, copies from `src` into `dest`.
    struct InsertionHole<T> {
        src: *const T,
        dest: *mut T,
    }

    impl<T> Drop for InsertionHole<T> {
        fn drop(&mut self) {
            unsafe {
                ptr::copy_nonoverlapping(self.src, self.dest, 1);
            }
        }
    }
}

/// Merges non-decreasing runs `v[..mid]` and `v[mid..]` using `buf` as temporary storage, and
/// stores the result into `v[..]`.
///
/// # Safety
///
/// The two slices must be non-empty and `mid` must be in bounds. Buffer `buf` must be long enough
/// to hold a copy of the shorter slice.
#[allow(unused_unsafe)]
#[cfg(not(no_global_oom_handling))]
unsafe fn merge<T, F>(v: &mut [T], mid: usize, buf: *mut T, is_less: &mut F)
where
    F: FnMut(&T, &T) -> bool,
{
    let len = v.len();
    let v = v.as_mut_ptr();
    let (v_mid, v_end) = unsafe { (v.add(mid), v.add(len)) };

    // The merge process first copies the shorter run into `buf`. Then it traces the newly copied
    // run and the longer run forwards (or backwards), comparing their next unconsumed elements and
    // copying the lesser (or greater) one into `v`.
    //
    // As soon as the shorter run is fully consumed, the process is done. If the longer run gets
    // consumed first, then we must copy whatever is left of the shorter run into the remaining
    // hole in `v`.
    //
    // Intermediate state of the process is always tracked by `hole`, which serves two purposes:
    // 1. Protects integrity of `v` from panics in `is_less`.
    // 2. Fills the remaining hole in `v` if the longer run gets consumed first.
    //
    // Panic safety:
    //
    // If `is_less` panics at any point during the process, `hole` will get dropped and fill the
    // hole in `v` with the unconsumed range in `buf`, thus ensuring that `v` still holds every
    // object it initially held exactly once.
    let mut hole;

    if mid <= len - mid {
        // The left run is shorter.
        unsafe {
            ptr::copy_nonoverlapping(v, buf, mid);
            hole = MergeHole { start: buf, end: buf.add(mid), dest: v };
        }

        // Initially, these pointers point to the beginnings of their arrays.
        let left = &mut hole.start;
        let mut right = v_mid;
        let out = &mut hole.dest;

        while *left < hole.end && right < v_end {
            // Consume the lesser side.
            // If equal, prefer the left run to maintain stability.
            unsafe {
                let to_copy = if is_less(&*right, &**left) {
                    get_and_increment(&mut right)
                } else {
                    get_and_increment(left)
                };
                ptr::copy_nonoverlapping(to_copy, get_and_increment(out), 1);
            }
        }
    } else {
        // The right run is shorter.
        unsafe {
            ptr::copy_nonoverlapping(v_mid, buf, len - mid);
            hole = MergeHole { start: buf, end: buf.add(len - mid), dest: v_mid };
        }

        // Initially, these pointers point past the ends of their arrays.
        let left = &mut hole.dest;
        let right = &mut hole.end;
        let mut out = v_end;

        while v < *left && buf < *right {
            // Consume the greater side.
            // If equal, prefer the right run to maintain stability.
            unsafe {
                let to_copy = if is_less(&*right.offset(-1), &*left.offset(-1)) {
                    decrement_and_get(left)
                } else {
                    decrement_and_get(right)
                };
                ptr::copy_nonoverlapping(to_copy, decrement_and_get(&mut out), 1);
            }
        }
    }
    // Finally, `hole` gets dropped. If the shorter run was not fully consumed, whatever remains of
    // it will now be copied into the hole in `v`.

    #[inline(always)]
    unsafe fn get_and_increment<T>(ptr: &mut *mut T) -> *mut T {
        let old = *ptr;
        *ptr = unsafe { ptr.offset(1) };
        old
    }

    #[inline(always)]
    unsafe fn decrement_and_get<T>(ptr: &mut *mut T) -> *mut T {
        *ptr = unsafe { ptr.offset(-1) };
        *ptr
    }

    // When dropped, copies the range `start..end` into `dest..`.
    struct MergeHole<T> {
        start: *mut T,
        end: *mut T,
        dest: *mut T,
    }

    impl<T> Drop for MergeHole<T> {
        fn drop(&mut self) {
            // `T` is not a zero-sized type, and these are pointers into a slice's elements.
            unsafe {
                let len = self.end.sub_ptr(self.start);
                ptr::copy_nonoverlapping(self.start, self.dest, len);
            }
        }
    }
}

/// This is a stable two stage merge sort with pre-sorted prefix optimization. The two stages are:
///
/// 1) Top-down recursive depth-first merge, which helps data locality
/// 2) Small slices sort using a fast insertion sort, then merge
///
/// The total running time is *O*(*n* \* log(*n*)) worst-case.
#[cfg(not(no_global_oom_handling))]
fn merge_sort<T, F>(v: &mut [T], mut is_less: F)
where
    F: FnMut(&T, &T) -> bool,
{
    // `gt!` macro centralizes and clarifies the logic.
    // Unchecked array access gives an approximate 20% performance improvement.
    //
    // # Safety
    //
    // `$left` and `$right` must be < `$v.len()`
    macro_rules! gt {
        ($v: ident, $left: expr, $right: expr, $is_less: ident) => {
            // $is_less(&$v[$right], &$v[$left])
            $is_less(unsafe { &$v.get_unchecked($right) }, unsafe { &$v.get_unchecked($left) })
        };
    }

    // Benchmarking determined these are the best sizes.
    // Recursive merge switches to insertion sort / merge when slice length is <= SMALL_SLICE_LEN*2.
    const SMALL_SLICE_LEN: usize = 10;
    // Slices of up to this length get sorted using insertion sort.
    const MAX_INSERTION: usize = 20;

    // Sorting has no meaningful behavior on zero-sized types.
    if size_of::<T>() == 0 {
        return;
    }

    let len = v.len();
    // Short arrays get sorted in-place via insertion sort to avoid allocations.
    if len <= MAX_INSERTION {
        for i in 1..len {
            insert_end(&mut v[..=i], &mut is_less);
        }
        return;
    }

    // Allocate a buffer to use as scratch memory. We keep the length 0 so we can keep in it
    // shallow copies of the contents of `v` without risking the dtors running on copies if
    // `is_less` panics. When merging two slices, this buffer holds a copy of the right-hand slice,
    // which will always have length at most `(len + 1) / 2`.
    let mut buf = Vec::with_capacity((len + 1) / 2);
    slice_merge_sort(v, 0, buf.as_mut_ptr(), &mut is_less);

    // Do a recursive depth-first merge while slice's length is greater than SMALL_SLICE_LEN*2.
    // Below that length use a combination of insertion sort and merging.
    // For optimization, `sorted` tracks how much of the slice's prefix is already sorted.
    fn slice_merge_sort<T, F>(v: &mut [T], mut sorted: usize, buf_ptr: *mut T, is_less: &mut F)
    where
        F: FnMut(&T, &T) -> bool,
    {
        let len = v.len();
        // find length of sorted prefix
        if sorted == 0 {
            sorted = if len <= 1 {
                len
            } else if gt!(v, 0, 1, is_less) {
                // strictly descending
                let mut i = 2;
                while i < len && gt!(v, i - 1, i, is_less) {
                    i += 1;
                }
                // Reverse the slice so we don't have to sort it later.
                v[..i].reverse();
                i
            } else {
                // ascending
                let mut i = 2;
                while i < len && !gt!(v, i - 1, i, is_less) {
                    i += 1;
                }
                i
            };
        }

        // Do merge sort, using `sorted` to avoid redundant sorting.
        if sorted < len {
            if len <= SMALL_SLICE_LEN + 2 {
                for i in sorted..len {
                    insert_end(&mut v[..=i], is_less);
                }
            } else {
                let mid;
                if len > SMALL_SLICE_LEN * 2 {
                    mid = sorted.max(len / 2);
                    if sorted < mid {
                        slice_merge_sort(&mut v[..mid], sorted, buf_ptr, is_less);
                    }
                    slice_merge_sort(&mut v[mid..], 0, buf_ptr, is_less);
                    if !gt!(v, mid - 1, mid, is_less) {
                        return;
                    } else if gt!(v, 0, len - 1, is_less) {
                        // strictly reverse sorted
                        unsafe {
                            swap_slices(v, mid, buf_ptr);
                        }
                        return;
                    }
                } else {
                    for i in sorted..SMALL_SLICE_LEN {
                        insert_end(&mut v[..=i], is_less);
                    }
                    for i in SMALL_SLICE_LEN + 1..len {
                        insert_end(&mut v[SMALL_SLICE_LEN..=i], is_less);
                    }
                    if !gt!(v, SMALL_SLICE_LEN - 1, SMALL_SLICE_LEN, is_less) {
                        return;
                    }
                    mid = SMALL_SLICE_LEN;
                }
                unsafe {
                    merge(v, mid, buf_ptr, is_less);
                }
            }
        }
    }

    /// swap contents of left-hand and right-hand slices divided at `mid`
    ///
    /// # Safety
    ///
    /// `buf_ptr` must point to a slice of `buf` which is long enough to hold `v[mid..]`
    ///   `v[mid..].len() <= buf.len()` because buf is sized as `(v.len() + 1) / 2' and `mid == sorted.max(v.len() / 2)`
    ///
    /// `mid` must be <= `v.len()`
    ///   `mid <= v.len()` because `swap_slices` is only called when: `sorted < len && mid == sorted.max(len / 2)`
    #[allow(unused_unsafe)]
    unsafe fn swap_slices<T>(v: &mut [T], mid: usize, buf_ptr: *mut T) {
        let rlen = v.len() - mid;
        let v_ptr = v.as_mut_ptr();
        unsafe {
            ptr::copy_nonoverlapping(v_ptr.add(mid), buf_ptr, rlen);
            ptr::copy(v_ptr, v_ptr.add(rlen), mid);
            ptr::copy_nonoverlapping(buf_ptr, v_ptr, rlen);
        }
    }
}
