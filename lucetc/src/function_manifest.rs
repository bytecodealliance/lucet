use crate::error::Error;
use crate::output::write_relocated_slice;
use crate::traps::trap_sym_for_func;
use faerie::{Artifact, Decl};
use lucet_module::FunctionSpec;
use std::io::Cursor;
use std::mem::size_of;

pub const FUNCTION_MANIFEST_SYM: &str = "lucet_function_manifest";

///
/// Writes a manifest of functions, with relocations, to the artifact.
///
pub fn write_function_manifest(
    functions: &[(String, FunctionSpec)],
    obj: &mut Artifact,
) -> Result<(), Error> {
    obj.declare(FUNCTION_MANIFEST_SYM, Decl::data())
        .map_err(|source| {
            let message = format!("Manifest error declaring {}", FUNCTION_MANIFEST_SYM);
            Error::ArtifactError(source, message)
        })?;

    let mut manifest_buf: Cursor<Vec<u8>> = Cursor::new(Vec::with_capacity(
        functions.len() * size_of::<FunctionSpec>(),
    ));

    for (fn_name, fn_spec) in functions.iter() {
        /*
         * This has implicit knowledge of the layout of `FunctionSpec`!
         *
         * Each iteraction writes out bytes with relocations that will
         * result in data forming a valid FunctionSpec when this is loaded.
         *
         * Because the addresses don't exist until relocations are applied
         * when the artifact is loaded, we can't just populate the fields
         * and transmute, unfortunately.
         */
        // Writes a (ptr, len) pair with relocation for code
        write_relocated_slice(
            obj,
            &mut manifest_buf,
            FUNCTION_MANIFEST_SYM,
            Some(fn_name),
            fn_spec.code_len() as u64,
        )?;
        // Writes a (ptr, len) pair with relocation for this function's trap table
        let trap_sym = trap_sym_for_func(fn_name);
        write_relocated_slice(
            obj,
            &mut manifest_buf,
            FUNCTION_MANIFEST_SYM,
            if fn_spec.traps_len() > 0 {
                Some(&trap_sym)
            } else {
                None
            },
            fn_spec.traps_len() as u64,
        )?;
    }

    obj.define(FUNCTION_MANIFEST_SYM, manifest_buf.into_inner())
        .map_err(|source| {
            let message = format!("Manifest error declaring {}", FUNCTION_MANIFEST_SYM);
            Error::ArtifactError(source, message)
        })?;

    Ok(())
}
