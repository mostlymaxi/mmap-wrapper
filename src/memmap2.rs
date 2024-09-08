use core::mem::transmute_copy;
use memmap2::{Mmap, MmapMut};
use std::{marker::PhantomData, sync::Arc};

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
    raw: Arc<Mmap>,
    _inner: PhantomData<T>,
}

impl<T> Clone for MmapWrapper<T> {
    fn clone(&self) -> Self {
        MmapWrapper {
            raw: self.raw.clone(),
            _inner: PhantomData,
        }
    }
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
    raw: Arc<MmapMut>,
    _inner: PhantomData<T>,
}

impl<T> Clone for MmapMutWrapper<T> {
    fn clone(&self) -> Self {
        MmapMutWrapper {
            raw: self.raw.clone(),
            _inner: PhantomData,
        }
    }
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
    /// # Safety
    /// the backing mmap pointer must point to valid
    /// memory for type T [T likely has to be repr(C)]
    pub fn new(m: Mmap) -> MmapWrapper<T> {
        // check that size of m matches
        // size of inner type
        MmapWrapper {
            raw: Arc::new(m),
            _inner: PhantomData,
        }
    }

    pub fn get_inner<'a>(&self) -> &'a T {
        unsafe { &*self.raw.as_ptr().cast::<T>() }
    }
}

impl<T> MmapMutWrapper<T> {
    /// # Safety
    /// the backing mmap pointer must point to valid
    /// memory for type T [T likely has to be repr(C)]
    pub unsafe fn new(m: MmapMut) -> MmapMutWrapper<T> {
        MmapMutWrapper {
            raw: Arc::new(m),
            _inner: PhantomData,
        }
    }

    pub fn get_inner<'a>(&mut self) -> &'a mut T {
        unsafe { &mut *self.raw.as_ptr().cast_mut().cast::<T>() }
    }
}

#[cfg(test)]
mod tests {

    struct TestStruct {
        _thing1: i32,
    }

    use std::{
        fs::{self, File},
        thread,
    };

    use crate::MmapMutWrapper;

    #[test]
    fn arc_thread_test() {
        let f = File::create_new("arc_thread_test").unwrap();
        f.set_len(size_of::<TestStruct>().try_into().unwrap())
            .unwrap();
        let m = unsafe { memmap2::MmapMut::map_mut(&f).unwrap() };
        let m: MmapMutWrapper<TestStruct> = unsafe { MmapMutWrapper::new(m) };

        let m_clone = m.clone();

        let t = thread::spawn(move || {
            let _ = m_clone;
        });

        let _ = t.join();

        drop(m);

        fs::remove_file("arc_thread_test").unwrap();
    }
}
