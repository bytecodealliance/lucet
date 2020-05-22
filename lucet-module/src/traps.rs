/// The type of a WebAssembly
/// [trap](http://webassembly.github.io/spec/core/intro/overview.html#trap).
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TrapCode {
    StackOverflow,
    HeapOutOfBounds,
    IndirectCallToNull,
    BadSignature,
    IntegerOverflow,
    IntegerDivByZero,
    BadConversionToInteger,
    Interrupt,
    TableOutOfBounds,
    Unreachable,
}

/// Trap information for an address in a compiled function
///
/// To support zero-copy deserialization of trap tables, this
/// must be repr(C) [to avoid cases where Rust may change the
/// layout in some future version, mangling the interpretation
/// of an old TrapSite struct]
#[repr(C)]
#[derive(Clone, Debug)]
pub struct TrapSite {
    pub offset: u32,
    pub code: TrapCode,
}

/// A collection of trap sites, typically obtained from a
/// single function (see [`FunctionSpec::traps`])
#[repr(C)]
#[derive(Clone, Debug)]
pub struct TrapManifest<'a> {
    pub traps: &'a [TrapSite],
}

impl<'a> TrapManifest<'a> {
    pub fn new(traps: &'a [TrapSite]) -> TrapManifest<'_> {
        TrapManifest { traps }
    }
    pub fn lookup_addr(&self, addr: u32) -> Option<TrapCode> {
        // predicate to find the trapsite for the addr via binary search
        let f = |ts: &TrapSite| ts.offset.cmp(&addr);

        if let Ok(i) = self.traps.binary_search_by(f) {
            Some(self.traps[i].code)
        } else {
            None
        }
    }
}
