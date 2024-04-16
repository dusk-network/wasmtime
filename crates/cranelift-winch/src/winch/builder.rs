use crate::winch::compiler::Compiler;
use anyhow::{bail, Result};
use std::path;
use std::sync::Arc;
use target_lexicon::Triple;
use wasmtime_cranelift_shared::isa_builder::IsaBuilder;
use wasmtime_environ::Setting;
use winch_codegen::{isa, TargetIsa};

/// Compiler builder.
pub struct Builder {
    inner: IsaBuilder<Result<Box<dyn TargetIsa>>>,
}

pub fn builder(triple: Option<Triple>) -> Result<Builder> {
    Ok(Builder {
        inner: IsaBuilder::new(triple, |triple| isa::lookup(triple).map_err(|e| e.into()))?,
    })
}

impl Builder {
    pub fn triple(&self) -> &target_lexicon::Triple {
        self.inner.triple()
    }

    pub fn target(&mut self, target: target_lexicon::Triple) -> Result<()> {
        self.inner.target(target)?;
        Ok(())
    }

    pub fn clif_dir(&mut self, _path: &path::Path) -> Result<()> {
        anyhow::bail!("clif output not supported");
    }

    pub fn set(&mut self, name: &str, value: &str) -> Result<()> {
        self.inner.set(name, value)
    }

    pub fn enable(&mut self, name: &str) -> Result<()> {
        self.inner.enable(name)
    }

    pub fn settings(&self) -> Vec<Setting> {
        self.inner.settings()
    }

    pub fn set_tunables(&mut self, tunables: wasmtime_environ::Tunables) -> Result<()> {
        let _ = tunables;
        Ok(())
    }

    pub fn build(&self) -> Result<crate::winch::compiler::Compiler> {
        let isa = self.inner.build()?;
        Ok(Compiler::new(isa))
    }

    pub fn enable_incremental_compilation(
        &mut self,
        _cache_store: Arc<dyn wasmtime_environ::CacheStore>,
    ) -> Result<()> {
        bail!("incremental compilation is not supported on this platform");
    }

    pub fn wmemcheck(&mut self, _enable: bool) {}
}

impl std::fmt::Debug for Builder {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Builder: {{ triple: {:?} }}", self.triple())
    }
}
