use crate::error::Error;
use crate::module::{AddrDetails, GlobalSpec, HeapSpec, Module, ModuleInternal, TableElement};
use libc::c_void;
use libloading::{Library, Symbol};
use lucet_module_data::{
    FunctionHandle, FunctionIndex, FunctionPointer, FunctionSpec, ModuleData, Signature,
};
use std::ffi::CStr;
use std::mem;
use std::path::Path;
use std::slice;
use std::slice::from_raw_parts;
use std::sync::Arc;

/// A Lucet module backed by a dynamically-loaded shared object.
pub struct DlModule {
    lib: Library,

    /// Base address of the dynamically-loaded module
    fbase: *const c_void,

    /// Metadata decoded from inside the module
    module_data: ModuleData<'static>,

    function_manifest: &'static [FunctionSpec],
}

// for the one raw pointer only
unsafe impl Send for DlModule {}
unsafe impl Sync for DlModule {}

impl DlModule {
    /// Create a module, loading code from a shared object on the filesystem.
    pub fn load<P: AsRef<Path>>(so_path: P) -> Result<Arc<Self>, Error> {
        // Load the dynamic library. The undefined symbols corresponding to the lucet_syscall_
        // functions will be provided by the current executable.  We trust our wasm->dylib compiler
        // to make sure these function calls are the way the dylib can touch memory outside of its
        // stack and heap.
        let abs_so_path = so_path.as_ref().canonicalize().map_err(Error::DlError)?;
        let lib = Library::new(abs_so_path.as_os_str()).map_err(Error::DlError)?;

        let module_data_ptr = unsafe {
            lib.get::<*const u8>(b"lucet_module_data").map_err(|e| {
                lucet_incorrect_module!("error loading required symbol `lucet_module_data`: {}", e)
            })?
        };

        let module_data_len = unsafe {
            lib.get::<usize>(b"lucet_module_data_len").map_err(|e| {
                lucet_incorrect_module!(
                    "error loading required symbol `lucet_module_data_len`: {}",
                    e
                )
            })?
        };

        // Deserialize the slice into ModuleData, which will hold refs into the loaded
        // shared object file in `module_data_slice`. Both of these get a 'static lifetime because
        // Rust doesn't have a safe way to describe that their lifetime matches the containing
        // struct (and the dll).
        //
        // The exposed lifetime of ModuleData will be the same as the lifetime of the
        // dynamically loaded library. This makes the interface safe.
        let module_data_slice: &'static [u8] =
            unsafe { slice::from_raw_parts(*module_data_ptr, *module_data_len) };
        let module_data = ModuleData::deserialize(module_data_slice)?;

        let fbase = if let Some(dli) = dladdr(*module_data_ptr as *const c_void) {
            dli.dli_fbase
        } else {
            std::ptr::null()
        };

        let function_manifest = unsafe {
            let manifest_len_ptr = lib.get::<*const u32>(b"lucet_function_manifest_len");
            let manifest_ptr = lib.get::<*const FunctionSpec>(b"lucet_function_manifest");

            match (manifest_ptr, manifest_len_ptr) {
                (Ok(ptr), Ok(len_ptr)) => {
                    let manifest_len = len_ptr.as_ref().ok_or(lucet_incorrect_module!(
                        "`lucet_function_manifest_len` is defined but null"
                    ))?;
                    let manifest = ptr.as_ref().ok_or(lucet_incorrect_module!(
                        "`lucet_function_manifest` is defined but null"
                    ))?;

                    from_raw_parts(manifest, *manifest_len as usize)
                }
                (Err(ptr_err), Err(len_err)) => {
                    if is_undefined_symbol(&ptr_err) && is_undefined_symbol(&len_err) {
                        &[]
                    } else {
                        // This is an unfortunate situation. Both attempts to look up symbols
                        // failed, but at least one is not due to an undefined symbol.
                        if !is_undefined_symbol(&ptr_err) {
                            // This returns `ptr_err` (rather than `len_err` or some mix) because
                            // of the following hunch: if both failed, and neither are undefined
                            // symbols, they are probably the same error.
                            return Err(Error::DlError(ptr_err));
                        } else {
                            return Err(Error::DlError(len_err));
                        }
                    }
                }
                (Ok(_), Err(e)) => {
                    return Err(lucet_incorrect_module!(
                        "error loading symbol `lucet_function_manifest_len`: {}",
                        e
                    ));
                }
                (Err(e), Ok(_)) => {
                    return Err(lucet_incorrect_module!(
                        "error loading symbol `lucet_function_manifest`: {}",
                        e
                    ));
                }
            }
        };

        Ok(Arc::new(DlModule {
            lib,
            fbase,
            module_data,
            function_manifest,
        }))
    }
}

impl Module for DlModule {}

impl ModuleInternal for DlModule {
    fn heap_spec(&self) -> Option<&HeapSpec> {
        self.module_data.heap_spec()
    }

    fn globals(&self) -> &[GlobalSpec] {
        self.module_data.globals_spec()
    }

    fn get_sparse_page_data(&self, page: usize) -> Option<&[u8]> {
        if let Some(ref sparse_data) = self.module_data.sparse_data() {
            *sparse_data.get_page(page)
        } else {
            None
        }
    }

    fn sparse_page_data_len(&self) -> usize {
        self.module_data.sparse_data().map(|d| d.len()).unwrap_or(0)
    }

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
        if len > std::u32::MAX as usize {
            lucet_incorrect_module!("table segment too long: {}", len);
        }
        Ok(unsafe { from_raw_parts(*p_table_segment, **p_table_segment_len as usize) })
    }

    fn get_export_func(&self, sym: &str) -> Result<FunctionHandle, Error> {
        self.module_data
            .get_export_func_id(sym)
            .ok_or_else(|| Error::SymbolNotFound(sym.to_string()))
            .map(|id| {
                let ptr = self.function_manifest()[id.as_u32() as usize].ptr();
                FunctionHandle { ptr, id }
            })
    }

    fn get_func_from_idx(&self, table_id: u32, func_id: u32) -> Result<FunctionHandle, Error> {
        if table_id != 0 {
            return Err(Error::FuncNotFound(table_id, func_id));
        }
        let table = self.table_elements()?;
        let func: FunctionPointer = table
            .get(func_id as usize)
            .map(|element| FunctionPointer::from_usize(element.rf as usize))
            .ok_or(Error::FuncNotFound(table_id, func_id))?;

        Ok(self.function_handle_from_ptr(func))
    }

    fn get_start_func(&self) -> Result<Option<FunctionHandle>, Error> {
        // `guest_start` is a pointer to the function the module designates as the start function,
        // since we can't have multiple symbols pointing to the same function and guest code might
        // call it in the normal course of execution
        if let Ok(start_func) = unsafe { self.lib.get::<*const extern "C" fn()>(b"guest_start") } {
            if start_func.is_null() {
                lucet_incorrect_module!("`guest_start` is defined but null");
            }
            Ok(Some(self.function_handle_from_ptr(
                FunctionPointer::from_usize(unsafe { **start_func } as usize),
            )))
        } else {
            Ok(None)
        }
    }

    fn function_manifest(&self) -> &[FunctionSpec] {
        self.function_manifest
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

    fn get_signature(&self, fn_id: FunctionIndex) -> &Signature {
        self.module_data.get_signature(fn_id)
    }
}

fn is_undefined_symbol(e: &std::io::Error) -> bool {
    // gross, but I'm not sure how else to differentiate this type of error from other
    // IO errors
    let msg = format!("{}", e);
    msg.contains("undefined symbol") || msg.contains("symbol not found")
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
