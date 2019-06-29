use crate::output::write_relocated_slice;
use failure::{Error, ResultExt};
use lucet_module::FunctionSpec;
use object::write::{Object, StandardSection, Symbol, SymbolId};
use object::{SymbolKind, SymbolScope};
use std::io::Cursor;
use std::mem::size_of;

pub const FUNCTION_MANIFEST_SYM: &str = "lucet_function_manifest";

///
/// Writes a manifest of functions, with relocations, to the artifact.
///
pub fn write_function_manifest(
    functions: &[(SymbolId, Option<SymbolId>, u32)],
    obj: &mut Object,
) -> Result<(), Error> {
    let mut manifest_buf: Cursor<Vec<u8>> = Cursor::new(Vec::with_capacity(
        functions.len() * size_of::<FunctionSpec>(),
    ));

    let mut relocs = Vec::new();
    for (fn_sym, trap_sym, trap_len) in functions.iter() {
        let fn_len = obj.symbol(*fn_sym).size;

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
        write_relocated_slice(&mut manifest_buf, &mut relocs, Some(*fn_sym), fn_len);
        // Writes a (ptr, len) pair with relocation for this function's trap table
        write_relocated_slice(
            &mut manifest_buf,
            &mut relocs,
            *trap_sym,
            u64::from(*trap_len),
        );
    }

    let section_id = obj.section_id(StandardSection::ReadOnlyDataWithRel);
    let manifest_buf = manifest_buf.into_inner();
    let manifest_offset = obj.append_section_data(section_id, &manifest_buf, 8);
    obj.add_symbol(Symbol {
        name: FUNCTION_MANIFEST_SYM.as_bytes().to_vec(),
        value: manifest_offset,
        size: manifest_buf.len() as u64,
        kind: SymbolKind::Data,
        scope: SymbolScope::Dynamic,
        weak: false,
        section: Some(section_id),
    });

    for mut reloc in relocs.drain(..) {
        reloc.offset += manifest_offset;
        obj.add_relocation(section_id, reloc)
            .map_err(failure::err_msg)
            .context("relocating function manifest")?;
    }

    Ok(())
}
