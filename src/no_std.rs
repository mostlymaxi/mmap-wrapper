#[cfg(not(target_family = "unix"))]
compile_error!("no_std feature only supports unix based operating systems");

use core::ffi::{c_char, c_int, c_longlong, c_uint, c_void, CStr};
use core::marker::PhantomData;
use core::mem::size_of;
use core::mem::transmute_copy;
use core::ptr;

const O_RDONLY: c_int = 0;
const O_RDWR: c_int = 2;
const O_CREAT: c_int = 64;
const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
const MAP_SHARED: c_int = 1;
const MAP_FAILED: *mut c_void = !0 as *mut c_void;

#[allow(non_camel_case_types)]
type off_t = usize;

extern "C" {
    // Could technically support Linux 32bit large file support (i.e mmap64) but we're only mapping Sized structs so shrug
    fn open(pathname: *const c_char, flags: c_int, mode: c_uint) -> c_int;
    fn mmap(
        addr: *mut c_void,
        length: off_t,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: c_longlong,
    ) -> *mut c_void;
    fn close(fd: c_int) -> c_int;
    fn ftruncate(fd: c_int, length: c_longlong) -> c_int;
    fn munmap(addr: *mut c_void, length: off_t) -> c_int;
}

/// A wrapper for a memory-mapped file with data of type `T`.
///
/// # Safety
///
/// `T` must have a consistent memory layout to ensure that the data is casted correctly.
///
/// Use `#[repr(transparent)]` if `T` is a newtype wrapper around a single field otherwise `#[repr(C)]`.
///
/// # Example
/// ```rust
/// use mmap_wrapper::MmapWrapper;
///
/// // repr(C) for consistent memory layout
/// #[repr(C)]
/// struct MyStruct {
///    thing1: i32,
///    thing2: f64,
/// }
///
/// let m_wrapper = MmapWrapper::<MyStruct>::new(c"/tmp/mystruct-mmap-test.bin").unwrap();
/// let mmap_backed_mystruct = unsafe {
///    m_wrapper.get_inner()
/// };
/// ```
pub struct MmapWrapper<T> {
    raw: *mut c_void,
    _inner: PhantomData<T>,
}

/// A mutable wrapper for a memory-mapped file with data of type `T`.
///
/// # Safety
///
/// `T` must have a consistent memory layout to ensure that the data is casted correctly.
///
/// Use `#[repr(transparent)]` if `T` is a newtype wrapper around a single field otherwise `#[repr(C)]`.
///
/// # Example
/// ```rust
/// use mmap_wrapper::MmapWrapper;
///
/// // repr(C) for consistent memory layout
/// #[repr(C)]
/// struct MyStruct {
///    thing1: i32,
///    thing2: f64,
/// }
///
/// let m_wrapper = MmapWrapper::<MyStruct>::new(c"/tmp/mystruct-mmap-test.bin").unwrap();
/// let mmap_backed_mystruct = unsafe {
///    m_wrapper.get_inner()
/// };
/// ```
pub struct MmapMutWrapper<T> {
    raw: *mut c_void,
    _inner: PhantomData<T>,
}

impl<T> MmapWrapper<T> {
    /// Maps a file to memory, creating it if necessary.
    ///
    /// # Safety
    ///
    /// - The `T` type must be `#[repr(C)]` or `#[repr(transparent)]` to ensure a consistent memory layout.
    /// - The file is truncated to the size of `T`. If this fails, the file descriptor is closed, and an error is returned.
    /// - Memory mapping is done with either (`PROT_READ | PROT_WRITE` or `PROT_READ`) and `MAP_SHARED`. If the mapping fails, the file descriptor is closed, and an error is returned.
    ///
    /// # Errors
    ///
    /// - Returns `Err` if the file cannot be opened, truncated, or mapped.
    /// - Returns `Err(-1)` specifically if memory mapping fails.
    fn map(path: &CStr, write: bool) -> Result<*mut c_void, c_int> {
        let fd = unsafe {
            let flag = if write { O_RDWR } else { O_RDONLY };
            open(path.as_ptr(), O_CREAT | flag, 0o644)
        };
        if fd < 0 {
            return Err(fd);
        }

        if write {
            let res = unsafe { ftruncate(fd, size_of::<T>() as c_longlong) };
            if res < 0 {
                unsafe { close(fd) };
                return Err(res);
            }
        }

        let mmap_prot = if write {
            PROT_READ | PROT_WRITE
        } else {
            PROT_READ
        };
        let mapped_region = unsafe {
            mmap(
                ptr::null_mut(),
                size_of::<T>(),
                mmap_prot,
                MAP_SHARED,
                fd,
                0,
            )
        };

        if mapped_region == MAP_FAILED {
            unsafe { close(fd) };
            return Err(-1);
        }

        unsafe { close(fd) };

        Ok(mapped_region)
    }

