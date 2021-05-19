lucet_wiggle::lucet_integration!({
    target: wasi_common::snapshots::preview_1,
    witx: ["$WASI_ROOT/phases/snapshot/witx/wasi_snapshot_preview1.witx"],
    ctx: { wasi_common::WasiCtx },
    async: *,
});

pub mod types {
    pub use wasi_common::snapshots::preview_1::types::*;
}

pub fn export_wasi_funcs() {
    hostcalls::init()
}
