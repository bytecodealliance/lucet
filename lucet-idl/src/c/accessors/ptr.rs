use crate::c::CGenerator;
use crate::cache::Cache;
use crate::errors::IDLError;
use crate::generator::Hierarchy;
use crate::module::{DataTypeRef, Module};
use crate::pretty_writer::PrettyWriter;
use std::io::prelude::*;

pub fn generate<W: Write>(
    cgenerator: &mut CGenerator,
    module: &Module,
    cache: &Cache,
    pretty_writer: &mut PrettyWriter<W>,
    type_: &DataTypeRef,
    hierarchy: &Hierarchy,
) -> Result<(), IDLError> {
    let type_info = cgenerator.type_info(module, cache, type_);
    let leaf_type_info = cgenerator.type_info(module, cache, type_info.leaf_data_type_ref);
    assert_eq!(leaf_type_info.indirections, 0);
    if type_info.indirections > 1 {
        pretty_writer.write_line(
            format!(
                "// `{}` is a sequence of {} pointers to `{}`.",
                hierarchy.idl_name(),
                type_info.indirections,
                leaf_type_info.type_name
            )
            .as_ref(),
        )?;
    } else if type_info.indirections == 1 {
        pretty_writer.write_line(
            format!(
                "// `{}` is a native pointer to a pointer to `{}`.",
                hierarchy.idl_name(),
                leaf_type_info.type_name
            )
            .as_ref(),
        )?;
    } else {
        pretty_writer.write_line(
            format!(
                "// `{}` is a native pointer to `{}`.",
                hierarchy.idl_name(),
                leaf_type_info.type_name
            )
            .as_ref(),
        )?;
    }
    pretty_writer.write_line(b"// Accessors are intentionally not defined for it: ")?;
    pretty_writer
        .write_line(
            b"// it is only designed to be used internally, and its value will not be serialized.",
        )?
        .eob()?;

    Ok(())
}
