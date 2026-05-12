#[cfg(all(not(target_os = "windows"), not(miri)))]
mod not_for_windows {
    use wasmtime::*;

    use rustix::mm::{MapFlags, MprotectFlags, ProtFlags, mmap_anonymous, mprotect, munmap};

    use std::ptr::null_mut;
    use std::sync::{Arc, Mutex};

    struct CustomMemory {
        mem: usize,
        size: usize,
        guard_size: usize,
        used_wasm_bytes: usize,
        needs_init: bool,
        glob_bytes_counter: Arc<Mutex<usize>>,
    }

    impl CustomMemory {
        unsafe fn new(
            minimum: usize,
            maximum: usize,
            glob_counter: Arc<Mutex<usize>>,
            needs_init: bool,
            initial_contents: &[(usize, Vec<u8>)],
        ) -> Self {
            let page_size = rustix::param::page_size();
            let guard_size = page_size;
            let size = maximum + guard_size;
            // We rely on the Wasm page size being multiple of host page size.
            assert_eq!(size % page_size, 0);

            let mem = unsafe {
                mmap_anonymous(null_mut(), size, ProtFlags::empty(), MapFlags::PRIVATE)
                    .expect("mmap failed")
            };

            // NOTE: mmap_anonymous returns zero initialized memory, which is relied upon by this
            // API.

            unsafe {
                mprotect(mem, minimum, MprotectFlags::READ | MprotectFlags::WRITE)
                    .expect("mprotect failed");
            }
            for (offset, bytes) in initial_contents {
                assert!(offset + bytes.len() <= minimum);
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        (mem as *mut u8).add(*offset),
                        bytes.len(),
                    );
                }
            }
            *glob_counter.lock().unwrap() += minimum;

