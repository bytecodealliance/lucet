use super::*;
use std::cmp;

pub fn generate<W: Write>(
    cgenerator: &mut CGenerator,
    module: &Module,
    pretty_writer: &mut PrettyWriter<W>,
    data_type_entry: &Named<DataType>,
) -> Result<(), IDLError> {
    let (named_members, _attrs) = if let DataType::Struct {
        members: named_members,
        attrs,
    } = &data_type_entry.entity
    {
        (named_members, attrs)
    } else {
        unreachable!()
    };
    pretty_writer.write_line(format!("struct {} {{", data_type_entry.name.name).as_bytes())?;
    let mut pretty_writer_i1 = pretty_writer.new_block();
    let mut offset: usize = 0;
    let mut first_member_align = 0;
    let mut members_offsets = vec![];
    for named_member in named_members {
        let type_info = cgenerator.type_info(module, &named_member.type_);
        let type_align = type_info.type_align;
        let type_size = type_info.type_size;
        let padding = (type_align - 1) - ((offset + (type_align - 1)) % type_align);
        if padding > 0 {
            pretty_writer_i1.write_line(
                format!("uint8_t ___pad{}_{}[{}];", type_align, offset, padding).as_bytes(),
            )?;
            offset += padding;
        }
        members_offsets.push(offset);
        pretty_writer_i1.indent()?;
        pretty_writer_i1.write(type_info.type_name.as_bytes())?;
        pretty_writer_i1.space()?;
        pretty_writer_i1.write(named_member.name.as_bytes())?;
        pretty_writer_i1.write(b";")?;
        pretty_writer_i1.eol()?;

        offset += type_size;
        first_member_align = cmp::max(first_member_align, type_align);
    }
    pretty_writer.write_line(b"};")?;
    pretty_writer.eob()?;

    // cache the total structure size, as well as its alignment, which is equal
    // to the alignment of the first member of that structure
    let struct_align = first_member_align;
    let struct_size = offset;
    let cached = cgenerator.cache.store_type(
        data_type_entry.id,
        CachedTypeEntry {
            type_size: struct_size,
            type_align: struct_align,
            members: vec![],
        },
    );

    // Cache members offsets
    cached.store_members(
        members_offsets
            .iter()
            .map(|&offset| CachedStructMemberEntry { offset })
            .collect::<Vec<_>>(),
    );

    // Add assertions to check that the target platform matches the expected alignment
    // Also add a macro definition for the structure size
    // Skip the first member, as it will always be at the beginning of the structure
    for (i, named_member) in named_members.iter().enumerate().skip(1) {
        pretty_writer.write_line(
            format!(
                "_Static_assert(offsetof(struct {}, {}) == {}, \"unexpected offset\");",
                data_type_entry.name.name, named_member.name, members_offsets[i]
            )
            .as_bytes(),
        )?;
    }
    pretty_writer.write_line(
        format!(
            "_Static_assert(sizeof(struct {}) == {}, \"unexpected structure size\");",
            data_type_entry.name.name, struct_size
        )
        .as_bytes(),
    )?;
    pretty_writer.eob()?;
    macros::define(
        cgenerator,
        pretty_writer,
        "BYTES",
        &data_type_entry.name.name,
        struct_size,
    )?;
    pretty_writer.eob()?;

    Ok(())
}
