use super::*;

pub fn generate<W: Write>(
    cgenerator: &mut CGenerator,
    data_description_helper: &DataDescriptionHelper,
    cache: &Cache,
    pretty_writer: &mut PrettyWriter<W>,
    type_: &DataTypeRef,
    hierarchy: &Hierarchy,
) -> Result<(), IDLError> {
    let type_info = cgenerator.type_info(data_description_helper, cache, type_);
    let leaf_type_info =
        cgenerator.type_info(data_description_helper, cache, type_info.leaf_data_type_ref);
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
