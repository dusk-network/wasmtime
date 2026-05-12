#![cfg(not(miri))]

use dusk_wasmtime::{
    Config, Engine, Instance, LinearMemory, MemoryCreator, MemoryType, Module, Store,
};
use std::sync::Arc;

struct VecMemory {
    storage: Vec<u8>,
    byte_size: usize,
    needs_init: bool,
}

unsafe impl LinearMemory for VecMemory {
    fn byte_size(&self) -> usize {
        self.byte_size
    }

    fn byte_capacity(&self) -> usize {
        self.storage.len()
    }

    fn grow_to(&mut self, new_size: usize) -> dusk_wasmtime::Result<()> {
        if new_size > self.storage.len() {
            dusk_wasmtime::bail!("memory maximum size exceeded");
        }
        self.byte_size = new_size;
        Ok(())
    }

    fn as_ptr(&self) -> *mut u8 {
        self.storage.as_ptr().cast_mut()
    }

    fn needs_init(&self) -> bool {
        self.needs_init
    }
}

struct VecMemoryCreator {
    needs_init: bool,
    initial_contents: Vec<(usize, Vec<u8>)>,
}

unsafe impl MemoryCreator for VecMemoryCreator {
    fn new_memory(
        &self,
        _ty: MemoryType,
        minimum: usize,
        maximum: Option<usize>,
        _reserved_size_in_bytes: Option<usize>,
        guard_size_in_bytes: usize,
    ) -> Result<Box<dyn LinearMemory>, String> {
        assert_eq!(guard_size_in_bytes, 0);

        let capacity = maximum.unwrap_or(10 << 20).max(minimum);
        let mut storage = vec![0; capacity];
        for (offset, bytes) in &self.initial_contents {
            storage[*offset..][..bytes.len()].copy_from_slice(bytes);
        }

        Ok(Box::new(VecMemory {
            storage,
            byte_size: minimum,
            needs_init: self.needs_init,
        }))
    }
}

fn store(needs_init: bool, initial_contents: Vec<(usize, Vec<u8>)>) -> Store<()> {
    let mut config = Config::new();
    config
        .with_host_memory(Arc::new(VecMemoryCreator {
            needs_init,
            initial_contents,
        }))
        .memory_guard_size(0)
        .memory_init_cow(false)
        .memory_may_move(false)
        .memory_reservation(0)
        .memory_reservation_for_growth(0);

    Store::new(&Engine::new(&config).unwrap(), ())
}

#[test]
fn custom_memory_initializes_by_default() -> dusk_wasmtime::Result<()> {
    let mut store = store(true, Vec::new());
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
    memory.read(&store, 8, &mut initialized)?;
    assert_eq!(&initialized, b"module");

    let mut zero = [0xff; 1];
    memory.read(&store, 0, &mut zero)?;
    assert_eq!(zero, [0]);

    Ok(())
}

#[test]
fn custom_memory_can_skip_initialization() -> dusk_wasmtime::Result<()> {
    let mut store = store(false, vec![(8, b"stored".to_vec())]);
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
    memory.read(&store, 8, &mut preserved)?;
    assert_eq!(&preserved, b"stored");

    Ok(())
}
