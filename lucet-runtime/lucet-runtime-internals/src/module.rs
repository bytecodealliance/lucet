mod globals;
mod sparse_page_data;

use crate::module::globals::GlobalsSpec;
use crate::probestack::{lucet_probestack, lucet_probestack_size};
use crate::trapcode::{TrapCode, TrapCodeType};
use failure::{bail, format_err, Error};
use libc::{c_void, uint64_t};
use libloading::{Library, Symbol};
use std::collections::HashMap;
use std::ffi::{CStr, OsStr};
use std::mem;
use std::slice::from_raw_parts;

pub struct DlModule {
    lib: Library,

    /// Base address of the dynamically-loaded module
    fbase: *const c_void,

    /// Spec for heap and globals required by the module
    runtime_spec: RuntimeSpec,

    trap_manifest: &'static [TrapManifestRecord],
}

#[repr(C)]
pub struct TrapManifestRecord {
    func_addr: u64,
    func_len: u64,
    table_addr: u64,
    table_len: u64,
}

impl TrapManifestRecord {
    pub fn contains_addr(&self, addr: *const c_void) -> bool {
        let addr = addr as u64;
        // TODO: is this correct? off-by-one error?
        addr >= self.func_addr && addr <= self.func_addr + self.func_len
    }

    pub fn trapsites(&self) -> &[TrapSite] {
        let table_addr = self.table_addr as *const TrapSite;
        assert!(!table_addr.is_null());
        unsafe { from_raw_parts(table_addr, self.table_len as usize) }
    }

