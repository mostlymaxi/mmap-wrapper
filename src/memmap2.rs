use memmap2::{Mmap, MmapMut};
use std::marker::PhantomData;

/// A wrapper wrapper for a memory-mapped file with data of type `T`.
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
/// #[repr(C)]
/// struct MyStruct {
///    thing1: i32,
///    thing2: f64,
/// }
///
/// let f = std::fs::File::options()
///     .read(true)
///     .write(true)
///     .create(true)
///     .truncate(false)
///     .open("/tmp/mystruct-mmap-test.bin")
///     .unwrap();
///
/// let _ = f.set_len(std::mem::size_of::<MyStruct>() as u64);
///
/// let m = unsafe {
///     memmap2::Mmap::map(&f).unwrap()
/// };
///
/// let m_wrapper = MmapWrapper::<MyStruct>::new(m);
/// let mmap_backed_mystruct = unsafe {
///    m_wrapper.get_inner()
/// };
/// ```
pub struct MmapWrapper<T> {
    raw: Mmap,
    _inner: PhantomData<T>,
}

/// A mutable wrapper wrapper for a memory-mapped file with data of type `T`.
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
/// #[repr(C)]
/// struct MyStruct {
///    thing1: i32,
///    thing2: f64,
/// }
///
/// let f = std::fs::File::options()
///     .read(true)
///     .write(true)
///     .create(true)
///     .truncate(false)
///     .open("/tmp/mystruct-mmap-test.bin")
///     .unwrap();
///
/// let _ = f.set_len(std::mem::size_of::<MyStruct>() as u64);
///
/// let m = unsafe {
///     memmap2::Mmap::map(&f).unwrap()
/// };
///
/// let m_wrapper = MmapWrapper::<MyStruct>::new(m);
/// let mmap_backed_mystruct = unsafe {
///    m_wrapper.get_inner()
/// };
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
        unsafe { MmapMutWrapper::new(m) }
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
    /// # Safety
    /// the backing mmap pointer must point to valid
    /// memory for type T [T likely has to be repr(C)]
    pub unsafe fn new(m: MmapMut) -> MmapMutWrapper<T> {
        MmapMutWrapper {
            raw: m,
            _inner: PhantomData,
        }
    }

    pub fn get_inner<'a>(&mut self) -> &'a mut T {
        unsafe { &mut *self.raw.as_mut_ptr().cast::<T>() }
    }
}
