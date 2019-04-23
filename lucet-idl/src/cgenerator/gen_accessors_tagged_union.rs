use super::*;

pub fn generate<W: Write>(
    cgenerator: &mut CGenerator,
    data_type_entry: &DataTypeEntry<'_>,
    data_description_helper: &DataDescriptionHelper,
    cache: &Cache,
    pretty_writer: &mut PrettyWriter<W>,
    internal_union_type_id: usize,
    named_member: &NamedMember<Option<DataTypeRef>>,
    hierarchy: &Hierarchy,
) -> Result<(), IDLError> {
    let fn_name = hierarchy.fn_name();
    let root_name = hierarchy.root_name();
    let root_macro_size_name = macros::macro_for("BYTES", &root_name);
    let name = &named_member.name;
    let member_type = named_member.type_.as_ref();
    let cached_type_entry = cache
        .load_type(data_type_entry.id)
        .expect("Type not cached");
    let first_member_offset = cached_type_entry
        .load_member(0)
        .expect("No member entry")
        .offset;
    let mut member_type_size = 0;
    if let Some(member_type) = member_type {
        let type_info = cgenerator.type_info(data_description_helper, cache, member_type);
        member_type_size = type_info.type_size;
    }
    let mut pretty_writer_i1 = pretty_writer.new_block();
    let mut pretty_writer_preprocessor = pretty_writer.new_from_writer();
    let member_macro_size_name = if let Some(member_type) = member_type {
        let unaliased_member_type = cgenerator.unalias(data_description_helper, member_type);
        macros::macro_for_data_type_ref(data_description_helper, "BYTES", unaliased_member_type)
    } else {
        CAtom::tagged_union_type().native_type_size.to_string()
    };
    pretty_writer.write_line(
        format!(
            "// The following set of functions accesses a tagged union `{}` as the internal type `{}`",
            fn_name, name
        )
        .as_ref(),
    )?;
    pretty_writer.eob()?;

    // --- is_*()

    pretty_writer.write_line(
        format!(
            "static inline bool is_{}_{}(const unsigned char buf[static {}])",
            fn_name, name, member_macro_size_name
        )
        .as_ref(),
    )?;
    pretty_writer.write_line(b"{")?;
    pretty_writer_i1
        .write_line(format!("{} type;", CAtom::tagged_union_type().native_type_name).as_ref())?
        .eob()?;
    pretty_writer_i1.write_line(
        format!(
            "_Static_assert({} >= sizeof type, \"unexpected size\");",
            member_macro_size_name
        )
        .as_ref(),
    )?;
    pretty_writer_i1
        .write_line(b"memcpy(&type, &buf[0], sizeof type);")?
        .eob()?;
    pretty_writer_i1.write_line(
        format!(
            "return type == {};",
            CAtom::tagged_union_type()
                .little_endian(cgenerator.target, &format!("{}", internal_union_type_id))
        )
        .as_ref(),
    )?;
    pretty_writer.write_line(b"}")?.eob()?;

    // --- set_*()

    if member_type.is_none() {
        pretty_writer.write_line(
            format!(
                "static inline void set_{}_{}(unsigned char buf[static {}])",
                fn_name, name, member_macro_size_name
            )
            .as_ref(),
        )?;
        pretty_writer.write_line(b"{")?;
        pretty_writer_i1
            .write_line(
                format!(
                    "const {} type = {};",
                    CAtom::tagged_union_type().native_type_name,
                    CAtom::tagged_union_type()
                        .little_endian(cgenerator.target, &format!("{}", internal_union_type_id))
                )
                .as_bytes(),
            )?
            .eob()?;

        pretty_writer_i1.write_line(b"memcpy(&buf[0], &type, sizeof type);")?;
        pretty_writer.write_line(b"}")?.eob()?;
        return Ok(());
    }

    // --- ref_*()

    pretty_writer.write_line(format!("static inline void ref_{}_{}(", fn_name, name).as_ref())?;
    pretty_writer
        .continuation()?
        .write(b"const unsigned char **ibuf_p,")?
        .eol()?;
    pretty_writer
        .continuation()?
        .write(format!("const unsigned char buf[static {}])", root_macro_size_name).as_ref())?
        .eol()?;
    pretty_writer.write_line(b"{")?;
    pretty_writer_i1.write_line(format!("assert(is_{}_{}(buf));", fn_name, name).as_ref())?;
    pretty_writer_i1
        .write_line(format!("*ibuf_p = &buf[offsetof(struct {}, variant)];", fn_name).as_ref())?;
    pretty_writer_i1.eob()?;
    pretty_writer_i1.write_line(
        format!(
            "_Static_assert({} >= offsetof(struct {}, variant) + {}, \"unexpected size\");",
            root_macro_size_name, fn_name, member_type_size
        )
        .as_ref(),
    )?;
    pretty_writer.write_line(b"}")?.eob()?;

    // --- mut_*()

    pretty_writer.write_line(format!("static inline void mut_{}_{}(", fn_name, name).as_ref())?;
    pretty_writer
        .continuation()?
        .write(b"unsigned char **ibuf_p,")?
        .eol()?;
    pretty_writer
        .continuation()?
        .write(format!("unsigned char buf[static {}])", root_macro_size_name).as_ref())?
        .eol()?;
    pretty_writer.write_line(b"{")?;
    pretty_writer_i1.write_line(format!("assert(is_{}_{}(buf));", fn_name, name).as_ref())?;
    pretty_writer_i1
        .write_line(format!("*ibuf_p = &buf[offsetof(struct {}, variant)];", fn_name).as_ref())?;
    pretty_writer_i1.eob()?;
    pretty_writer_i1.write_line(
        format!(
            "_Static_assert({} >= offsetof(struct {}, variant) + {}, \"unexpected size\");",
            root_macro_size_name, fn_name, member_type_size
        )
        .as_ref(),
    )?;
    pretty_writer.write_line(b"}")?.eob()?;

    // --- Accessors for inner members

    let type_ = member_type.expect("Empty member");
    let hierarchy1 = hierarchy.push(named_member.name.to_string(), first_member_offset);
    cgenerator.gen_accessors_for_data_type_ref(
        data_description_helper,
        cache,
        pretty_writer,
        type_,
        &named_member.name,
        &hierarchy1,
    )?;

    // --- Accessors for the whole tagged union

    let type_size = cached_type_entry.type_size;
    let union_macro_size_name = macros::macro_for("BYTES", &fn_name);
    let is_eventually_an_atom_or_enum =
        cgenerator.is_type_eventually_an_atom_or_enum(data_description_helper, type_);

    pretty_writer
        .write_line(
            format!(
                "// Store the whole `{}` tagged union, when used as type `{}`",
                fn_name, name
            )
            .as_ref(),
        )?
        .eob()?;
    pretty_writer.write_line(
        format!(
            "static inline void store_{}_as_{}(unsigned char buf[static {}], const struct {} *t)",
            fn_name, name, union_macro_size_name, fn_name
        )
        .as_ref(),
    )?;
    pretty_writer.write_line(b"{")?;
    pretty_writer_i1.write_line(
        format!(
            "_Static_assert({} == {}, \"unexpected size\");",
            union_macro_size_name, type_size
        )
        .as_ref(),
    )?;
    pretty_writer_i1
        .write_line(format!("assert(t->___type == {});", internal_union_type_id).as_ref())?
        .eob()?;
    pretty_writer_i1.write_line(
        format!(
            "const {} type = {}; // {}",
            CAtom::tagged_union_type().native_type_name,
            CAtom::tagged_union_type()
                .little_endian(cgenerator.target, &format!("{}", internal_union_type_id)),
            name
        )
        .as_bytes(),
    )?;
    pretty_writer_i1
        .write_line(b"memcpy(&buf[0], &type, sizeof type);")?
        .eob()?;
    if let DataTypeRef::Ptr(_) = type_ {
        pretty_writer_preprocessor.write_line(b"# ifdef ZERO_NATIVE_POINTERS")?;
        pretty_writer_i1.write_line(
            format!(
                "memset(&buf[offsetof(struct {}, variant)], 0, sizeof *t);",
                fn_name
            )
            .as_ref(),
        )?;
        pretty_writer_preprocessor.write_line(b"# else")?;
        pretty_writer_i1.write_line(
            format!(
                "memcpy(&buf[offsetof(struct {}, variant)], &t->variant.{}, sizeof *t);",
                fn_name, name
            )
            .as_ref(),
        )?;
        pretty_writer_preprocessor.write_line(b"# endif")?;
    } else if is_eventually_an_atom_or_enum {
        pretty_writer_i1.write_line(
            format!(
                "store_{}_{}(&buf[offsetof(struct {}, variant)], t->variant.{});",
                fn_name, name, fn_name, name
            )
            .as_ref(),
        )?;
    } else {
        pretty_writer_i1.write_line(
            format!(
                "store_{}_{}(&buf[offsetof(struct {}, variant)], &t->variant.{});",
                fn_name, name, fn_name, name
            )
            .as_ref(),
        )?;
    }
    pretty_writer.write_line(b"}")?;

    Ok(())
}
