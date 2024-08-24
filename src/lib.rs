#![cfg_attr(feature = "no_std", no_std)]

mod backend;

pub use backend::*;

#[cfg(all(test, feature = "no_std"))]
mod tests {
    use core::ffi::CStr;

    use crate::MmapWrapperBuilder;

    #[repr(C)]
    struct MyStruct {
        thing1: i32,
        thing2: f64,
    }

    #[test]
    fn basic_rw() {
        const PATH: &CStr = c"/tmp/mmap-wrapper-test";

        let rw_wrapper = MmapWrapperBuilder::<MyStruct>::new(PATH)
            .write(true)
            .truncate(true)
            .build_mut()
            .unwrap();

        let mut_inner = unsafe { rw_wrapper.get_inner() };
        mut_inner.thing1 = i32::MAX;
        mut_inner.thing2 = f64::MIN;

        let ro_wrapper = MmapWrapperBuilder::<MyStruct>::new(PATH).build().unwrap();
        let inner = unsafe { ro_wrapper.get_inner() };

        assert_eq!(inner.thing1, i32::MAX);
        assert_eq!(inner.thing2, f64::MIN);

        drop(ro_wrapper);

        mut_inner.thing1 = i32::MIN;
        mut_inner.thing2 = f64::MAX;

        assert_eq!(mut_inner.thing1, i32::MIN);
        assert_eq!(mut_inner.thing2, f64::MAX);
    }
}
