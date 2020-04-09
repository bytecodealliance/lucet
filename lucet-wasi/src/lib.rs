#![deny(bare_trait_objects)]

#[cfg(feature = "runtime")]
pub mod c_api;
#[cfg(feature = "runtime")]
pub mod runtime;

#[cfg(feature = "runtime")]
pub use runtime::*;
// Wasi-common re-exports:
pub use wasi_common::{WasiCtx, WasiCtxBuilder};

// Wasi executables export the following symbol for the entry point:
pub const START_SYMBOL: &str = "_start";

pub fn bindings() -> lucet_module::bindings::Bindings {
    lucet_wiggle_generate::bindings(&wasi_common::wasi::metadata::document())
}

pub fn document() -> wiggle::witx::Document {
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