    /// Retrieves a reference to the inner value of type `T` from the mapped memory.
    ///
    /// # Safety
    ///
    /// - The function assumes that `self.raw` is a valid region aligned to `size_of::<T>()`.
    /// - The caller must ensure that `T` has a consistent layout by using `#[repr(transparent)]` or `#[repr(C)]`.
    ///
    /// # Panics
    ///
    /// This function is `unsafe` and does not perform any checks, so it may lead to undefined behavior if the safety guarantees are not met.
    pub fn new(path: &CStr) -> Result<MmapWrapper<T>, c_int> {
        Ok(MmapWrapper {
            raw: Self::map(path, false)?,
            _inner: PhantomData,
        })
    }

    pub fn get_inner<'a>(&self) -> &'a T {
        unsafe { &*self.raw.cast::<T>() }
    }
}

impl<T> Clone for MmapMutWrapper<T> {
    fn clone(&self) -> Self {
        // this is horrifying
        MmapMutWrapper {
            raw: unsafe { transmute_copy(&self.raw) },
            _inner: PhantomData,
        }
    }
}

impl<T> Clone for MmapWrapper<T> {
    fn clone(&self) -> Self {
        // this is horrifying
        MmapWrapper {
            raw: unsafe { transmute_copy(&self.raw) },
            _inner: PhantomData,
        }
    }
}

impl<T> MmapMutWrapper<T> {
    /// Retrieves a mutable reference to the inner value of type `T` from the mapped memory.
    ///
    /// # Safety
    ///
    /// - The function assumes that `self.raw` is a valid region aligned to `size_of::<T>()`.
    /// - The caller must ensure that `T` has a consistent layout by using `#[repr(transparent)]` or `#[repr(C)]`.
    ///
    /// # Panics
    ///
    /// This function is `unsafe` and does not perform any checks, so it may lead to undefined behavior if the safety guarantees are not met.
    pub unsafe fn new(path: &CStr) -> Result<MmapMutWrapper<T>, c_int> {
        Ok(MmapMutWrapper {
            raw: MmapWrapper::<T>::map(path, true)?,
            _inner: PhantomData,
        })
    }

    pub fn get_inner<'a>(&self) -> &'a mut T {
        unsafe { &mut *self.raw.cast::<T>() }
    }
}

impl<T> Drop for MmapWrapper<T> {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                munmap(self.raw, size_of::<T>());
            }
        }
    }
}

impl<T> Drop for MmapMutWrapper<T> {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                munmap(self.raw, size_of::<T>());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use core::ffi::CStr;

    use crate::{MmapMutWrapper, MmapWrapper};

    #[repr(C)]
    struct MyStruct {
        thing1: i32,
        thing2: f64,
    }

    #[test]
    fn basic_rw() {
        const PATH: &CStr = c"/tmp/mmap-wrapper-test";

        let rw_wrapper = MmapMutWrapper::<MyStruct>::new(PATH).unwrap();

        let mut_inner = unsafe { rw_wrapper.get_inner() };
        mut_inner.thing1 = i32::MAX;
        mut_inner.thing2 = f64::MIN;

        let ro_wrapper = MmapWrapper::<MyStruct>::new(PATH).unwrap();
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
