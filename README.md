# mmap wrapper
a simple wrapper for the memmap2 crate to cast mmap backed pointers to
structs.

# Example
a common use case for mmaps in C is to cast the mmap backed pointer
to a struct such as:
```c
MyStruct* mmap_backed_mystruct;
int fd;

fd = open(path, O_RDWR | O_CREAT, 0644);
ftruncate(fd, sizeof(MyStruct));

mmap_backed_mystruct = (MyStruct*)mmap(0, sizeof(MyStruct), PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
```

this is a helpful wrapper for this use case:
```rust
 use mmap_wrapper::MmapWrapper;

 // structs musthave a well defined layout,
 // generally want them to be transparent or repr(C)
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

