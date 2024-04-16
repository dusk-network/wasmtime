use crate::cranelift::builder::builder as cranelift_builder;
use crate::winch::builder::builder as winch_builder;

use crate::cranelift::builder::Builder as CraneliftBuilder;
use crate::winch::builder::Builder as WinchBuilder;

use crate::compiler::Compiler as CraneliftOrWinchCompiler;

use std::path;
use std::sync::Arc;

use anyhow::Result;
use target_lexicon::Triple;
use wasmtime_environ::{CacheStore, Compiler, CompilerBuilder, Setting, Tunables};

#[derive(Debug)]
struct Builder {
    cranelift: CraneliftBuilder,
    winch: WinchBuilder,
}

pub fn builder(triple: Option<Triple>) -> Result<Box<dyn CompilerBuilder>> {
    Ok(Box::new(Builder {
        cranelift: cranelift_builder(triple.clone())?,
        winch: winch_builder(triple)?,
    }))
}

impl CompilerBuilder for Builder {
    fn target(&mut self, target: target_lexicon::Triple) -> Result<()> {
        self.cranelift.target(target.clone())?;
        self.winch.target(target)?;
        Ok(())
    }

    fn clif_dir(&mut self, path: &path::Path) -> Result<()> {
        self.cranelift.clif_dir(path)?;
        self.winch.clif_dir(path)?;
        Ok(())
    }

    fn triple(&self) -> &target_lexicon::Triple {
        self.cranelift.triple()
    }

    fn set(&mut self, name: &str, val: &str) -> Result<()> {
        self.cranelift.set(name, val)?;
        self.winch.set(name, val)?;
        Ok(())
    }

    fn enable(&mut self, name: &str) -> Result<()> {
        self.cranelift.enable(name)?;
        self.winch.enable(name)?;
        Ok(())
    }

    fn settings(&self) -> Vec<Setting> {
        let cranelift_settings = self.cranelift.settings();
        let winch_settings = self.winch.settings();

        // Return the intersection of the two settings.
        let mut settings = cranelift_settings;
        settings.retain(|setting| {
            let mut retain = false;
            for winch_setting in &winch_settings {
                if setting.name == winch_setting.name {
                    retain = true;
                    break;
                }
            }
            retain
        });
        settings
    }

    fn enable_incremental_compilation(&mut self, cache_store: Arc<dyn CacheStore>) -> Result<()> {
        self.cranelift
            .enable_incremental_compilation(cache_store.clone())?;
        self.winch.enable_incremental_compilation(cache_store)?;
        Ok(())
    }

    fn set_tunables(&mut self, tunables: Tunables) -> Result<()> {
        self.cranelift.set_tunables(tunables.clone())?;
        self.winch.set_tunables(tunables)?;
        Ok(())
    }

    fn build(&self) -> Result<Box<dyn Compiler>> {
        let cranelift = self.cranelift.build()?;
        let winch = self.winch.build()?;
        Ok(Box::new(CraneliftOrWinchCompiler { cranelift, winch }))
    }

    fn wmemcheck(&mut self, enable: bool) {
        self.cranelift.wmemcheck(enable);
        self.winch.wmemcheck(enable);
    }
}
