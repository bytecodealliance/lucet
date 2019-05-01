use super::*;

// The most important thing in alias generation is to cache the size
// and alignment rules of what it ultimately points to
pub fn generate<W: Write>(
    cgenerator: &mut CGenerator,
    module: &Module,
    cache: &mut Cache,
    pretty_writer: &mut PrettyWriter<W>,
    data_type_entry: &DataTypeEntry<'_>,
) -> Result<(), IDLError> {
    let (type_, _attrs) = if let DataType::Alias { to: type_, attrs } = &data_type_entry.data_type {
        (type_, attrs)
    } else {
        unreachable!()
    };
    let type_info = cgenerator.type_info(module, cache, type_);
    pretty_writer.indent()?;
    pretty_writer.write(format!("typedef {}", type_info.type_name).as_bytes())?;
    pretty_writer.space()?;
    pretty_writer.write(data_type_entry.name.name.as_bytes())?;
    pretty_writer.write(b";")?;
    let leaf_type_info = cgenerator.type_info(module, cache, type_info.leaf_data_type_ref);
    if leaf_type_info.type_name != type_info.type_name {
        pretty_writer.write(b" // equivalent to ")?;
        pretty_writer.write(leaf_type_info.type_name.as_bytes())?;
    }
    pretty_writer.eol()?;
    pretty_writer.eob()?;
    cache.store_type(
        data_type_entry.id,
        CachedTypeEntry {
            type_size: type_info.type_size,
            type_align: type_info.type_align,
            members: vec![],
        },
    );

    // Add an assertion to check that resolved size is the one we computed
    pretty_writer.write_line(
        format!(
            "_Static_assert(sizeof({}) == {}, \"unexpected alias size\");",
            data_type_entry.name.name, type_info.type_size
        )
        .as_bytes(),
    )?;
    pretty_writer.eob()?;

    Ok(())
}
