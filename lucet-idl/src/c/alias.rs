use super::*;

// The most important thing in alias generation is to cache the size
// and alignment rules of what it ultimately points to
pub fn generate(
    gen: &mut CGenerator,
    module: &Module,
    data_type_entry: &Named<DataType>,
    alias: &AliasDataType,
) -> Result<(), IDLError> {
    let type_info = gen.type_info(module, &alias.to);
    gen.w.indent()?;
    gen.w
        .write(format!("typedef {}", type_info.type_name).as_bytes())?;
    gen.w.space()?;
    gen.w.write(data_type_entry.name.name.as_bytes())?;
    gen.w.write(b";")?;
    let leaf_type_info = gen.type_info(module, type_info.leaf_data_type_ref);
    if leaf_type_info.type_name != type_info.type_name {
        gen.w.write(b" // equivalent to ")?;
        gen.w.write(leaf_type_info.type_name.as_bytes())?;
    }
    gen.w.eol()?;
    gen.w.eob()?;
    gen.cache.store_type(
        data_type_entry.id,
        CachedTypeEntry {
            type_size: type_info.type_size,
            type_align: type_info.type_align,
            members: vec![],
        },
    );

    // Add an assertion to check that resolved size is the one we computed
    gen.w.write_line(
        format!(
            "_Static_assert(sizeof({}) == {}, \"unexpected alias size\");",
            data_type_entry.name.name, type_info.type_size
        )
        .as_bytes(),
    )?;
    gen.w.eob()?;

    Ok(())
}
