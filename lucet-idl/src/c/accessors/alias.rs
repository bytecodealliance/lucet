use crate::c::CGenerator;
use crate::cache::Cache;
use crate::errors::IDLError;
use crate::generator::Hierarchy;
use crate::module::{DataType, DataTypeEntry, Module};
use crate::pretty_writer::PrettyWriter;
use std::io::prelude::*;

pub fn generate<W: Write>(
    cgenerator: &mut CGenerator,
    module: &Module,
    cache: &Cache,
    pretty_writer: &mut PrettyWriter<W>,
    data_type_entry: &DataTypeEntry<'_>,
    hierarchy: &Hierarchy,
) -> Result<(), IDLError> {
    if hierarchy.depth() > 1 {
        return Ok(());
    }
    let name = &data_type_entry.name;
    let (type_, _attrs) = if let DataType::Alias { to: type_, attrs } = &data_type_entry.data_type {
        (type_, attrs)
    } else {
        unreachable!()
    };
    let type_info = cgenerator.type_info(module, cache, type_);

    pretty_writer.indent()?;
    pretty_writer
        .write(format!("// `{}` is an alias for `{}`", name, type_info.type_name).as_ref())?;
    if type_info.indirections == 0 {
        let leaf_type_info = cgenerator.type_info(module, cache, type_info.leaf_data_type_ref);
        if leaf_type_info.type_name != type_info.type_name {
            pretty_writer
                .write(format!(", itself equivalent to `{}`", leaf_type_info.type_name).as_ref())?;
        }
    }
    pretty_writer.write(b".")?.eol()?;
    pretty_writer.write_line(b"// No dedicated accessors have been generated.")?;
    pretty_writer.eob()?;
    Ok(())
}
