#![doc = include_str!("../README.md")]
#![cfg_attr(feature = "no_std", no_std)]

#[cfg(not(feature = "no_std"))]
pub mod memmap2;

#[cfg(feature = "no_std")]
pub mod no_std;

#[cfg(not(feature = "no_std"))]
pub use memmap2::*;

#[cfg(feature = "no_std")]
pub use no_std::*;
