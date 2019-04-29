use crate::c::macros;
use crate::c::CGenerator;
use crate::cache::Cache;
use crate::errors::IDLError;
use crate::generator::Hierarchy;
use crate::module::{DataType, DataTypeEntry, DataTypeRef, Module};
use crate::pretty_writer::PrettyWriter;
use std::io::prelude::*;

pub fn generate<W: Write>(
    cgenerator: &mut CGenerator,
    module: &Module,
    cache: &Cache,
    pretty_writer: &mut PrettyWriter<W>,
    data_type_entry: &DataTypeEntry<'_>,
    hierarchy: &Hierarchy,
) -> Result<(), IDLError> {
    let root_name = hierarchy.root_name();
    let root_macro_size_name = macros::macro_for("BYTES", &root_name);
    let current_offset = hierarchy.current_offset();

    let (named_members, _attrs) = if let DataType::Struct {
        members: named_members,
        attrs,
    } = &data_type_entry.data_type
    {
        (named_members, attrs)
    } else {
        unreachable!()
    };
    pretty_writer
        .eob()?
        .write_line(
            format!(
                "// Platform-independent accessors for the members of the `{}` structure",
                data_type_entry.name
            )
            .as_bytes(),
        )?
        .eob()?;
    for (i, named_member) in named_members.iter().enumerate() {
        let offset = cache
            .load_type(data_type_entry.id)
            .unwrap()
            .load_member(i)
            .unwrap()
            .offset;
        let type_: &DataTypeRef = &named_member.type_;
        let hierarchy = hierarchy.push(named_member.name.to_string(), current_offset + offset);
        cgenerator.gen_accessors_for_data_type_ref(
            module,
            cache,
            &mut pretty_writer.new_block(),
            type_,
            &named_member.name,
            &hierarchy,
        )?;
    }

    pretty_writer
        .eob()?
        .write_line(
            format!(
                "// Platform-independent accessors for the whole `{}` structure",
                data_type_entry.name
            )
            .as_bytes(),
        )?
        .eob()?;
    let cached = cache.load_type(data_type_entry.id).unwrap();
    let fn_name = hierarchy.fn_name();
    let mut pretty_writer_i1 = pretty_writer.new_block();
    let mut pretty_writer_preprocessor = pretty_writer.new_from_writer();
    let is_reference_alignment_compatible = cgenerator.target.is_reference_alignment_compatible();
    let struct_macro_size_name = macros::macro_for("BYTES", &data_type_entry.name.name);

    // --- store_*()

    pretty_writer
        .write_line(
            format!(
                "static inline void store_{}(unsigned char buf[static {}], const struct {} *v)",
                fn_name, root_macro_size_name, data_type_entry.name
            )
            .as_bytes(),
        )?
        .write_line(b"{")?;
    pretty_writer_i1.write_line(
        format!(
            "_Static_assert({} == {}, \"unexpected size\");",
            struct_macro_size_name, cached.type_size
        )
        .as_bytes(),
    )?;
    if !is_reference_alignment_compatible {
        pretty_writer_preprocessor.write_line(b"#ifdef ___REFERENCE_COMPATIBLE_ENCODING")?;
    }
    pretty_writer_i1.write_line(
        format!("memcpy(&buf[{}], v, {});", current_offset, cached.type_size).as_bytes(),
    )?;
    if !is_reference_alignment_compatible {
        pretty_writer_preprocessor.write_line(b"#else")?;
        for (i, named_member) in named_members.iter().enumerate() {
            let offset = cache
                .load_type(data_type_entry.id)
                .unwrap()
                .load_member(i)
                .unwrap()
                .offset;
            let hierarchy = hierarchy.push(named_member.name.to_string(), offset);
            let fn_name = hierarchy.fn_name();
            let type_ = &named_member.type_;
            let is_eventually_an_atom_or_enum =
                cgenerator.is_type_eventually_an_atom_or_enum(module, type_);
            if let DataTypeRef::Ptr(_) = type_ {
                pretty_writer_preprocessor.write_line(b"# ifdef ZERO_NATIVE_POINTERS")?;
                pretty_writer_i1.write_line(
                    format!(
                        "memset(&buf[{} + {}], 0, sizeof v->{});",
                        current_offset, offset, named_member.name
                    )
                    .as_bytes(),
                )?;
                pretty_writer_preprocessor.write_line(b"# else")?;
                pretty_writer_i1.write_line(
                    format!(
                        "memcpy(&buf[{} + {}], &v->{}, sizeof v->{});",
                        current_offset, offset, named_member.name, named_member.name
                    )
                    .as_bytes(),
                )?;
                pretty_writer_preprocessor.write_line(b"# endif")?;
            } else if is_eventually_an_atom_or_enum {
                pretty_writer_i1.write_line(
                    format!("store_{}(buf, v->{});", fn_name, named_member.name).as_bytes(),
                )?;
            } else {
                pretty_writer_i1.write_line(
                    format!("store_{}(buf, &v->{});", fn_name, named_member.name).as_bytes(),
                )?;
            }
        }
        pretty_writer_preprocessor.write_line(b"#endif")?;
    }
    pretty_writer.write_line(b"}")?.eob()?;

    // --- load_*()

    pretty_writer
        .write_line(
            format!(
                "static inline void load_{}(struct {} *v_p, const unsigned char buf[static {}])",
                fn_name, data_type_entry.name, root_macro_size_name
            )
            .as_bytes(),
        )?
        .write_line(b"{")?;
    pretty_writer_i1.write_line(
        format!(
            "_Static_assert({} == {}, \"unexpected size\");",
            struct_macro_size_name, cached.type_size
        )
        .as_bytes(),
    )?;
    if !is_reference_alignment_compatible {
        pretty_writer_preprocessor.write_line(b"#ifdef ___REFERENCE_COMPATIBLE_ENCODING")?;
    }
    pretty_writer_i1.write_line(
        format!(
            "memcpy(v_p, &buf[{}], {});",
            current_offset, cached.type_size
        )
        .as_bytes(),
    )?;
    if !is_reference_alignment_compatible {
        pretty_writer_preprocessor.write_line(b"#else")?;
        for (i, named_member) in named_members.iter().enumerate() {
            let offset = cache
                .load_type(data_type_entry.id)
                .unwrap()
                .load_member(i)
                .unwrap()
                .offset;
            let hierarchy = hierarchy.push(named_member.name.to_string(), offset);
            let fn_name = hierarchy.fn_name();
            let type_ = &named_member.type_;
            match type_ {
                DataTypeRef::Atom(_) | DataTypeRef::Defined(_) => {
                    pretty_writer_i1.write_line(
                        format!("load_{}(&v_p->{}, buf);", fn_name, named_member.name).as_bytes(),
                    )?;
                }
                DataTypeRef::Ptr(_) => {
                    pretty_writer_i1.write_line(
                        format!(
                            "memcpy(&v_p->{}, &buf[{} + {}], sizeof v_p->{});",
                            named_member.name, current_offset, offset, named_member.name
                        )
                        .as_bytes(),
                    )?;
                }
            }
        }
        pretty_writer_preprocessor.write_line(b"#endif")?;
    }
    pretty_writer.write_line(b"}")?.eob()?;

    Ok(())
}
