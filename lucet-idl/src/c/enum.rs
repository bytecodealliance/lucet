use super::*;

// Enums generate both a specific typedef, and a traditional C-style enum
// The typedef is required to use a native type which is consistent across all architectures
pub fn generate(
    gen: &mut CGenerator,
    _module: &Module,
    data_type_entry: &Named<DataType>,
) -> Result<(), IDLError> {
    let (named_members, _attrs) = if let DataType::Enum {
        members: named_members,
        attrs,
    } = &data_type_entry.entity
    {
        (named_members, attrs)
    } else {
        unreachable!()
    };

    let enum_native_type = CAtom::enum_();
    let (type_align, type_size, type_name) = (
        enum_native_type.native_type_align,
        enum_native_type.native_type_size,
        enum_native_type.native_type_name,
    );

    gen.w.write_line(
        format!(
            "typedef {} {}; // enum, should be in the [0...{}] range",
            type_name,
            data_type_entry.name.name,
            named_members.len() - 1
        )
        .as_bytes(),
    )?;
    gen.w
        .write_line(format!("enum ___{} {{", data_type_entry.name.name).as_bytes())?;
    let mut pretty_writer_i1 = gen.w.new_block();
    for (i, named_member) in named_members.iter().enumerate() {
        pretty_writer_i1.write_line(
            format!(
                "{}, // {}",
                macros::macro_for(&data_type_entry.name.name, &named_member.name),
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
            data_type_entry.name.name, type_size
        )
        .as_bytes(),
    )?;
    gen.w.eob()?;
    macros::define(gen, "BYTES", &data_type_entry.name.name, type_size)?;
    gen.w.eob()?;

    gen.cache.store_type(
        data_type_entry.id,
        CachedTypeEntry {
            type_size,
            type_align,
            members: vec![],
        },
    );
    Ok(())
}
