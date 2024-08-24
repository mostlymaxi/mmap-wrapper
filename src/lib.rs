#![cfg_attr(feature = "no_std", no_std)]

#[cfg(not(any(feature = "memmap2", feature = "no_std")))]
compile_error!("You must have one of [memmap2, no_std] enabled");

#[cfg(not(any(
    all(feature = "memmap2", not(any(feature = "no_std"))),
    all(feature = "no_std", not(any(feature = "memmap2"))),
)))]
compile_error!("You must only have one of [memmap2, no_std] enabled");

mod backend;

pub use backend::*;

#[cfg(test)]
mod tests {
    use crate::{MmapMutWrapper, MmapWrapper};

    #[repr(C)]
    struct MyStruct {
        thing1: i32,
        thing2: f64,
    }

    #[test]
    fn test() {
        let wrapper = MmapMutWrapper::<MyStruct>::new(c"/tmp/mmap-wrapper-test").unwrap();
        let ro_wrapper = MmapWrapper::<MyStruct>::new(c"/tmp/mmap-wrapper-test").unwrap();
        let mut_inner = unsafe { wrapper.get_inner() };
        let inner = unsafe { ro_wrapper.get_inner() };

        mut_inner.thing1 = i32::MAX;
        mut_inner.thing2 = f64::MIN;

        assert_eq!(inner.thing1, i32::MAX);
        assert_eq!(inner.thing2, f64::MIN);
    }
}
