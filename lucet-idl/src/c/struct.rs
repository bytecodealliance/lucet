use super::*;
use std::cmp;

pub fn generate(
    gen: &mut CGenerator,
    module: &Module,
    data_type_entry: &Named<DataType>,
    struct_: &StructDataType,
) -> Result<(), IDLError> {
    gen.w
        .write_line(format!("struct {} {{", data_type_entry.name.name).as_bytes())?;
    let mut w_block = gen.w.new_block();
    let mut offset: usize = 0;
    let mut first_member_align = 0;
    let mut members_offsets = vec![];
    for named_member in struct_.members.iter() {
        let type_info = gen.type_info(module, &named_member.type_);
        let type_align = type_info.type_align;
        let type_size = type_info.type_size;
        let padding = (type_align - 1) - ((offset + (type_align - 1)) % type_align);
        if padding > 0 {
            w_block.write_line(
                format!("uint8_t ___pad{}_{}[{}];", type_align, offset, padding).as_bytes(),
            )?;
            offset += padding;
        }
        members_offsets.push(offset);
        w_block.indent()?;
        w_block.write(type_info.type_name.as_bytes())?;
        w_block.space()?;
        w_block.write(named_member.name.as_bytes())?;
        w_block.write(b";")?;
        w_block.eol()?;

        offset += type_size;
        first_member_align = cmp::max(first_member_align, type_align);
    }
    gen.w.write_line(b"};")?;
    gen.w.eob()?;

    // cache the total structure size, as well as its alignment, which is equal
    // to the alignment of the first member of that structure
    let struct_align = first_member_align;
    let struct_size = offset;
    let cached = gen.cache.store_type(
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
    for (i, named_member) in struct_.members.iter().enumerate().skip(1) {
        gen.w.write_line(
            format!(
                "_Static_assert(offsetof(struct {}, {}) == {}, \"unexpected offset\");",
                data_type_entry.name.name, named_member.name, members_offsets[i]
            )
            .as_bytes(),
        )?;
    }
    gen.w.write_line(
        format!(
            "_Static_assert(sizeof(struct {}) == {}, \"unexpected structure size\");",
            data_type_entry.name.name, struct_size
        )
        .as_bytes(),
    )?;
    gen.w.eob()?;
    macros::define(gen, "BYTES", &data_type_entry.name.name, struct_size)?;
    gen.w.eob()?;

    Ok(())
}
