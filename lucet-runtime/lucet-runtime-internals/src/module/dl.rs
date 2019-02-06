use crate::error::Error;
use crate::module::globals::{self, GlobalsSpec};
use crate::module::sparse_page_data::SparsePageData;
use crate::module::{
    AddrDetails, HeapSpec, Module, ModuleInternal, RuntimeSpec, TableElement, TrapManifestRecord,
};
use libc::c_void;
use libloading::{Library, Symbol};
use std::ffi::{CStr, OsStr};
use std::mem;
use std::slice::from_raw_parts;
use std::sync::Arc;

/// A Lucet module backed by a dynamically-loaded shared object.
pub struct DlModule {
    lib: Library,

    /// Base address of the dynamically-loaded module
    fbase: *const c_void,

    /// Spec for heap and globals required by the module
    runtime_spec: RuntimeSpec,

    trap_manifest: &'static [TrapManifestRecord],
}

// for the one raw pointer only
unsafe impl Send for DlModule {}
unsafe impl Sync for DlModule {}

impl DlModule {
    /// Create a module, loading code from a shared object on the filesystem.
    pub fn load<P: AsRef<OsStr>>(so_path: P) -> Result<Arc<Self>, Error> {
        // Load the dynamic library. The undefined symbols corresponding to the lucet_syscall_
        // functions will be provided by the current executable.  We trust our wasm->dylib compiler
        // to make sure these function calls are the way the dylib can touch memory outside of its
        // stack and heap.
        let lib = Library::new(so_path).map_err(Error::DlError)?;

        let heap_ptr = unsafe {
            lib.get::<*const HeapSpec>(b"lucet_heap_spec")
                .map_err(|e| {
                    lucet_incorrect_module!(
                        "error loading required symbol `lucet_heap_spec`: {}",
                        e
                    )
                })?
        };
        let heap: HeapSpec = unsafe {
            heap_ptr
                .as_ref()
                .ok_or(lucet_incorrect_module!(
                    "`lucet_heap_spec` is defined but null"
                ))?
                .clone()
        };

        let fbase = if let Some(dli) = dladdr(*heap_ptr as *const c_void) {
            dli.dli_fbase
        } else {
            std::ptr::null()
        };

        let globals = unsafe {
            globals::read_from_module({
                let spec = lib
                    .get::<*const GlobalsSpec>(b"lucet_globals_spec")
                    .map_err(|e| {
                        lucet_incorrect_module!(
                            "error loading required symbol `lucet_globals_spec`: {}",
                            e
                        )
                    })?;
                let spec_raw: *const GlobalsSpec = *spec;
                spec_raw
            })?
        };
        let runtime_spec = RuntimeSpec { heap, globals };

        let trap_manifest = unsafe {
            if let Ok(len_ptr) = lib.get::<*const u32>(b"lucet_trap_manifest_len") {
                let len = len_ptr.as_ref().ok_or(lucet_incorrect_module!(
                    "`lucet_trap_manifest_len` is defined but null"
                ))?;
                let records = lib
                    .get::<*const TrapManifestRecord>(b"lucet_trap_manifest")
                    .map_err(|e| {
                        lucet_incorrect_module!("error loading symbol `lucet_trap_manifest`: {}", e)
                    })?
                    .as_ref()
                    .ok_or(lucet_incorrect_module!(
                        "`lucet_trap_manifest` is defined but null"
                    ))?;
                from_raw_parts(records, *len as usize)
            } else {
                &[]
            }
        };

        Ok(Arc::new(DlModule {
            lib,
            fbase,
            runtime_spec,
            trap_manifest,
        }))
    }
}

impl Module for DlModule {}

impl ModuleInternal for DlModule {
    fn table_elements(&self) -> Result<&[TableElement], Error> {
        let p_table_segment: Symbol<*const TableElement> = unsafe {
            self.lib.get(b"guest_table_0").map_err(|e| {
                lucet_incorrect_module!("error loading required symbol `guest_table_0`: {}", e)
            })?
        };
        let p_table_segment_len: Symbol<*const usize> = unsafe {
            self.lib.get(b"guest_table_0_len").map_err(|e| {
                lucet_incorrect_module!("error loading required symbol `guest_table_0_len`: {}", e)
            })?
        };
        let len = unsafe { **p_table_segment_len };
        let elem_size = mem::size_of::<TableElement>();
        if len > std::u32::MAX as usize * elem_size {
            lucet_incorrect_module!("table segment too long: {}", len);
        }
        if len % elem_size != 0 {
            lucet_incorrect_module!(
                "table segment length {} not a multiple of table element size: {}",
                len,
                elem_size
            );
        }
        Ok(unsafe { from_raw_parts(*p_table_segment, **p_table_segment_len as usize / elem_size) })
    }

