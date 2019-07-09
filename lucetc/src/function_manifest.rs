use crate::traps::trap_sym_for_func;
use byteorder::{LittleEndian, WriteBytesExt};
use faerie::{Artifact, Decl, Link};
use failure::{Error, ResultExt};
use lucet_module_data::FunctionSpec;
use std::io::Cursor;
use std::mem::size_of;
use target_lexicon::BinaryFormat;

pub const FUNCTION_MANIFEST_SYM: &str = "lucet_function_manifest";

fn write_relocated_slice(
    obj: &mut Artifact,
    buf: &mut Cursor<Vec<u8>>,
    from: &str,
    to: Option<&str>,
    len: u64,
) -> Result<(), Error> {
    match (to, len) {
        (Some(to), 0) => {
            // This is an imported slice of unknown size
            let absolute_reloc = match obj.target.binary_format {
                BinaryFormat::Elf => faerie::artifact::Reloc::Raw {
                    reloc: goblin::elf::reloc::R_X86_64_64,
                    addend: 0,
                },
                BinaryFormat::Macho => faerie::artifact::Reloc::Raw {
                    reloc: goblin::mach::relocation::X86_64_RELOC_UNSIGNED as u32,
                    addend: 0,
                },
                _ => panic!("Unsupported target format!"),
            };

            obj.link_with(
                Link {
                    from,
                    to,
                    at: buf.position(),
                },
                absolute_reloc,
            )
            .context(format!("linking {} into function manifest", to))?;
        }
        (Some(to), _len) => {
            // This is a local buffer of known size
            obj.link(Link {
                from, // the data at `from` + `at` (eg. FUNCTION_MANIFEST_SYM)
                to,   // is a reference to `to`    (eg. fn_name)
                at: buf.position(),
            })
            .context(format!("linking {} into function manifest", to))?;
        }
        (None, len) => {
            // There's actually no relocation to add, because there's no slice to put here.
            //
            // Since there's no slice, its length must be zero.
            assert!(
                len == 0,
                "Invalid slice: no data, but there are more than zero bytes of it"
            );
        }
    }

    buf.write_u64::<LittleEndian>(0).unwrap();
    buf.write_u64::<LittleEndian>(len).unwrap();

    Ok(())
}

///
/// Writes a manifest of functions, with relocations, to the artifact.
///
pub fn write_function_manifest(
    functions: &[(String, FunctionSpec)],
    obj: &mut Artifact,
) -> Result<(), Error> {
    obj.declare(FUNCTION_MANIFEST_SYM, Decl::data())
        .context(format!("declaring {}", FUNCTION_MANIFEST_SYM))?;

    let mut manifest_buf: Cursor<Vec<u8>> = Cursor::new(Vec::with_capacity(
        functions.len() * size_of::<FunctionSpec>(),
    ));

    for (fn_name, fn_spec) in functions.iter() {
        /*
         * This has implicit knowledge of the layout of `FunctionSpec`!
         *
         * Each iteraction writes out bytes with relocations that will
         * result in data forming a valid FunctionSpec when this is loaded.
         *
         * Because the addresses don't exist until relocations are applied
         * when the artifact is loaded, we can't just populate the fields
         * and transmute, unfortunately.
         */
        // Writes a (ptr, len) pair with relocation for code
        write_relocated_slice(
            obj,
            &mut manifest_buf,
            FUNCTION_MANIFEST_SYM,
            Some(fn_name),
            fn_spec.code_len() as u64,
        )?;
        // Writes a (ptr, len) pair with relocation for this function's trap table
        let trap_sym = trap_sym_for_func(fn_name);
        write_relocated_slice(
            obj,
            &mut manifest_buf,
            FUNCTION_MANIFEST_SYM,
            if fn_spec.traps_len() > 0 {
                Some(&trap_sym)
            } else {
                None
            },
            fn_spec.traps_len() as u64,
        )?;
    }

    obj.define(FUNCTION_MANIFEST_SYM, manifest_buf.into_inner())
        .context(format!("defining {}", FUNCTION_MANIFEST_SYM))?;

    Ok(())
}
