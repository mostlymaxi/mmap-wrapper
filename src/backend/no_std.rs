use core::ffi::{c_char, c_int, c_longlong, c_uint, c_void, CStr};
use core::marker::PhantomData;
use core::mem::size_of;
use core::ptr;

// TODO: Try to fix where it's not 1:1 compatible with the memmap2 feature, it's okay if certain things just can't be though.
// TODO: More control to the person using it (i.e don't hardcode protection and fd creation flags)
// TODO: Ensure no leaks
// TODO: More documentation / Fix original crate documentation to be in-line with having a no_std feature and show-
// examples for it too
// TODO: For fd creation flags and protection try to make it user-proof (i.e enum for flags so the user can't input any custom value they want)

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

pub struct MmapWrapper<T> {
    raw: *mut c_void,
    _inner: PhantomData<T>,
}

pub struct MmapMutWrapper<T> {
    raw: *mut c_void,
    _inner: PhantomData<T>,
}

impl<T> MmapWrapper<T> {
    /// Maps a file to memory, creating it if necessary.
    ///
    /// # Safety
    ///
    /// - The `path` argument must be a valid, null-terminated C string representing the file path.
    /// - The `T` type must be a `#[repr(C)]` struct to ensure correct memory layout.
    /// - The caller must ensure that the size of `T` is appropriate for the intended use of the mapped region.
    /// - The function opens the file with read/write permissions and creates it if it does not exist. The file's permissions are set to `0o644`.
    /// - The file is truncated to the size of `T`. If this fails, the file descriptor is closed, and an error is returned.
    /// - Memory mapping is done with `PROT_READ | PROT_WRITE` and `MAP_SHARED`. If the mapping fails, the file descriptor is closed, and an error is returned.
    /// - The caller is responsible for unmapping the memory region using `munmap` and closing the file descriptor with `close` when done.
    ///
    /// # Errors
    ///
    /// - Returns `Err` if the file cannot be opened, truncated, or mapped.
    /// - Returns `Err(-1)` specifically if memory mapping fails.
    fn map(path: &CStr) -> Result<*mut c_void, c_int> {
        let fd = unsafe { open(path.as_ptr(), O_RDWR | O_CREAT, 0o644) };
        if fd < 0 {
            return Err(fd);
        }

        let res = unsafe { ftruncate(fd, size_of::<T>() as c_longlong) };
        if res < 0 {
            unsafe { close(fd) };
            return Err(res);
        }

        let mapped_region = unsafe {
            mmap(
                ptr::null_mut(),
                size_of::<T>(),
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                fd,
                0,
            )
        };

        if mapped_region == MAP_FAILED {
            unsafe { close(fd) };
            return Err(-1);
        }

        Ok(mapped_region)
    }

    pub fn new(path: &CStr) -> Result<MmapWrapper<T>, c_int> {
        Ok(MmapWrapper {
            raw: Self::map(path)?,
            _inner: PhantomData,
        })
    }

    // The std version uses std::io::Error for the error type so this isn't perfectly 1:1, can't do anything about it.
    pub fn make_mut(self) -> Result<MmapMutWrapper<T>, ()> {
        Ok(MmapMutWrapper {
            raw: self.raw,
            _inner: PhantomData,
        })
    }

    /// Retrieves a reference to the inner value of type `T` from the mapped memory.
    ///
    /// # Safety
    ///
    /// - The `self.raw` pointer must be a valid, properly aligned pointer to a memory-mapped region that contains a valid instance of `T`.
    /// - The memory mapped must be valid and correctly represent an instance of `T`, which should typically have `#[repr(C)]` to ensure a consistent layout.
    /// - The caller must ensure that `self.raw` points to valid memory and that the memory region has not been modified or invalidated elsewhere.
    /// - The function assumes that the memory pointed to by `self.raw` is initialized and correctly aligned for `T`.
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
            raw: MmapWrapper::<T>::map(path)?,
            _inner: PhantomData,
        })
    }

    /// Retrieves a mutable reference to the inner value of type `T` from the mapped memory.
    ///
    /// # Safety
    ///
    /// - The `self.raw` pointer must be a valid, properly aligned pointer to a memory-mapped region that contains a valid instance of `T`.
    /// - The memory mapped must be valid and correctly represent an instance of `T`, which should typically have `#[repr(C)]` to ensure a consistent layout.
    /// - The caller must ensure that `self.raw` points to valid memory and that the memory region has not been modified or invalidated elsewhere.
    /// - The function assumes that the memory pointed to by `self.raw` is initialized and correctly aligned for `T`.
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