    fn sparse_page_data(&self) -> Result<&[*const c_void], Error> {
        unsafe {
            let spd = self
                .lib
                .get::<*const SparsePageData>(b"guest_sparse_page_data")
                .map_err(|e| {
                    lucet_incorrect_module!(
                        "error loading required symbol `guest_sparse_page_data`: {}",
                        e
                    )
                })?
                .as_ref()
                .ok_or(lucet_incorrect_module!(
                    "`guest_sparse_page_data` is defined but null"
                ))?;
            Ok(from_raw_parts(&spd.pages, spd.num_pages as usize))
        }
    }

    fn runtime_spec(&self) -> &RuntimeSpec {
        &self.runtime_spec
    }

    fn get_export_func(&self, sym: &[u8]) -> Result<*const extern "C" fn(), Error> {
        let mut guest_sym: Vec<u8> = b"guest_func_".to_vec();
        guest_sym.extend_from_slice(sym);
        match unsafe { self.lib.get::<*const extern "C" fn()>(&guest_sym) } {
            Err(ref e) if is_undefined_symbol(e) => Err(Error::SymbolNotFound(
                String::from_utf8_lossy(sym).into_owned(),
            )),
            Err(e) => Err(Error::DlError(e)),
            Ok(f) => Ok(*f),
        }
    }

    fn get_func_from_idx(
        &self,
        table_id: u32,
        func_id: u32,
    ) -> Result<*const extern "C" fn(), Error> {
        if table_id != 0 {
            return Err(Error::FuncNotFound(table_id, func_id));
        }
        let table = self.table_elements()?;
        let func: extern "C" fn() = table
            .get(func_id as usize)
            .map(|element| unsafe { std::mem::transmute(element.rf) })
            .ok_or(Error::FuncNotFound(table_id, func_id))?;
        Ok(&func as *const extern "C" fn())
    }

    fn get_start_func(&self) -> Result<Option<*const extern "C" fn()>, Error> {
        // `guest_start` is a pointer to the function the module designates as the start function,
        // since we can't have multiple symbols pointing to the same function and guest code might
        // call it in the normal course of execution
        if let Ok(start_func) = unsafe {
            self.lib
                .get::<*const *const extern "C" fn()>(b"guest_start")
        } {
            if start_func.is_null() {
                lucet_incorrect_module!("`guest_start` is defined but null");
            }
            Ok(Some(unsafe { **start_func }))
        } else {
            Ok(None)
        }
    }

    fn trap_manifest(&self) -> &[TrapManifestRecord] {
        self.trap_manifest
    }

    fn addr_details(&self, addr: *const c_void) -> Result<Option<AddrDetails>, Error> {
        if let Some(dli) = dladdr(addr) {
            let file_name = if dli.dli_fname.is_null() {
                None
            } else {
                Some(unsafe { CStr::from_ptr(dli.dli_fname).to_owned().into_string()? })
            };
            let sym_name = if dli.dli_sname.is_null() {
                None
            } else {
                Some(unsafe { CStr::from_ptr(dli.dli_sname).to_owned().into_string()? })
            };
            Ok(Some(AddrDetails {
                in_module_code: dli.dli_fbase as *const c_void == self.fbase,
                file_name,
                sym_name,
            }))
        } else {
            Ok(None)
        }
    }
}

fn is_undefined_symbol(e: &std::io::Error) -> bool {
    // gross, but I'm not sure how else to differentiate this type of error from other
    // IO errors
    format!("{}", e).contains("undefined symbol")
}

// TODO: PR to nix or libloading?
// TODO: possibly not safe to use without grabbing the mutex within libloading::Library?
fn dladdr(addr: *const c_void) -> Option<libc::Dl_info> {
    let mut info = unsafe { mem::uninitialized::<libc::Dl_info>() };
    let res = unsafe { libc::dladdr(addr, &mut info as *mut libc::Dl_info) };
    if res != 0 {
        Some(info)
    } else {
        None
    }
}
