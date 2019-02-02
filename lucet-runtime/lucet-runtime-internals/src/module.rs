mod dl;
mod globals;
mod mock;
mod sparse_page_data;

pub use crate::module::dl::DlModule;
pub use crate::module::mock::MockModule;

use crate::error::Error;
use crate::probestack::{lucet_probestack, lucet_probestack_size};
use crate::trapcode::{TrapCode, TrapCodeType};
use libc::{c_void, uint64_t};
use std::slice::from_raw_parts;

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

/// The read-only parts of a Lucet program, including its code and initial heap configuration.
///
/// Types that implement this trait are suitable for use with
/// [`Region::new_instance()`](trait.Region.html#method.new_instance).
pub trait Module: ModuleInternal {}

pub trait ModuleInternal {
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

    fn get_export_func_from_id(
        &self,
        table_id: u32,
        func_id: u32,
    ) -> Result<*const extern "C" fn(), Error>;

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

    /// This is a hack to make sure we don't DCE away the `lucet_vmctx_*` C API
    ///
    /// It's on this trait because no guest code can run without using some instance of `Module`,
    /// but could've gone on `Region`.
    ///
    /// This should never actually get called, but it is harmless if it is.
    #[doc(hidden)]
    fn ensure_linked(&self) {
        crate::vmctx::vmctx_capi_init();
    }
}
