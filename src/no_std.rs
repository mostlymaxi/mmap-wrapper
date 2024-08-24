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
/// a common use case for mmaps in C is to cast the mmap backed pointer
/// to a struct such as:
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
/// this is a helpful wrapper for this use case:
/// ```rust
///  use mmap_wrapper::MmapWrapper;
///
///  // structs musthave a well defined layout,
///  // generally want them to be transparent or repr(C)
///  #[repr(C)]
///  struct MyStruct {
///     thing1: i32,
///     thing2: f64,
///  }
///
///  let m_wrapper = MmapWrapper::<MyStruct>::new(c"/tmp/mystruct-mmap-test.bin", /* write */ true, /* truncate */ true).unwrap();
///  let mmap_backed_mystruct = unsafe {
///     m_wrapper.get_inner()
///  };
/// ```
pub struct MmapWrapper<T> {
    raw: *mut c_void,
    _inner: PhantomData<T>,
}

/// # MmapMutWrapper
/// this is identical to [`MmapWrapper`] but returns a mutable reference instead.
///
/// this is a helpful wrapper for this use case:
/// ```rust
///  use mmap_wrapper::MmapMutWrapper;
///
///  // structs musthave a well defined layout,
///  // generally want them to be transparent or repr(C)
///  #[repr(C)]
///  struct MyStruct {
///     thing1: i32,
///     thing2: f64,
///  }
///
///  let mut m_wrapper = MmapMutWrapper::<MyStruct>::new(c"/tmp/mystruct-mmap-test.bin", /* write */ true, /* truncate */ true).unwrap();
///  let mmap_backed_mystruct = unsafe {
///     m_wrapper.get_inner()
///  };
///
///  mmap_backed_mystruct.thing1 = 123;
/// ```
pub struct MmapMutWrapper<T> {
    raw: *mut c_void,
    _inner: PhantomData<T>,
}

/// A builder for creating [`MmapWrapper`] or [`MmapMutWrapper`].
///
/// # Example
///
/// ```rust
/// use std::ffi::CStr;
/// use mmap_wrapper::MmapWrapperBuilder;
///
/// #[repr(C)]
/// struct MyStruct {
///    thing1: i32,
///    thing2: f64,
/// }
///
/// let m_wrapper = MmapWrapperBuilder::<MyStruct>::new(c"/tmp/mmap-mystruct-test.bin")
///     .write(true)
///     .truncate(false)
///     .build_mut()
///     .unwrap();
/// let mmap_backed_mystruct: &mut MyStruct = unsafe {
///     m_wrapper.get_inner()
/// };
/// ```
pub struct MmapWrapperBuilder<'a, T> {
    path: &'a CStr,
    write: bool,
    truncate: bool,
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
    fn map(path: &CStr, write: bool, truncate: bool) -> Result<*mut c_void, c_int> {
        let fd = unsafe {
            let flag = if write { O_RDWR } else { O_RDONLY };
            open(path.as_ptr(), O_CREAT | flag, 0o644)
        };
        if fd < 0 {
            return Err(fd);
        }

        if truncate {
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

        Ok(mapped_region)
    }

    pub fn new(path: &CStr, write: bool, truncate: bool) -> Result<MmapWrapper<T>, c_int> {
        Ok(MmapWrapper {
            raw: Self::map(path, write, truncate)?,
            _inner: PhantomData,
        })
    }

    // The std version uses std::io::Error for the error type so this isn't 1:1, can't do anything about it.
    pub fn make_mut(self) -> MmapMutWrapper<T> {
        MmapMutWrapper {
            raw: self.raw,
            _inner: PhantomData,
        }
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
    pub fn new(path: &CStr, write: bool, truncate: bool) -> Result<MmapMutWrapper<T>, c_int> {
        Ok(MmapMutWrapper {
            raw: MmapWrapper::<T>::map(path, write, truncate)?,
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

impl<'a, T> MmapWrapperBuilder<'a, T> {
    pub fn new(path: &'a CStr) -> MmapWrapperBuilder<T> {
        Self {
            path,
            write: false,
            truncate: false,
            _inner: PhantomData,
        }
    }

    pub fn write(mut self, value: bool) -> MmapWrapperBuilder<'a, T> {
        self.write = value;
        self
    }

    pub fn truncate(mut self, value: bool) -> MmapWrapperBuilder<'a, T> {
        self.truncate = value;
        self
    }

    pub fn build(self) -> Result<MmapWrapper<T>, c_int> {
        MmapWrapper::new(&self.path, self.write, self.truncate)
    }

    pub fn build_mut(self) -> Result<MmapMutWrapper<T>, c_int> {
        MmapMutWrapper::new(&self.path, self.write, self.truncate)
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
