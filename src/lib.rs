//! # MmapWrapper
//! a common use case for mmaps in C is to cast the mmap backed pointer
//! to a struct such as:
//! ```c
//! MyStruct* mmap_backed_mystruct;
//! int fd;
//!
//! fd = open(path, O_RDWR | O_CREAT, 0644);
//! ftruncate(fd, sizeof(MyStruct));
//!
//! mmap_backed_mystruct = (MyStruct*)mmap(0, sizeof(MyStruct), PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
//! ```
//!
//! this is a helpful wrapper for this use case:
//! ```rust
//!  use mmap_wrapper::MmapWrapper;
//!
//!  // structs musthave a well defined layout,
//!  // generally want them to be transparent or repr(C)
//!  #[repr(C)]  
//!  struct MyStruct {
//!     thing1: i32,
//!     thing2: f64,
//!  }
//!
//!  let f = std::fs::File::options()
//!      .read(true)
//!      .write(true)
//!      .create(true)
//!      .truncate(false)
//!      .open("/tmp/mystruct-mmap-test.bin")
//!      .unwrap();
//!   
//!  let _ = f.set_len(std::mem::size_of::<MyStruct>() as u64);
//!
//!  let m = unsafe {
//!      memmap2::Mmap::map(&f).unwrap()
//!  };
//!
//!  let m_wrapper = MmapWrapper::<MyStruct>::new(m);
//!  let mmap_backed_mystruct = unsafe {
//!     m_wrapper.get_inner()
//!  };
//! ```
use memmap2::{Mmap, MmapMut};
use std::marker::PhantomData;

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
///  let f = std::fs::File::options()
///      .read(true)
///      .write(true)
///      .create(true)
///      .truncate(false)
///      .open("/tmp/mystruct-mmap-test.bin")
///      .unwrap();
///   
///  let _ = f.set_len(std::mem::size_of::<MyStruct>() as u64);
///
///  let m = unsafe {
///      memmap2::Mmap::map(&f).unwrap()
///  };
///
///  let m_wrapper = MmapWrapper::<MyStruct>::new(m);
///  let mmap_backed_mystruct = unsafe {
///     m_wrapper.get_inner()
///  };
/// ```
pub struct MmapWrapper<T> {
    raw: Mmap,
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
///  let f = std::fs::File::options()
///      .read(true)
///      .write(true)
///      .create(true)
///      .truncate(false)
///      .open("/tmp/mystruct-mmap-test.bin")
///      .unwrap();
///   
///  let _ = f.set_len(std::mem::size_of::<MyStruct>() as u64);
///
///  let m = unsafe {
///      memmap2::MmapMut::map_mut(&f).unwrap()
///  };
///
///  let mut m_wrapper = MmapMutWrapper::<MyStruct>::new(m);
///  let mmap_backed_mystruct = unsafe {
///     m_wrapper.get_inner()
///  };
///
///  mmap_backed_mystruct.thing1 = 123;
/// ```
pub struct MmapMutWrapper<T> {
    raw: MmapMut,
    _inner: PhantomData<T>,
}

impl<T> From<Mmap> for MmapWrapper<T> {
    fn from(m: Mmap) -> MmapWrapper<T> {
        MmapWrapper::new(m)
    }
}

impl<T> From<MmapMut> for MmapMutWrapper<T> {
    fn from(m: MmapMut) -> MmapMutWrapper<T> {
        MmapMutWrapper::new(m)
    }
}

impl<T> MmapWrapper<T> {
    pub fn new(m: Mmap) -> MmapWrapper<T> {
        // check that size of m matches
        // size of inner type
        MmapWrapper {
            raw: m,
            _inner: PhantomData,
        }
    }

    pub fn make_mut(self) -> Result<MmapMutWrapper<T>, std::io::Error> {
        Ok(MmapMutWrapper {
            raw: self.raw.make_mut()?,
            _inner: PhantomData,
        })
    }

    /// # Safety
    /// the backing mmap pointer must point to valid
    /// memory for type T [T likely has to be repr(C)]
    pub unsafe fn get_inner<'a>(&self) -> &'a T {
        unsafe { &*self.raw.as_ptr().cast::<T>() }
    }
}

impl<T> MmapMutWrapper<T> {
    pub fn new(m: MmapMut) -> MmapMutWrapper<T> {
        // check that size of m matches
        // size of inner type
        MmapMutWrapper {
            raw: m,
            _inner: PhantomData,
        }
    }

    /// # Safety
    /// the backing mmap pointer must point to valid
    /// memory for type T [T likely has to be repr(C)]
    pub unsafe fn get_inner<'a>(&mut self) -> &'a mut T {
        unsafe { &mut *self.raw.as_mut_ptr().cast::<T>() }
    }
}
