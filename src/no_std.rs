#[cfg(not(target_family = "unix"))]
compile_error!("no_std feature only supports unix based operating systems");

use core::ffi::{c_char, c_int, c_longlong, c_uint, c_void, CStr};
use core::marker::PhantomData;
use core::mem::size_of;
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

/// # MmapWrapper
///
/// A common use case for `mmap` in C is to cast the mmap backed region to a struct:
/// ```c
/// MyStruct* mmap_backed_mystruct;
/// int fd;
///
/// fd = open(path, O_RDWR | O_CREAT, 0644);
/// ftruncate(fd, sizeof(MyStruct));
///
/// mmap_backed_mystruct = (MyStruct*)mmap(0, sizeof(MyStruct), PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
/// ```
///
/// This is a helpful wrapper for the same usecase:
/// ```rust
/// use mmap_wrapper::MmapWrapper;
///
/// // Your struct MUST have a consistent memory layout.
/// // Use either #[repr(transparent)] or #[repr(C)].
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

/// # MmapMutWrapper
///
/// This is identical to [`MmapWrapper`] but returns a mutable reference instead.
///
/// A common use case for `mmap` in C is to cast the mmap backed region to a struct:
/// ```rust
/// use mmap_wrapper::MmapMutWrapper;
///
/// // Your struct MUST have a consistent memory layout.
/// // Use either #[repr(transparent)] or #[repr(C)].
/// #[repr(C)]
/// struct MyStruct {
///    thing1: i32,
///    thing2: f64,
/// }
///
/// let mut m_wrapper = MmapMutWrapper::<MyStruct>::new(c"/tmp/mystruct-mmap-test.bin").unwrap();
/// let mmap_backed_mystruct = unsafe {
///    m_wrapper.get_inner()
/// };
///
/// mmap_backed_mystruct.thing1 = 123;
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

    pub fn new(path: &CStr) -> Result<MmapWrapper<T>, c_int> {
        Ok(MmapWrapper {
            raw: Self::map(path, false)?,
            _inner: PhantomData,
        })
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
    pub unsafe fn get_inner<'a>(&self) -> &'a T {
        &*self.raw.cast::<T>()
    }
}

impl<T> MmapMutWrapper<T> {
    pub fn new(path: &CStr) -> Result<MmapMutWrapper<T>, c_int> {
        Ok(MmapMutWrapper {
            raw: MmapWrapper::<T>::map(path, true)?,
            _inner: PhantomData,
        })
    }

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
    pub unsafe fn get_inner<'a>(&self) -> &'a mut T {
        &mut *self.raw.cast::<T>()
    }
}

impl<T> Drop for MmapWrapper<T> {
    fn drop(&mut self) {
        unsafe {
            if self.raw != ptr::null_mut() {
                munmap(self.raw, size_of::<T>());
            }
        }
    }
}

impl<T> Drop for MmapMutWrapper<T> {
    fn drop(&mut self) {
        unsafe {
            if self.raw != ptr::null_mut() {
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