    pub fn lookup_addr(&self, addr: *const c_void) -> Option<TrapCode> {
        if !self.contains_addr(addr) {
            return None;
        }

        // predicate to find the trapsite for the addr via binary search
        let f =
            |ts: &TrapSite| (self.func_addr as usize + ts.offset as usize).cmp(&(addr as usize));

        let trapsites = self.trapsites();
        if let Ok(i) = trapsites.binary_search_by(f) {
            let trapcode =
                TrapCode::try_from_u32(trapsites[i].trapcode).expect("valid trapcode value");
            Some(trapcode)
        } else {
            None
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TrapSite {
    offset: u32,
    trapcode: u32,
}

#[repr(C)]
pub struct SparsePageData {
    num_pages: u64,
    pages: *const c_void,
}

#[repr(C)]
pub struct TableElement {
    ty: u64,
    rf: u64,
}

/// Specifications from the WebAssembly module about its heap.
///
/// The `reserved_size` and `guard_size`, when added together, must not exceed the
/// `heap_address_space_size` given in the corresponding `Limits`. The `initial_size` and `max_size`
/// (if given) must fit into the `heap_memory_size` given in the corresponding `Limits`.
///
/// This is serialized into the object file by the compiler, so we need to take care not to change
/// the layout.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct HeapSpec {
    /// A region of the heap that is addressable, but only a subset of it is accessible.
    ///
    /// Specified in bytes, and must be evenly divisible by the host page size (4K).
    pub reserved_size: uint64_t,

    /// A region of the heap that is addressable, but never accessible.
    ///
    /// Specified in bytes, and must be evenly divisible by the host page size (4K).
    pub guard_size: uint64_t,

    /// The amount of heap that is accessible upon initialization.
    ///
    /// Specified in bytes, must be evenly divisible by the WebAssembly page size (64K), and must be less than or equal to `reserved_size`.
    pub initial_size: uint64_t,

    /// The maximum amount of the heap that the program will request; only valid if `max_size_valid == 1`.
    ///
    /// This comes directly from the WebAssembly program's memory definition.
    pub max_size: uint64_t,

    /// Set to `1` when `max_size` is valid, and `0` when it is not.
    ///
    /// This will eventually be nicer once we are using an IDL.
    pub max_size_valid: uint64_t,
}

impl Default for HeapSpec {
    fn default() -> HeapSpec {
        // from the lucet tests' `helpers.h`
        HeapSpec {
            reserved_size: 4 * 1024 * 1024,
            guard_size: 4 * 1024 * 1024,
            initial_size: 64 * 1024,
            max_size: 64 * 1024,
            max_size_valid: 1,
        }
    }
}

/// Specifications from the compiled WebAssembly module about its heap and globals.
#[derive(Clone, Debug, Default)]
pub struct RuntimeSpec {
    pub heap: HeapSpec,
    pub globals: Vec<i64>,
}

/// Details about a program address.
///
/// It is possible to determine whether an address lies within the module code if the module is
/// loaded from a shared object. Statically linked modules are not resolvable. Best effort is made
/// to resolve the symbol the address is found inside, and the file that symbol is found in. See
/// `dladdr(3)` for more details.
#[derive(Clone, Debug)]
pub struct AddrDetails {
    pub in_module_code: bool,
    pub file_name: Option<String>,
    pub sym_name: Option<String>,
}

impl DlModule {
    /// Create a module, loading code from a shared object on the filesystem.
    pub fn load<P: AsRef<OsStr>>(so_path: P) -> Result<Self, Error> {
        // Load the dynamic library. The undefined symbols corresponding to the lucet_syscall_
        // functions will be provided by the current executable.  We trust our wasm->dylib compiler
        // to make sure these function calls are the way the dylib can touch memory outside of its
        // stack and heap.
        let lib = Library::new(so_path)?;

        let heap_ptr = unsafe { lib.get::<*const HeapSpec>(b"lucet_heap_spec")? };
        let heap: HeapSpec = unsafe {
            heap_ptr
                .as_ref()
                .ok_or(format_err!("null wasm memory spec"))?
                .clone()
        };

        let fbase = if let Some(dli) = dladdr(*heap_ptr as *const c_void) {
            dli.dli_fbase
        } else {
            std::ptr::null()
        };

        let globals = unsafe {
            globals::read_from_module({
                let spec = lib.get::<*const GlobalsSpec>(b"lucet_globals_spec")?;
                let spec_raw: *const GlobalsSpec = *spec;
                spec_raw
            })?
        };
        let runtime_spec = RuntimeSpec { heap, globals };

        let trap_manifest = unsafe {
            if let Ok(len_ptr) = lib.get::<*const u32>(b"lucet_trap_manifest_len") {
                let len = len_ptr.as_ref().expect("non-null trap manifest length");
                let records = lib
                    .get::<*const TrapManifestRecord>(b"lucet_trap_manifest")?
                    .as_ref()
                    .ok_or(format_err!("null trap manifest records"))?;
                from_raw_parts(records, *len as usize)
            } else {
                &[]
            }
        };

        Ok(DlModule {
            lib,
            fbase,
            runtime_spec,
            trap_manifest,
        })
    }
}

impl Module for DlModule {
    fn table_elements(&self) -> Result<&[TableElement], Error> {
        let p_table_segment: Symbol<*const TableElement> =
            unsafe { self.lib.get(b"guest_table_0")? };
        let p_table_segment_len: Symbol<*const usize> =
            unsafe { self.lib.get(b"guest_table_0_len")? };
        let len = unsafe { **p_table_segment_len };
        let elem_size = mem::size_of::<TableElement>();
        if len > std::u32::MAX as usize * elem_size {
            bail!("table segment too long: {}", len);
        }
        if len % elem_size != 0 {
            bail!(
                "table segment length not a multiple of table element size: {}",
                len
            );
        }
        Ok(unsafe { from_raw_parts(*p_table_segment, **p_table_segment_len as usize / elem_size) })
    }

    fn sparse_page_data(&self) -> Result<&[*const c_void], Error> {
        unsafe {
            let spd = self
                .lib
                .get::<*const SparsePageData>(b"guest_sparse_page_data")?
                .as_ref()
                .ok_or(format_err!("null wasm sparse page data"))?;
            Ok(from_raw_parts(&spd.pages, spd.num_pages as usize))
        }
    }

    fn runtime_spec(&self) -> &RuntimeSpec {
        &self.runtime_spec
    }

    fn get_export_func(&self, sym: &[u8]) -> Result<*const extern "C" fn(), Error> {
        let mut guest_sym: Vec<u8> = b"guest_func_".to_vec();
        guest_sym.extend_from_slice(sym);
        let f = unsafe { self.lib.get::<*const extern "C" fn()>(&guest_sym)? };
        // eprintln!("{} at {:p}", String::from_utf8_lossy(sym), *f);
        Ok(*f)
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
                bail!("guest_start symbol exists but contains a null pointer");
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

pub trait Module {
    /// Get the table elements from the module.
    fn table_elements(&self) -> Result<&[TableElement], Error>;

    /// Returns the sparse page data encoded into the module object.
    ///
    /// Indices into the returned slice correspond to the offset, in host page increments, from the
    /// base of the instance heap.
    ///
    /// If the pointer at a given index is null, there is no data for that page. Otherwise, it
    /// should be a pointer to the base of a host page-sized area of data.
    ///
    /// This method does no checking to ensure the above is valid; it relies on the correctness of
    /// `lucetc`'s output.
    fn sparse_page_data(&self) -> Result<&[*const c_void], Error>;

    fn runtime_spec(&self) -> &RuntimeSpec;

    /// Get the heap specification encoded into the module by `lucetc`.
    fn heap_spec(&self) -> &HeapSpec {
        &self.runtime_spec().heap
    }

    /// Get the WebAssembly globals of the module.
    ///
    /// The indices into the returned slice correspond to the WebAssembly indices of the globals
    /// (<https://webassembly.github.io/spec/core/syntax/modules.html#syntax-globalidx>)
    fn globals(&self) -> &[i64] {
        &self.runtime_spec().globals
    }

    fn get_export_func(&self, sym: &[u8]) -> Result<*const extern "C" fn(), Error>;

    fn get_start_func(&self) -> Result<Option<*const extern "C" fn()>, Error>;

    fn trap_manifest(&self) -> &[TrapManifestRecord];

    fn addr_details(&self, addr: *const c_void) -> Result<Option<AddrDetails>, Error>;

    /// Look up an instruction pointer in the trap manifest.
    ///
    /// This function must be signal-safe.
    fn lookup_trapcode(&self, rip: *const c_void) -> Option<TrapCode> {
        for record in self.trap_manifest() {
            if record.contains_addr(rip) {
                // the trap falls within a known function
                if let Some(trapcode) = record.lookup_addr(rip) {
                    return Some(trapcode);
                } else {
                    // stop looking through the rest of the trap manifests; only one function should
                    // ever match
                    break;
                }
            }
        }

        // handle the special case when the probe stack is running
        let probestack = lucet_probestack as *const c_void;
        if rip >= probestack
            && rip as usize <= probestack as usize + unsafe { lucet_probestack_size } as usize
        {
            Some(TrapCode {
                ty: TrapCodeType::StackOverflow,
                tag: std::u16::MAX,
            })
        } else {
            // we couldn't find a trapcode
            None
        }
    }
}

pub struct MockModule {
    pub table_elements: Vec<TableElement>,
    pub sparse_page_data: Vec<*const c_void>,
    pub runtime_spec: RuntimeSpec,
    pub export_funcs: HashMap<Vec<u8>, *const extern "C" fn()>,
    pub start_func: Option<extern "C" fn()>,
    pub trap_manifest: Vec<TrapManifestRecord>,
}

impl MockModule {
    pub fn new() -> Self {
        MockModule {
            table_elements: vec![],
            sparse_page_data: vec![],
            runtime_spec: RuntimeSpec::default(),
            export_funcs: HashMap::new(),
            start_func: None,
            trap_manifest: vec![],
        }
    }

    pub fn boxed() -> Box<dyn Module> {
        Box::new(MockModule::new())
    }

    pub fn boxed_with_heap(heap: &HeapSpec) -> Box<dyn Module> {
        let mut module = MockModule::new();
        module.runtime_spec.heap = heap.clone();
        Box::new(module)
    }
}

impl Module for MockModule {
    fn table_elements(&self) -> Result<&[TableElement], Error> {
        Ok(&self.table_elements)
    }

    fn sparse_page_data(&self) -> Result<&[*const c_void], Error> {
        Ok(&self.sparse_page_data)
    }

    fn runtime_spec(&self) -> &RuntimeSpec {
        &self.runtime_spec
    }

    fn get_export_func(&self, sym: &[u8]) -> Result<*const extern "C" fn(), Error> {
        let func = self.export_funcs.get(sym).ok_or(format_err!(
            "export func not found: {}",
            String::from_utf8_lossy(sym)
        ))?;
        // eprintln!("{} at {:p}", String::from_utf8_lossy(sym), *func);
        Ok(*func)
    }

    fn get_start_func(&self) -> Result<Option<*const extern "C" fn()>, Error> {
        Ok(self.start_func.map(|start| start as *const extern "C" fn()))
    }

    fn trap_manifest(&self) -> &[TrapManifestRecord] {
        &self.trap_manifest
    }

    fn addr_details(&self, _addr: *const c_void) -> Result<Option<AddrDetails>, Error> {
        // TODO: possible to reflect on size of Rust functions?
        Ok(None)
    }
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
