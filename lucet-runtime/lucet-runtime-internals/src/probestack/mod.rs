extern "C" {
    pub fn lucet_probestack_private();
    pub static lucet_probestack_size: u32;
}

#[no_mangle]
pub unsafe extern "C" fn lucet_probestack() {
    // TODO: this is a hack to make sure the symbol `lucet_probestack` is exported in
    // `lucet_runtime.so`: see https://github.com/rust-lang/rust/issues/36342. As soon as that issue
    // is resolved, we should remove this to avoid any safety issues around the weird probestack
    // calling convention, and to avoid the overhead of an additional function call.
    lucet_probestack_private()
}
