# mmap wrapper

A common use case for `mmap` in C is to cast the mmap backed region to a struct:
```c
MyStruct* mmap_backed_mystruct;
int fd;

fd = open(path, O_RDWR | O_CREAT, 0644);
ftruncate(fd, sizeof(MyStruct));

mmap_backed_mystruct = (MyStruct*)mmap(0, sizeof(MyStruct), PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
```

# Example

This is a helpful wrapper for the same usecase:
```rust ignore
use mmap_wrapper::MmapWrapper;

// Your struct MUST have a consistent memory layout.
// Use either #[repr(transparent)] or #[repr(C)].
#[repr(C)]
struct MyStruct {
   thing1: i32,
   thing2: f64,
}

let f = std::fs::File::options()
    .read(true)
    .write(true)
    .create(true)
    .truncate(false)
    .open("/tmp/mystruct-mmap-test.bin")
    .unwrap();

let _ = f.set_len(std::mem::size_of::<MyStruct>() as u64);

let m = unsafe {
    memmap2::Mmap::map(&f).unwrap()
};

let m_wrapper = MmapWrapper::<MyStruct>::new(m);
let mmap_backed_mystruct = unsafe {
   m_wrapper.get_inner()
};
```

# `no_std` Example

```rust ignore
use mmap_wrapper::MmapWrapper;

// Your struct MUST have a consistent memory layout.
// Use either #[repr(transparent)] or #[repr(C)].
#[repr(C)]
struct MyStruct {
   thing1: i32,
   thing2: f64,
}

let m_wrapper = MmapWrapper::<MyStruct>::new(c"/tmp/mystruct-mmap-test.bin").unwrap();
let mmap_backed_mystruct = unsafe {
   m_wrapper.get_inner()
};
```
