pub mod atomic_counting;
pub mod flag_based;

// Export the implementation based on the selected feature
#[cfg(feature = "ref-counting")]
pub use atomic_counting::*;

#[cfg(feature = "flag-based")]
pub use flag_based::*;

// If neither feature is explicitly selected, use the default (flag-based)
#[cfg(all(not(feature = "ref-counting"), not(feature = "flag-based")))]
pub use flag_based::*;