            Self {
                mem: mem as usize,
                size,
                guard_size,
                used_wasm_bytes: minimum,
                needs_init,
                glob_bytes_counter: glob_counter,
            }
        }
    }

    impl Drop for CustomMemory {
        fn drop(&mut self) {
            *self.glob_bytes_counter.lock().unwrap() -= self.used_wasm_bytes;
            unsafe { munmap(self.mem as *mut _, self.size).expect("munmap failed") };
        }
    }

    unsafe impl LinearMemory for CustomMemory {
        fn byte_size(&self) -> usize {
            self.used_wasm_bytes
        }

        fn byte_capacity(&self) -> usize {
            self.size - self.guard_size
        }

        fn grow_to(&mut self, new_size: usize) -> wasmtime::Result<()> {
            println!("grow to {new_size:x}");
            let delta = new_size - self.used_wasm_bytes;
            unsafe {
                let start = (self.mem as *mut u8).add(self.used_wasm_bytes) as _;
                mprotect(start, delta, MprotectFlags::READ | MprotectFlags::WRITE)
                    .expect("mprotect failed");
            }

            *self.glob_bytes_counter.lock().unwrap() += delta;
            self.used_wasm_bytes = new_size;
            Ok(())
        }

        fn as_ptr(&self) -> *mut u8 {
            self.mem as *mut u8
        }

        fn needs_init(&self) -> bool {
            self.needs_init
        }
    }

    struct CustomMemoryCreator {
        pub num_created_memories: Mutex<usize>,
        pub num_total_bytes: Arc<Mutex<usize>>,
        pub needs_init: bool,
        pub initial_contents: Vec<(usize, Vec<u8>)>,
    }

    impl CustomMemoryCreator {
        pub fn new() -> Self {
            Self {
                num_created_memories: Mutex::new(0),
                num_total_bytes: Arc::new(Mutex::new(0)),
                needs_init: true,
                initial_contents: Vec::new(),
            }
        }

        pub fn preinitialized(initial_contents: Vec<(usize, Vec<u8>)>) -> Self {
            Self {
                num_created_memories: Mutex::new(0),
                num_total_bytes: Arc::new(Mutex::new(0)),
                needs_init: false,
                initial_contents,
            }
        }
    }

    unsafe impl MemoryCreator for CustomMemoryCreator {
        fn new_memory(
            &self,
            ty: MemoryType,
            minimum: usize,
            maximum: Option<usize>,
            reserved_size: Option<usize>,
            guard_size: usize,
        ) -> Result<Box<dyn LinearMemory>, String> {
            assert_eq!(guard_size, 0);
            assert_eq!(reserved_size, Some(0));
            assert!(!ty.is_64());
            unsafe {
                // Cap the maximum at 10MiB to reduce the virtual memory
                // allocated by this test to execute on 32-bit platforms.
                let mem = Box::new(CustomMemory::new(
                    minimum,
                    maximum.unwrap_or(10 << 20),
                    self.num_total_bytes.clone(),
                    self.needs_init,
                    &self.initial_contents,
                ));
                *self.num_created_memories.lock().unwrap() += 1;
                Ok(mem)
            }
        }
    }

    fn store(mem_creator: Arc<CustomMemoryCreator>) -> Store<()> {
        let mut config = Config::new();
        config
            .with_host_memory(mem_creator.clone())
            .memory_init_cow(false)
            .memory_reservation(0)
            .memory_guard_size(0);
        Store::new(&Engine::new(&config).unwrap(), ())
    }

    fn config() -> (Store<()>, Arc<CustomMemoryCreator>) {
        let mem_creator = Arc::new(CustomMemoryCreator::new());
        (store(mem_creator.clone()), mem_creator)
    }

    #[test]
    fn host_memory() -> wasmtime::Result<()> {
        let (mut store, mem_creator) = config();
        let module = Module::new(
            store.engine(),
            r#"
            (module
                (memory (export "memory") 1)
            )
        "#,
        )?;
        Instance::new(&mut store, &module, &[])?;

        assert_eq!(*mem_creator.num_created_memories.lock().unwrap(), 1);

        Ok(())
    }

    #[test]
    fn host_memory_initializes_by_default() -> wasmtime::Result<()> {
        let (mut store, _) = config();
        let module = Module::new(
            store.engine(),
            r#"
            (module
                (memory (export "memory") 1)
                (data (i32.const 8) "module")
            )
        "#,
        )?;

        let instance = Instance::new(&mut store, &module, &[])?;
        let memory = instance.get_memory(&mut store, "memory").unwrap();

        let mut initialized = [0; 6];
        memory.read(&store, 8, &mut initialized).unwrap();
        assert_eq!(&initialized, b"module");

        let mut zero = [0xff; 1];
        memory.read(&store, 0, &mut zero).unwrap();
        assert_eq!(zero, [0]);

        Ok(())
    }

    #[test]
    fn host_memory_can_skip_initialization() -> wasmtime::Result<()> {
        let mem_creator = Arc::new(CustomMemoryCreator::preinitialized(vec![(
            8,
            b"stored".to_vec(),
        )]));
        let mut store = store(mem_creator);
        let module = Module::new(
            store.engine(),
            r#"
            (module
                (memory (export "memory") 1)
                (data (i32.const 8) "module")
            )
        "#,
        )?;

        let instance = Instance::new(&mut store, &module, &[])?;
        let memory = instance.get_memory(&mut store, "memory").unwrap();

        let mut preserved = [0; 6];
        memory.read(&store, 8, &mut preserved).unwrap();
        assert_eq!(&preserved, b"stored");

        Ok(())
    }

    #[test]
    fn host_memory_grow() -> wasmtime::Result<()> {
        let (mut store, mem_creator) = config();
        let module = Module::new(
            store.engine(),
            r#"
            (module
                (func $f (drop (memory.grow (i32.const 1))))
                (memory (export "memory") 1 2)
                (start $f)
            )
        "#,
        )?;

        Instance::new(&mut store, &module, &[])?;
        let instance2 = Instance::new(&mut store, &module, &[])?;

        assert_eq!(*mem_creator.num_created_memories.lock().unwrap(), 2);

        assert_eq!(
            instance2
                .get_memory(&mut store, "memory")
                .unwrap()
                .size(&store),
            2
        );

        // we take the lock outside the assert, so it won't get poisoned on assert failure
        let tot_pages = *mem_creator.num_total_bytes.lock().unwrap();
        assert_eq!(
            tot_pages,
            (4 * wasmtime_environ::Memory::DEFAULT_PAGE_SIZE) as usize
        );

        drop(store);
        let tot_pages = *mem_creator.num_total_bytes.lock().unwrap();
        assert_eq!(tot_pages, 0);

        Ok(())
    }
}
