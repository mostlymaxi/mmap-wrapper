#[cfg(not(feature = "no_std"))]
mod memmap2;

#[cfg(feature = "no_std")]
mod no_std;

#[cfg(not(feature = "no_std"))]
pub use memmap2::*;

#[cfg(feature = "no_std")]
pub use no_std::*;
