// Allow dead code when the ref-counting feature is not enabled
#![cfg_attr(not(feature = "ref-counting"), allow(dead_code))]

//! # Atomic Lend Cell
//! 
//! A thread-safe container that allows lending references to data across threads
//! with atomic reference counting for safe resource management.
//! 
//! This crate provides two main types:
//! - `AtomicLendCell<T>`: The owner that contains the data and can lend it out
//! - `AtomicBorrowCell<T>`: A reference-counted borrow of data that can be freely cloned and sent between threads
//!
//! Unlike standard Rust borrowing, `AtomicLendCell` allows multiple threads to access
//! the same data simultaneously, while ensuring the original value outlives all borrows.

use std::{ops::Deref, sync::atomic::{AtomicUsize, Ordering}};

/// A container that allows thread-safe lending of its contained value
///
/// `AtomicLendCell<T>` owns a value of type `T` and maintains an atomic reference count
/// to track outstanding borrows. It ensures that the value isn't dropped while
/// borrows exist, panicking if this invariant would be violated.
pub struct AtomicLendCell<T> {
    data: T,
    refcount: AtomicUsize
}

impl<T> AtomicLendCell<T> {
    /// Returns a reference to the contained value
    ///
    /// This method provides direct access to the value inside the cell without
    /// incrementing the reference counter.
    pub fn as_ref(&self) -> &T{
        &self.data
    }
}

impl<T> Deref for AtomicLendCell<T> {
    type Target = T;
    /// Dereferences to the contained value
    ///
    /// This provides convenient access to the contained value through the dereference operator (*).
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> Drop for AtomicLendCell<T> {
    /// Ensures no borrows exist when the cell is dropped
    ///
    /// If outstanding borrows exist when the cell is dropped, this will panic
    /// to prevent use-after-free errors.
    fn drop(&mut self) {
        if self.refcount.load(Ordering::Relaxed) > 0 {
            panic!("An AtomicBorrowCell outlives the AtomicLendCell which issues it!");
        }
    }
}

/// A thread-safe reference to data contained in an `AtomicLendCell`
///
/// `AtomicBorrowCell<T>` holds a pointer to data in an `AtomicLendCell<T>` and
/// automatically decrements the reference count when dropped. It can be safely
/// cloned, sent between threads, and shared.
pub struct AtomicBorrowCell<T> {
    data_ptr: *const T,
    refcount_ptr: *const AtomicUsize
}

impl<T> AtomicBorrowCell<T> {
    /// Returns a reference to the borrowed value
    ///
    /// This method provides access to the value inside the original `AtomicLendCell`.
    pub fn as_ref(&self) -> &T{
        unsafe {self.data_ptr.as_ref().unwrap()}
    }
}

impl<T> Deref for AtomicBorrowCell<T> {
    type Target = T;
    /// Dereferences to the borrowed value
    ///
    /// This provides convenient access to the borrowed value through the dereference operator (*).
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> Drop for AtomicBorrowCell<T> {
    /// Decrements the reference count when the borrow is dropped
    fn drop(&mut self) {
        unsafe {
            self.refcount_ptr.as_ref().unwrap().fetch_sub(1, Ordering::Release);
        }
    }
}

// These trait implementations make `AtomicBorrowCell` safe to send between threads
unsafe impl<T: Sync> Send for AtomicBorrowCell<T> {}
unsafe impl<T: Sync> Sync for AtomicBorrowCell<T> {}

impl<T> AtomicLendCell<T> {
    /// Creates a new `AtomicLendCell` containing the given value
    ///
    /// # Examples
    ///
    /// ```
    /// use atomic_lend_cell::AtomicLendCell;
    ///
    /// let cell = AtomicLendCell::new(42);
    /// ```
    pub fn new(data: T) -> Self {
        Self {data, refcount: 0.into()}
    }

    /// Creates a new `AtomicBorrowCell` for the contained value
    ///
    /// This increments the internal reference count and returns a borrow that can
    /// be sent to other threads. The borrow will automatically decrement the
    /// reference count when dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use atomic_lend_cell::AtomicLendCell;
    ///
    /// let cell = AtomicLendCell::new(42);
    /// let borrow = cell.borrow();
    ///
    /// assert_eq!(*borrow, 42);
    /// ```
    pub fn borrow(&self) -> AtomicBorrowCell<T> {
        self.refcount.fetch_add(1, Ordering::Acquire);
        AtomicBorrowCell {data_ptr: (&self.data) as * const T, refcount_ptr: &self.refcount as * const AtomicUsize}
    }
}

impl<'a, T> AtomicLendCell<&'a T> {
    /// Creates a new `AtomicBorrowCell` that borrows the referenced value directly
    ///
    /// This is useful when the `AtomicLendCell` contains a reference, and you want to
    /// borrow the underlying value rather than the reference itself.
    pub fn borrow_deref(&'a self) -> AtomicBorrowCell<T> {
        self.refcount.fetch_add(1, Ordering::Acquire);
        AtomicBorrowCell {data_ptr: self.data as * const T, refcount_ptr: &self.refcount as * const AtomicUsize}
    }
}

impl<T> Clone for AtomicBorrowCell<T> {
    /// Creates a new `AtomicBorrowCell` that borrows the same value
    ///
    /// This increments the reference count in the original `AtomicLendCell`.
    fn clone(&self) -> Self {
        let count = unsafe {self.refcount_ptr.as_ref()}.unwrap();
        count.fetch_add(1, Ordering::SeqCst);
        AtomicBorrowCell {data_ptr: self.data_ptr, refcount_ptr: self.refcount_ptr}
    }
}

#[test]
/// Tests that borrowing works across threads
fn test_lambda_borrow(){
    let x = AtomicLendCell::new(4);
    let xr = x.borrow();
    let t1 = std::thread::spawn(move ||{
        let y = xr.as_ref();
        println!("{:?}", y);
    });
    let xr = x.borrow();
    let t2 = std::thread::spawn(move ||{
        let y = xr.as_ref();
        println!("{:?}", y);
    });
    t1.join().unwrap();
    t2.join().unwrap();
}
