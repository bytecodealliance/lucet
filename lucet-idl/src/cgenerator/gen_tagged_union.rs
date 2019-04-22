use super::*;

pub fn generate<W: Write>(
    cgenerator: &mut CGenerator,
    data_description_helper: &DataDescriptionHelper,
    cache: &mut Cache,
    pretty_writer: &mut PrettyWriter<W>,
    data_type_entry: &DataTypeEntry<'_>,
) -> Result<(), IDLError> {
    let (named_members, _attrs) = if let DataType::TaggedUnion {
        members: named_members,
        attrs,
    } = &data_type_entry.data_type
    {
        (named_members, attrs)
    } else {
        unreachable!()
    };

    for (i, named_member) in named_members.iter().enumerate() {
        let internal_union_type_id = i + 1;
        macros::define(
            cgenerator,
            pretty_writer,
            "TYPE",
            format!("{}_{}", data_type_entry.name.name, named_member.name).as_str(),
            internal_union_type_id,
        )?;
    }
    pretty_writer.eob()?;
    pretty_writer.write_line(format!("struct {} {{", data_type_entry.name.name).as_bytes())?;
    let mut pretty_writer_i1 = pretty_writer.new_block();
    let mut offset: usize = 0;

    // The first member of the structure is the variant type
    // This will also define the required alignment for the tagged union
    // Variant types start at `1` and not `0` to help find possibly uninitialized variable
    let tagged_union_type_native_type = CAtom::tagged_union_type();
    let (tagged_union_type_align, tagged_union_type_size, tagged_union_type_name) = (
        tagged_union_type_native_type.native_type_align,
        tagged_union_type_native_type.native_type_size,
        tagged_union_type_native_type.native_type_name,
    );
    let first_member_align = Some(tagged_union_type_align);
    pretty_writer_i1.indent()?;
    pretty_writer_i1.write(tagged_union_type_name.as_bytes())?;
    pretty_writer_i1.space()?;
    pretty_writer_i1.write(
        format!(
            "___type; // tagged union type, should be in the [1...{}] range",
            named_members.len()
        )
        .as_bytes(),
    )?;
    pretty_writer_i1.eol()?;
    offset += tagged_union_type_size;

    let mut includes_pointers = false;

    // The size of the union is the size of the largest member.
    // Ditto for the alignment.
    // In order to know how much padding is required between the variant type and the
    // union, find these maximum values first.
    // We also need to find if we have at least one pointer. In that case, additional
    // padding at the end of the structure may be required for targets with 32-bit
    // pointers.
    let mut max_align = 0;
    let mut max_size = 0;
    for named_member in named_members {
        if let Some(type_) = named_member.type_.as_ref() {
            let type_info = cgenerator.type_info(data_description_helper, cache, type_);
            if type_info.type_align > max_align {
                max_align = type_info.type_align;
            }
            if type_info.type_size > max_size {
                max_size = type_info.type_size;
            }
            if type_info.indirections > 0 {
                includes_pointers = true;
            }
        }
    }

    // Add optional padding between the variant type and the union
    let padding = (max_align - 1) - ((offset + (max_align - 1)) % max_align);
    if padding > 0 {
        pretty_writer_i1.write_line(
            format!("uint8_t ___pad{}_{}[{}];", max_align, offset, padding).as_bytes(),
        )?;
        offset += padding;
    }

    let variant_offset = offset;

    pretty_writer_i1.write_line(b"union {")?;
    let mut pretty_writer_i2 = pretty_writer_i1.new_block();
    for (i, named_member) in named_members.iter().enumerate() {
        let internal_union_type_id = i + 1;
        if named_member.type_.is_none() {
            // Untyped union member
            pretty_writer_i2.write_line(
                format!(
                    "// void {}; - type {}",
                    named_member.name, internal_union_type_id
                )
                .as_bytes(),
            )?;
            continue;
        }
        let type_ = named_member.type_.as_ref().unwrap();
        let type_info = cgenerator.type_info(data_description_helper, cache, type_);
        pretty_writer_i2.indent()?;
        pretty_writer_i2.write(type_info.type_name.as_bytes())?;
        pretty_writer_i2.space()?;
        for _ in 0..type_info.indirections {
            pretty_writer_i2.write(b"*")?;
        }
        pretty_writer_i2.write(named_member.name.as_bytes())?;
        pretty_writer_i2.write(b";")?;
        pretty_writer_i2.space()?;
        pretty_writer_i2.write(format!("// - type {}", internal_union_type_id).as_bytes())?;
        pretty_writer_i2.eol()?;
    }

    pretty_writer_i1.write_line(b"} variant;")?;
    offset += max_size;

    // If the tagged enum includes pointers, we may have to extend its size
    // to match what it would be on a target with 64-bit pointers
    if includes_pointers {
        cgenerator.pointer_pad(&mut pretty_writer_i1, 1, offset, "inside tagged union")?;
    }

    pretty_writer.write_line(b"};")?;
    pretty_writer.eob()?;

    let struct_align = first_member_align.unwrap();
    let struct_size = offset;
    cache.store_type(
        data_type_entry.id,
        CachedTypeEntry {
            type_size: struct_size,
            type_align: struct_align,
            members: vec![CachedStructMemberEntry {
                offset: variant_offset,
            }],
        },
    );

    // Add an assertion to check that the variant is properly aligned
    pretty_writer.write_line(
        format!(
            "_Static_assert(offsetof(struct {}, variant) == {}, \"unexpected variant offset\");",
            data_type_entry.name.name, variant_offset
        )
        .as_bytes(),
    )?;

    // Add an assertion to check that the structure size matches what we expect
    pretty_writer.write_line(
        format!(
            "_Static_assert(sizeof(struct {}) == {}, \"unexpected tagged union structure size\");",
            data_type_entry.name.name, struct_size
        )
        .as_bytes(),
    )?;
    pretty_writer.eob()?;

    // Add a macro for the tagged union size
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
