#![deny(bare_trait_objects)]

pub mod c_api;
pub mod runtime;

pub use runtime::*;
// Wasi-common re-exports:
pub use wasi_common::{WasiCtx, WasiCtxBuilder, WasiCtxBuilderError};

/// Wasi executables export the following symbol for the entry point:
pub const START_SYMBOL: &str = "_start";

/// Bindings for the hostcalls exposed by this crate. These are identical to the bindings in
/// `bindings.json`. These are exposed as part of a transition path away from bindings.json files.
pub fn bindings() -> lucet_module::bindings::Bindings {
    lucet_wiggle::bindings(&wasi_common::wasi::metadata::document())
}

/// The witx document for the interface implemented by this crate. This is exposed as part of a
/// transition path away from always loading witx documents from the filesystem.
pub fn witx_document() -> lucet_wiggle::witx::Document {
    wasi_common::wasi::metadata::document()
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;
    #[test]
    fn bindings_json_matches_crate() {
        // Check that the bindings.json provided in the crate sources matches the bindings
        // we expose as part of the crate.
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("bindings.json");
        let file = lucet_module::bindings::Bindings::from_file(path).expect("load bindings file");
        // Iterate through crate bindings, comparing to file, to give a friendly message
        // if anything is missing
        for (m, bs) in super::bindings().hash_map().iter() {
            let fbs = file
                .hash_map()
                .get(m)
                .expect(&format!("file has module {}", m));
            for (name, binding) in bs.iter() {
                let file_binding = fbs
                    .get(name)
                    .expect(&format!("bindings file missing {}:{}", name, binding));
                assert_eq!(
                    binding, file_binding,
                    "canonical vs file binding for module {}",
                    name
                );
            }
            assert_eq!(bs, fbs, "bindings for module {}", m);
        }

        // in case the above hasnt caught any differences:
        assert_eq!(
            super::bindings(),
            file,
            "crate bindings compared to bindings.json"
        );
    }
}
