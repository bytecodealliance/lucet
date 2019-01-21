use crate::compiler::entity::{
    FunctionIndex, GlobalIndex, GlobalValue, MemoryIndex, SignatureIndex,
};
use crate::program::{Function, FunctionSig};
use cranelift_codegen::ir;
use failure::Error;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum FunctionCacheIndex {
    Wasm(FunctionIndex),
    Runtime(String),
}

pub struct Cache<'p> {
    /// Collection of global variables that have been brought into scope
    globals: HashMap<GlobalIndex, GlobalValue>,
    /// Collection of heaps that have been brought into scope
    heaps: HashMap<MemoryIndex, ir::Heap>,
    /// Collection of indirect call signatures that have been brought into scope,
    /// and the signatures themselves
    signatures: HashMap<SignatureIndex, (ir::SigRef, FunctionSig)>,
    /// Collection of functions that have been brought into scope, and the functions
    /// themselves
    functions: HashMap<FunctionCacheIndex, (ir::FuncRef, &'p Function)>,
}

impl<'p> Cache<'p> {
    pub fn new() -> Self {
        Self {
            globals: HashMap::new(),
            heaps: HashMap::new(),
            signatures: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    pub fn global<F>(&mut self, index: GlobalIndex, makeglob: F) -> Result<GlobalValue, Error>
    where
        F: FnOnce() -> Result<GlobalValue, Error>,
    {
        let r = entry_or_insert_result(&mut self.globals, index, makeglob);
        Ok(*r?)
    }

    pub fn heap<F>(&mut self, index: MemoryIndex, makeheap: F) -> Result<ir::Heap, Error>
    where
        F: FnOnce() -> Result<ir::Heap, Error>,
    {
        let r = entry_or_insert_result(&mut self.heaps, index, makeheap);
        Ok(*r?)
    }

    pub fn signature<F>(
        &mut self,
        index: SignatureIndex,
        makesig: F,
    ) -> Result<&(ir::SigRef, FunctionSig), Error>
    where
        F: FnOnce() -> Result<(ir::SigRef, FunctionSig), Error>,
    {
        entry_or_insert_result(&mut self.signatures, index, makesig)
    }

    pub fn function<F>(
        &mut self,
        index: FunctionCacheIndex,
        makefunc: F,
    ) -> Result<&(ir::FuncRef, &'p Function), Error>
    where
        F: FnOnce() -> Result<(ir::FuncRef, &'p Function), Error>,
    {
        entry_or_insert_result(&mut self.functions, index, makefunc)
    }
}

use std::collections::hash_map::Entry;
use std::hash::Hash;

fn entry_or_insert_result<K, V, E, F>(map: &mut HashMap<K, V>, key: K, mkval: F) -> Result<&V, E>
where
    K: Eq + Hash,
    F: FnOnce() -> Result<V, E>,
{
    let entry = map.entry(key);
    Ok(match entry {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => entry.insert(mkval()?),
    })
}
