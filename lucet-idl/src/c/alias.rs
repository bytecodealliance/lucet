use super::*;

pub fn generate(
    gen: &mut CGenerator,
    module: &Module,
    dt: &Named<DataType>,
    alias: &AliasDataType,
) -> Result<(), IDLError> {
    let dtname = &dt.name.name;
    gen.w.indent()?;
    gen.w.writeln(format!(
        "typedef {} {};",
        gen.type_name(module, &alias.to),
        dtname
    ))?;
    gen.w.eob()?;

    // Add an assertion to check that resolved size is the one we computed
    gen.w.writeln(format!(
        "_Static_assert(sizeof({}) == {}, \"unexpected alias size\");",
        dtname, dt.entity.repr_size
    ))?;
    gen.w.eob()?;

    Ok(())
}
