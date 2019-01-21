mod bases;
mod cache;
mod global;

pub use self::global::GlobalValue;

use self::bases::GlobalBases;
use self::cache::{Cache, FunctionCacheIndex};
use crate::compiler::Compiler;
use crate::program::{CtonSignature, Function, FunctionSig, Program, TableDef};
use cranelift_codegen::ir;
use cranelift_codegen::ir::types::{I32, I64};
use cranelift_module::Linkage;
use failure::{format_err, Error, ResultExt};
use std::fmt;

pub type GlobalIndex = u32;
pub type MemoryIndex = u32;
pub type SignatureIndex = u32;
pub type FunctionIndex = u32;
pub type TableIndex = u32;

fn global_var_offset(index: isize) -> isize {
    index * POINTER_SIZE as isize
}

// For readability
pub const POINTER_SIZE: usize = 8;
pub const NATIVE_POINTER: ir::Type = I64;

pub struct EntityCreator<'p> {
    program: &'p Program,
    bases: GlobalBases,
    cache: Cache<'p>,
}

impl<'p> EntityCreator<'p> {
    pub fn new(program: &'p Program) -> Self {
        Self {
            program: program,
            bases: GlobalBases::new(),
            cache: Cache::new(),
        }
    }

    pub fn get_global(
        &mut self,
        func: &mut ir::Function,
        index: GlobalIndex,
        compiler: &Compiler,
    ) -> Result<GlobalValue, Error> {
        let global = self
            .program
            .globals()
            .get(index as usize)
            .ok_or_else(|| format_err!("global out of range: {}", index))?;
        let base = self.bases.globals(func, compiler);
        self.cache.global(index, || {
            let offset = global_var_offset(index as isize);
            let gv = func.create_global_value(ir::GlobalValueData::IAddImm {
                base: base,
                offset: (offset as i64).into(),
                global_type: NATIVE_POINTER,
            });
            Ok(GlobalValue {
                var: gv,
                ty: global.cton_type(),
            })
        })
    }

    pub fn get_heap(
        &mut self,
        func: &mut ir::Function,
        index: MemoryIndex,
        compiler: &Compiler,
    ) -> Result<ir::Heap, Error> {
        let base = self.bases.heap(func, compiler);
        let heap_spec = self.program.heap_spec();

        self.cache.heap(index, || {
            if index != 0 {
                return Err(format_err!(
                    "can only create heap for memory index 0; got {}",
                    index
                ));
            }
            Ok(func.create_heap(ir::HeapData {
                base,
                min_size: heap_spec.initial_size.into(),
                offset_guard_size: heap_spec.guard_size.into(),
                style: ir::HeapStyle::Static {
                    bound: heap_spec.reserved_size.into(),
                },
                index_type: I32,
            }))
        })
    }

    pub fn get_table(
        &mut self,
        ix: TableIndex,
        func: &mut ir::Function,
        compiler: &Compiler,
    ) -> Result<(TableDef, ir::GlobalValue), Error> {
        let tbl = self.program.get_table(ix)?;
        let base = self.bases.table(func, compiler);
        Ok((tbl.clone(), base))
    }

    pub fn get_indirect_sig(
        &mut self,
        func: &mut ir::Function,
        index: SignatureIndex,
    ) -> Result<&(ir::SigRef, FunctionSig), Error> {
        let sig = self.program.get_signature(index)?;
        self.cache.signature(index, || {
            let sigref = func.import_signature(sig.cton_signature());
            Ok((sigref, sig))
        })
    }

    pub fn get_direct_func(
        &mut self,
        func: &mut ir::Function,
        index: FunctionIndex,
        compiler: &Compiler,
    ) -> Result<&(ir::FuncRef, &'p Function), Error> {
        let f = self.program.get_function(index)?;
        self.cache
            .function(FunctionCacheIndex::Wasm(index), move || {
                let import = func.import_signature(f.signature());
                let fref = func.import_function(ir::ExtFuncData {
                    name: compiler.get_function(f).context("direct call")?.into(),
                    signature: import,
                    colocated: match f.linkage() {
                        Linkage::Import => false,
                        _ => true,
                    },
                });
                Ok((fref, f))
            })
    }

    pub fn get_runtime_func(
        &mut self,
        func: &mut ir::Function,
        name: String,
        compiler: &Compiler,
    ) -> Result<&(ir::FuncRef, &'p Function), Error> {
        let f = self.program.get_runtime_function(&name)?;
        self.cache
            .function(FunctionCacheIndex::Runtime(name), move || {
                let import = func.import_signature(f.signature());
                let fref = func.import_function(ir::ExtFuncData {
                    name: compiler.get_function(f).context("runtime call")?.into(),
                    signature: import,
                    colocated: false,
                });
                Ok((fref, f))
            })
    }
}

impl<'p> fmt::Debug for EntityCreator<'p> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "EntityCreator")
    }
}
