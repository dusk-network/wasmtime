use crate::cranelift::compiler::Compiler as CraneliftCompiler;
use crate::winch::compiler::Compiler as WinchCompiler;

use std::any::Any;

use anyhow::Result;
use cranelift_entity::PrimaryMap;
use object::write::{Object, SymbolId};
use wasmtime_environ::{
    CompileError, DefinedFuncIndex, FlagValue, FuncIndex, FunctionBodyData, FunctionLoc,
    ModuleTranslation, ModuleTypesBuilder, ObjectKind, WasmFuncType, WasmFunctionInfo,
};

pub struct Compiler {
    pub(crate) cranelift: CraneliftCompiler,
    pub(crate) winch: WinchCompiler,
}

impl 

impl wasmtime_environ::Compiler for Compiler {
    /// Compiles the function `index` within `translation`.
    ///
    /// The body of the function is available in `data` and configuration
    /// values are also passed in via `tunables`. Type information in
    /// `translation` is all relative to `types`.
    fn compile_function(
        &self,
        translation: &ModuleTranslation<'_>,
        index: DefinedFuncIndex,
        data: FunctionBodyData<'_>,
        types: &ModuleTypesBuilder,
    ) -> Result<(WasmFunctionInfo, Box<dyn Any + Send>), CompileError> {
        // If the WASM is 64-bit we use Cranelift, otherwise we use Winch.
        for (_, plan) in &translation.module.memory_plans {
            plan.memory.memory64
        }
        todo!()
    }

    /// Compile a trampoline for an array-call host function caller calling the
    /// `index`th Wasm function.
    ///
    /// The trampoline should save the necessary state to record the
    /// host-to-Wasm transition (e.g. registers used for fast stack walking).
    fn compile_array_to_wasm_trampoline(
        &self,
        translation: &ModuleTranslation<'_>,
        types: &ModuleTypesBuilder,
        index: DefinedFuncIndex,
    ) -> Result<Box<dyn Any + Send>, CompileError> {
        todo!()
    }

    /// Compile a trampoline for a native-call host function caller calling the
    /// `index`th Wasm function.
    ///
    /// The trampoline should save the necessary state to record the
    /// host-to-Wasm transition (e.g. registers used for fast stack walking).
    fn compile_native_to_wasm_trampoline(
        &self,
        translation: &ModuleTranslation<'_>,
        types: &ModuleTypesBuilder,
        index: DefinedFuncIndex,
    ) -> Result<Box<dyn Any + Send>, CompileError> {
        todo!()
    }

    /// Compile a trampoline for a Wasm caller calling a native callee with the
    /// given signature.
    ///
    /// The trampoline should save the necessary state to record the
    /// Wasm-to-host transition (e.g. registers used for fast stack walking).
    fn compile_wasm_to_native_trampoline(
        &self,
        wasm_func_ty: &WasmFuncType,
    ) -> Result<Box<dyn Any + Send>, CompileError> {
        todo!()
    }

    /// Appends a list of compiled functions to an in-memory object.
    ///
    /// This function will receive the same `Box<dyn Any>` produced as part of
    /// compilation from functions like `compile_function`,
    /// `compile_host_to_wasm_trampoline`, and other component-related shims.
    /// Internally this will take all of these functions and add information to
    /// the object such as:
    ///
    /// * Compiled code in a `.text` section
    /// * Unwind information in Wasmtime-specific sections
    /// * Relocations, if necessary, for the text section
    ///
    /// Each function is accompanied with its desired symbol name and the return
    /// value of this function is the symbol for each function as well as where
    /// each function was placed within the object.
    ///
    /// The `resolve_reloc` argument is intended to resolving relocations
    /// between function, chiefly resolving intra-module calls within one core
    /// wasm module. The closure here takes two arguments:
    ///
    /// 1. First, the index within `funcs` that is being resolved,
    ///
    /// 2. and next the `FuncIndex` which is the relocation target to
    /// resolve.
    ///
    /// The return value is an index within `funcs` that the relocation points
    /// to.
    fn append_code(
        &self,
        obj: &mut Object<'static>,
        funcs: &[(String, Box<dyn Any + Send>)],
        resolve_reloc: &dyn Fn(usize, FuncIndex) -> usize,
    ) -> Result<Vec<(SymbolId, FunctionLoc)>> {
        todo!()
    }

