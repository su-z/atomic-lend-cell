# AtomicLendCell

[https://crates.io/crates/atomic-lend-cell]()

A thread-safe Rust container that allows lending references to data across thread boundaries with clear ownership semantics.

## Why AtomicLendCell?

Rust's ownership model excels at preventing memory safety issues, but sometimes the borrow checker is too restrictive for certain patterns, especially across thread boundaries. When the compiler can't statically verify that borrowing relationships are safe, developers often turn to `Arc<T>` as a solution, but this comes with downsides:

1. **Shared ownership overhead**: `Arc<T>` creates shared ownership where any handle can keep data alive
2. **Reference counting cost**: Each clone and drop requires atomic operations
3. **Unclear ownership intent**: There's no distinction between "owner" and "borrower"

`AtomicLendCell` is designed for scenarios where:

- You have a clear primary owner of some data
- You know the owner will outlive all borrows
- You need to share references across threads or non-lexical scopes
- You want to express a lending relationship rather than shared ownership

## Features

- **Thread-safe borrowing**: Share immutable references across thread boundaries
- **Explicit ownership model**: Clear distinction between the owner and borrowers
- **Sync-only requirement**: Only requires `T: Sync`, not `T: Send + Sync`
- **Implementation options**: Choose between reference counting or flag-based approaches

## Usage

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
atomic-lend-cell = "0.1.0"
```

### Basic Example

```rust
use atomic_lend_cell::AtomicLendCell;
use std::thread;

fn main() {
    // Create a cell with some data
    let cell = AtomicLendCell::new(vec![1, 2, 3, 4, 5]);
  
    // Borrow the data
    let borrow = cell.borrow();
  
    // Use the borrow in another thread
    let handle = thread::spawn(move || {
        // Access the data through the borrow
        println!("Data in thread: {:?}", *borrow);
    });
  
    // Meanwhile, the original thread can still access the data
    println!("Original data: {:?}", *cell);
  
    // Wait for the thread to complete
    handle.join().unwrap();
}
```

### Implementation Options

This library offers two different implementations with different performance characteristics:

#### Reference Counting (similar to `Arc`)

```toml
[dependencies]
atomic-lend-cell = { version = "0.1.0", default-features = false, features = ["ref-counting"] }
```

This implementation:

- Tracks exact reference counts
- Provides stronger safety guarantees
- Has higher performance overhead due to atomic operations on each borrow/drop

#### Flag-based (default)

```toml
[dependencies]
atomic-lend-cell = "0.1.0"  # Uses flag-based by default
```

This implementation:

- Uses a single atomic flag instead of reference counting
- Has less overhead for borrowing operations
- Relies more heavily on correct usage patterns

## Safety

`AtomicLendCell` enforces safety by ensuring:

1. The owner cannot be dropped while borrows exist (debug builds)
2. Multiple threads can safely access the same data concurrently
3. Thread-safety is guaranteed through appropriate atomic operations

In debug builds, violations of the borrowing contract will cause panics to catch issues early. In release builds, some checks may be optimized away for performance.

## Safety Considerations

⚠️ **Important Safety Warning**

Both implementations will panic if the `AtomicLendCell` is dropped while active borrowers exist, however:

- **Reference counting implementation**: Will reliably panic as soon as the owner is dropped with active borrows, providing strong safety guarantees.
- **Flag-based implementation**: The panic is based on checking an atomic flag during specific operations. In rare cases with concurrent access across threads, a segmentation fault might occur before the panic is triggered, particularly in release builds or high-concurrency scenarios.

If your application requires absolute memory safety guarantees, consider:

1. Using the reference counting implementation (`ref-counting` feature)
2. Adding additional synchronization to ensure all borrows complete before dropping the owner
3. Using standard library alternatives like `Arc<T>` when appropriate

The flag-based implementation (default) prioritizes performance at the cost of some safety guarantees, so use it when you're confident about your borrowing patterns and ownership lifecycle.

## When to Use

`AtomicLendCell` is ideal for:

- Service objects that need to lend data to worker threads
- Long-lived objects created at program start that need to be accessed from multiple threads
- Data pipelines where ownership is clear but standard borrowing is too restrictive
- Scenarios where performance matters and `Arc` introduces too much overhead

## When Not to Use

Consider alternatives when:

- You need mutable access (consider `RwLock` or `Mutex`)
- Ownership is truly shared (use `Arc`)
- The borrowing relationship isn't clearly defined
- The owner might be dropped before borrows

## License

This project is licensed under the MIT License - see the LICENSE file for details.
