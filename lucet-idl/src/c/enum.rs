use super::*;
use crate::types::AtomType;

// Enums generate both a specific typedef, and a traditional C-style enum
// The typedef is required to use a native type which is consistent across all architectures
pub fn generate(
    gen: &mut CGenerator,
    _module: &Module,
    dt: &Named<DataType>,
    enum_: &EnumDataType,
) -> Result<(), IDLError> {
    let type_size = dt.entity.repr_size;
    gen.w.write_line(
        format!(
            "typedef {} {}; // enum, should be in the [0...{}] range",
            CAtom::from(AtomType::U32).native_type_name,
            dt.name.name,
            enum_.members.len() - 1
        )
        .as_bytes(),
    )?;
    gen.w
        .write_line(format!("enum ___{} {{", dt.name.name).as_bytes())?;
    let mut pretty_writer_i1 = gen.w.new_block();
    for (i, named_member) in enum_.members.iter().enumerate() {
        pretty_writer_i1.write_line(
            format!(
                "{}, // {}",
                macros::macro_for(&dt.name.name, &named_member.name),
                i
            )
            .as_bytes(),
        )?;
    }
    gen.w.write_line(b"};")?;
    gen.w.eob()?;
    gen.w.write_line(
        format!(
            "_Static_assert(sizeof({}) == {}, \"unexpected enumeration size\");",
            dt.name.name, type_size
        )
        .as_bytes(),
    )?;
    gen.w.eob()?;
    macros::define(gen, "BYTES", &dt.name.name, type_size)?;
    gen.w.eob()?;

    Ok(())
}