    /// Inserts two trampolines into `obj` for a array-call host function:
    ///
    /// 1. A wasm-call trampoline: A trampoline that takes arguments in their
    ///    wasm-call locations, moves them to their array-call locations, calls
    ///    the array-call host function, and finally moves the return values
    ///    from the array-call locations to the wasm-call return
    ///    locations. Additionally, this trampoline manages the wasm-to-host
    ///    state transition for the runtime.
    ///
    /// 2. A native-call trampoline: A trampoline that takes arguments in their
    ///    native-call locations, moves them to their array-call locations,
    ///    calls the array-call host function, and finally moves the return
    ///    values from the array-call locations to the native-call return
    ///    locations. Does not need to manage any wasm/host state transitions,
    ///    since both caller and callee are on the host side.
    ///
    /// This will configure the same sections as `append_code`, but will likely
    /// be much smaller.
    ///
    /// The two returned `FunctionLoc` structures describe where to find these
    /// trampolines in the text section, respectively.
    ///
    /// These trampolines are only valid for in-process JIT usage. They bake in
    /// the function pointer to the host code.
    fn emit_trampolines_for_array_call_host_func(
        &self,
        ty: &WasmFuncType,
        // Actually `host_fn: VMArrayCallFunction` but that type is not
        // available in `wasmtime-environ`.
        host_fn: usize,
        obj: &mut Object<'static>,
    ) -> Result<(FunctionLoc, FunctionLoc)> {
        todo!()
    }

    /// Creates a new `Object` file which is used to build the results of a
    /// compilation into.
    ///
    /// The returned object file will have an appropriate
    /// architecture/endianness for `self.triple()`, but at this time it is
    /// always an ELF file, regardless of target platform.
    fn object(&self, kind: ObjectKind) -> Result<Object<'static>> {
        todo!()
    }

    /// Returns the target triple that this compiler is compiling for.
    fn triple(&self) -> &target_lexicon::Triple {
        todo!()
    }

    /// Returns the alignment necessary to align values to the page size of the
    /// compilation target. Note that this may be an upper-bound where the
    /// alignment is larger than necessary for some platforms since it may
    /// depend on the platform's runtime configuration.
    fn page_size_align(&self) -> u64 {
        use target_lexicon::*;
        match (self.triple().operating_system, self.triple().architecture) {
            (
                OperatingSystem::MacOSX { .. }
                | OperatingSystem::Darwin
                | OperatingSystem::Ios
                | OperatingSystem::Tvos,
                Architecture::Aarch64(..),
            ) => 0x4000,
            // 64 KB is the maximal page size (i.e. memory translation granule size)
            // supported by the architecture and is used on some platforms.
            (_, Architecture::Aarch64(..)) => 0x10000,
            _ => 0x1000,
        }
    }

    /// Returns a list of configured settings for this compiler.
    fn flags(&self) -> Vec<(&'static str, FlagValue<'static>)> {
        todo!()
    }

    /// Same as [`Compiler::flags`], but ISA-specific (a cranelift-ism)
    fn isa_flags(&self) -> Vec<(&'static str, FlagValue<'static>)> {
        todo!()
    }

    /// Get a flag indicating whether branch protection is enabled.
    fn is_branch_protection_enabled(&self) -> bool {
        todo!()
    }

    /// Returns a suitable compiler usable for component-related compliations.
    ///
    /// Note that the `ComponentCompiler` trait can also be implemented for
    /// `Self` in which case this function would simply return `self`.
    #[cfg(feature = "component-model")]
    fn component_compiler(&self) -> &dyn wasmtime_environ::component::ComponentCompiler {
        todo!()
    }

    /// Appends generated DWARF sections to the `obj` specified for the compiled
    /// functions.
    fn append_dwarf(
        &self,
        obj: &mut Object<'_>,
        translation: &ModuleTranslation<'_>,
        funcs: &PrimaryMap<DefinedFuncIndex, (SymbolId, &(dyn Any + Send))>,
    ) -> Result<()> {
        todo!()
    }

    /// Creates a new System V Common Information Entry for the ISA.
    ///
    /// Returns `None` if the ISA does not support System V unwind information.
    fn create_systemv_cie(&self) -> Option<gimli::write::CommonInformationEntry> {
        // By default, an ISA cannot create a System V CIE.
        None
    }
}
