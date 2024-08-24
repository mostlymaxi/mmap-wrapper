#[cfg(feature = "memmap2")]
mod memmap2;

#[cfg(feature = "no_std")]
mod no_std;

#[cfg(feature = "memmap2")]
pub use memmap2::*;

#[cfg(feature = "no_std")]
pub use no_std::*;
