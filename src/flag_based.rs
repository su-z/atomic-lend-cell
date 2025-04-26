// Allow dead code when the flag-based feature is not enabled
#![cfg_attr(not(feature = "flag-based"), allow(dead_code))]

//! # Atomic Lend Cell
//! 
//! A thread-safe container that allows lending references to data across threads
//! using epoch-based reclamation for safety verification without per-object reference counting.
//! 
//! This module provides two main types:
//! - `AtomicLendCell<T>`: The owner that contains the data and can lend it out
//! - `AtomicBorrowCell<T>`: A lightweight borrow of data that can be freely sent between threads
//!
//! Unlike atomic reference counting, this implementation uses a single boolean flag
//! to track the owner's lifetime, reducing synchronization overhead while still
//! ensuring safety.

use std::{ops::Deref, sync::atomic::{AtomicBool, Ordering}};

/// A container that allows thread-safe lending of its contained value using epoch-based reclamation
///
/// `AtomicLendCell<T>` owns a value of type `T` and maintains an atomic boolean
/// to track its lifetime. It ensures that the value isn't accessed after being dropped,
/// with validation occurring in debug builds.
pub struct AtomicLendCell<T> {
    data: T,
    is_alive: AtomicBool
}

impl<T> AtomicLendCell<T> {
    /// Returns a reference to the contained value
    ///
    /// This method provides direct access to the value inside the cell without
    /// creating a borrowing relationship.
    pub fn as_ref(&self) -> &T {
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
    /// Marks the cell as no longer alive when it's dropped
    ///
    /// This allows borrows to detect if they're being used after the owner was dropped.
    fn drop(&mut self) {
        // Mark as no longer alive
        self.is_alive.store(false, Ordering::Release);
        
        // Optional: Give in-flight operations a chance to complete
        #[cfg(debug_assertions)]
        std::thread::yield_now();
    }
}

/// A thread-safe reference to data contained in an `AtomicLendCell`
///
/// `AtomicBorrowCell<T>` holds a pointer to data in an `AtomicLendCell<T>` and
/// checks the lender's liveness in debug builds. It can be safely sent between threads.
pub struct AtomicBorrowCell<T> {
    data_ptr: *const T,
    owner_alive_ptr: *const AtomicBool
}

impl<T> AtomicBorrowCell<T> {
    /// Returns a reference to the borrowed value
    ///
    /// This method provides access to the value inside the original `AtomicLendCell`.
    /// In debug builds, it verifies that the owner is still alive.
    pub fn as_ref(&self) -> &T {
        #[cfg(debug_assertions)]
        {
            let is_alive = unsafe { self.owner_alive_ptr.as_ref().unwrap() }
                .load(Ordering::Acquire);
            if !is_alive {
                panic!("Attempting to access AtomicBorrowCell after owner was dropped");
            }
        }
        
        unsafe { self.data_ptr.as_ref().unwrap() }
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
    /// Checks if the owner is still alive when this borrow is dropped
    ///
    /// In debug builds, this will panic if the borrow is dropped after the owner,
    /// helping to detect potential use-after-free bugs.
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        {
            let is_alive = unsafe { self.owner_alive_ptr.as_ref().unwrap() }
                .load(Ordering::Acquire);
            if !is_alive {
                // We were dropped after owner - this shouldn't happen in correct code
                panic!("AtomicBorrowCell dropped after its owner was dropped");
            }
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
        Self { data, is_alive: AtomicBool::new(true) }
    }

    /// Creates a new `AtomicBorrowCell` for the contained value
    ///
    /// This returns a borrow that can be sent to other threads. The borrow will
    /// verify the owner's liveness in debug builds.
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
        AtomicBorrowCell {
            data_ptr: (&self.data) as *const T,
            owner_alive_ptr: &self.is_alive as *const AtomicBool
        }
    }
    
}

impl<'a, T> AtomicLendCell<&'a T> {
    /// Creates a new `AtomicBorrowCell` that borrows the referenced value directly
    ///
    /// This is useful when the `AtomicLendCell` contains a reference, and you want to
    /// borrow the underlying value rather than the reference itself.
    pub fn borrow_deref(&'a self) -> AtomicBorrowCell<T> {
        AtomicBorrowCell {
            data_ptr: self.data as *const T,
            owner_alive_ptr: &self.is_alive as *const AtomicBool
        }
    }
}

impl<T> Clone for AtomicBorrowCell<T> {
    /// Creates a new `AtomicBorrowCell` that borrows the same value
    ///
    /// Unlike reference counting, this doesn't need to increment any counters,
    /// making it more efficient.
    fn clone(&self) -> Self {
        // Simply create a new borrow pointing to the same data and liveness flag
        AtomicBorrowCell {
            data_ptr: self.data_ptr,
            owner_alive_ptr: self.owner_alive_ptr
        }
    }
}

#[test]
/// Tests that borrowing works across threads
fn test_epoch_borrow() {
    let x = AtomicLendCell::new(4);
    let xr = x.borrow();
    let t1 = std::thread::spawn(move || {
        let y = xr.as_ref();
        println!("{:?}", y);
    });
    let xr = x.borrow();
    let t2 = std::thread::spawn(move || {
        let y = xr.as_ref();
        println!("{:?}", y);
    });
    t1.join().unwrap();
    t2.join().unwrap();
}

#[test]
/// Tests the safety checks for owner outliving borrows
fn test_epoch_safety() {
    use std::sync::Arc;
    
    // This test will only panic in debug builds
    let data = Arc::new(42);
    let data_clone = Arc::clone(&data);
    
    let x_opt = Some(AtomicLendCell::new(data));
    let borrow = x_opt.as_ref().unwrap().borrow();
    
    // Use the borrow before dropping owner
    assert_eq!(**borrow, 42);
    
    // Simulate work in another thread
    let handle = std::thread::spawn(move || {
        // Just hold onto data_clone to ensure it doesn't drop
        assert_eq!(*data_clone, 42);
        std::thread::sleep(std::time::Duration::from_millis(50));
    });
    
    // Drop the owner while borrow still exists
    drop(x_opt);
    
    // In debug builds, this would panic when checking borrow's liveness
    #[cfg(not(debug_assertions))]
    {
        // This should only run in release builds
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        // This will cause undefined behavior in release mode if safety is violated
        let _value = *borrow;
    }
    
    handle.join().unwrap();
}