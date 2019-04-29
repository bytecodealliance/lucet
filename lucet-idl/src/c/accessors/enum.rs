use crate::c::catom::CAtom;
use crate::c::macros;
use crate::c::CGenerator;
use crate::cache::Cache;
use crate::errors::IDLError;
use crate::generator::Hierarchy;
use crate::module::{DataType, DataTypeEntry, Module};
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
    let (named_members, _attrs) = if let DataType::Enum {
        members: named_members,
        attrs,
    } = &data_type_entry.data_type
    {
        (named_members, attrs)
    } else {
        unreachable!()
    };
    let root_name = hierarchy.root_name();
    let fn_name = hierarchy.fn_name();
    let root_macro_size_name = macros::macro_for("BYTES", &root_name);
    let current_offset = hierarchy.current_offset();

    let catom = CAtom::enum_();
    let mut preprocessor_writer = PrettyWriter::new_from_writer(pretty_writer);
    let uses_reference_target_endianness = cgenerator
        .target
        .uses_reference_target_endianness_for_atom_type(catom.as_atom_type().unwrap());

    pretty_writer.eob()?;

    if hierarchy.depth() > 1 {
        pretty_writer.write_line(
            format!(
                "// Accessors for the `{}` enumeration in `{}`",
                data_type_entry.name,
                hierarchy.idl_name()
            )
            .as_bytes(),
        )?;
    } else {
        pretty_writer.write_line(
            format!(
                "// Platform-independent accessors for the `{}` enumeration",
                data_type_entry.name
            )
            .as_bytes(),
        )?;
    }
    pretty_writer.eob()?;

    let mut pretty_writer_i1 = pretty_writer.new_block();

    // --- store_*()

    pretty_writer
        .write_line(
            format!(
                "static inline void store_{}(unsigned char buf[static {}], const {} v)",
                fn_name, root_macro_size_name, catom.native_type_name
            )
            .as_bytes(),
        )?
        .write_line(b"{")?;
    pretty_writer_i1.write_line(
        format!(
            "_Static_assert({} >= {} + {}, \"unexpected size\");",
            root_macro_size_name, current_offset, catom.native_type_size
        )
        .as_bytes(),
    )?;
    pretty_writer_i1
        .write_line(format!("assert(v >= 0 && v < {});", named_members.len()).as_ref())?
        .eob()?;
    if !uses_reference_target_endianness {
        preprocessor_writer.write_line(b"#ifdef ___REFERENCE_COMPATIBLE_ENCODING")?;
    }
    pretty_writer_i1.write_line(
        format!(
            "memcpy(&buf[{}], &v, {});",
            current_offset, catom.native_type_size
        )
        .as_bytes(),
    )?;

    if !uses_reference_target_endianness {
        preprocessor_writer.write_line(b"#else")?;
        pretty_writer_i1
            .write_line(
                format!(
                    "{} t = {};",
                    catom.native_type_name,
                    catom.little_endian(cgenerator.target, "v")
                )
                .as_bytes(),
            )?
            .write_line(
                format!(
                    "memcpy(&buf[{}], &t, {});",
                    current_offset, catom.native_type_size
                )
                .as_bytes(),
            )?;
        preprocessor_writer.write_line(b"#endif")?;
    }
    pretty_writer.write_line(b"}")?.eob()?;

    // --- load_*()

    pretty_writer
        .write_line(
            format!(
                "static inline void load_{}({} *v_p, const unsigned char buf[static {}])",
                fn_name, catom.native_type_name, root_macro_size_name
            )
            .as_bytes(),
        )?
        .write_line(b"{")?;
    pretty_writer_i1.write_line(
        format!(
            "_Static_assert({} >= {} + {}, \"unexpected size\");",
            root_macro_size_name, current_offset, catom.native_type_size
        )
        .as_bytes(),
    )?;
    if !uses_reference_target_endianness {
        preprocessor_writer.write_line(b"#ifdef ___REFERENCE_COMPATIBLE_ENCODING")?;
    }
    pretty_writer_i1.write_line(
        format!(
            "memcpy(v_p, &buf[{}], {});",
            current_offset, catom.native_type_size
        )
        .as_bytes(),
    )?;
    if !uses_reference_target_endianness {
        preprocessor_writer.write_line(b"#else")?;
        pretty_writer_i1
            .write_line(format!("{} t = 0;", catom.native_type_name).as_bytes())?
            .write_line(
                format!(
                    "memcpy(&t, &buf[{}], {});",
                    current_offset, catom.native_type_size
                )
                .as_bytes(),
            )?
            .write_line(
                format!("*v_p = {};", catom.little_endian(cgenerator.target, "t")).as_bytes(),
            )?;
        preprocessor_writer.write_line(b"#endif")?;
    }
    pretty_writer_i1
        .write_line(format!("assert(*v_p >= 0 && *v_p < {});", named_members.len()).as_ref())?;
    pretty_writer.write_line(b"}")?.eob()?;

    Ok(())
}
