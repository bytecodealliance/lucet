use crate::c::catom::CAtom;
use crate::c::macros;
use crate::c::CGenerator;
use crate::errors::IDLError;
use crate::generator::Hierarchy;
use crate::module::Module;
use crate::pretty_writer::PrettyWriter;
use crate::types::AtomType;
use std::io::prelude::*;

pub fn generate<W: Write>(
    cgenerator: &mut CGenerator,
    _module: &Module,
    pretty_writer: &mut PrettyWriter<W>,
    atom_type: AtomType,
    hierarchy: &Hierarchy,
) -> Result<(), IDLError> {
    let fn_name = hierarchy.fn_name();
    let root_name = hierarchy.root_name();
    let root_macro_size_name = macros::macro_for("BYTES", &root_name);
    let current_offset = hierarchy.current_offset();
    let catom = CAtom::from(atom_type);
    let mut pretty_writer_i1 = pretty_writer.new_block();
    let mut preprocessor_writer = PrettyWriter::new_from_writer(pretty_writer);
    let uses_reference_target_endianness = cgenerator
        .target
        .uses_reference_target_endianness_for_atom_type(atom_type);

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
    pretty_writer.write_line(b"}")?.eob()?;

    Ok(())
}
