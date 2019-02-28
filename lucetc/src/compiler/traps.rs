use cranelift_codegen::ir;
use cranelift_faerie::traps::FaerieTrapManifest;

use byteorder::{LittleEndian, WriteBytesExt};
use faerie::{Artifact, Decl, Link};
use failure::{Error, ResultExt};
use std::io::Cursor;

pub fn write_trap_manifest(manifest: &FaerieTrapManifest, obj: &mut Artifact) -> Result<(), Error> {
    // declare traptable symbol
    let manifest_len_sym = "lucet_trap_manifest_len";
    obj.declare(&manifest_len_sym, Decl::data().global())
        .context(format!("declaring {}", &manifest_len_sym))?;

    let manifest_sym = "lucet_trap_manifest";
    obj.declare(&manifest_sym, Decl::data().global())
        .context(format!("declaring {}", &manifest_sym))?;

    let manifest_len = manifest.sinks.len();
    let mut manifest_len_buf: Vec<u8> = Vec::new();
    manifest_len_buf
        .write_u32::<LittleEndian>(manifest_len as u32)
        .unwrap();
    obj.define(&manifest_len_sym, manifest_len_buf)
        .context(format!("defining {}", &manifest_len_sym))?;

    // Manifests are serialized with the following struct elements in order:
    // { func_start: ptr, func_len: u64, traps: ptr, traps_len: u64 }
    let manifest_row_size = 8 * 4;
    let mut manifest_buf: Cursor<Vec<u8>> =
        Cursor::new(Vec::with_capacity(manifest_len * manifest_row_size));

    for sink in manifest.sinks.iter() {
        let func_sym = &sink.name;
        let trap_sym = trap_sym_for_func(func_sym);

        // declare function-level trap table
        obj.declare(&trap_sym, Decl::data().global())
            .context(format!("declaring {}", &trap_sym))?;

        // function symbol is provided via a link (abs8 relocation)
        obj.link(Link {
            from: &manifest_sym,
            to: func_sym,
            at: manifest_buf.position(),
        })
        .context("linking function sym into trap manifest")?;
        manifest_buf.write_u64::<LittleEndian>(0).unwrap();

        // write function length
        manifest_buf
            .write_u64::<LittleEndian>(sink.code_size as u64)
            .unwrap();

        // table for this function is provided via a link (abs8 relocation)
        obj.link(Link {
            from: &manifest_sym,
            to: &trap_sym,
            at: manifest_buf.position(),
        })
        .context("linking trap table into trap manifest")?;
        manifest_buf.write_u64::<LittleEndian>(0).unwrap();

        // finally, write the length of the trap table
        manifest_buf
            .write_u64::<LittleEndian>(sink.sites.len() as u64)
            .unwrap();

        // ok, now write the actual function-level trap table
        let mut traps: Vec<u8> = Vec::new();

        for site in sink.sites.iter() {
            // write offset into trap table
            traps.write_u32::<LittleEndian>(site.offset as u32).unwrap();
            // write serialized trap code into trap table
            traps
                .write_u32::<LittleEndian>(serialize_trapcode(site.code))
                .unwrap();
        }

        // and finally write the function trap table into the object
        obj.define(&trap_sym, traps)
            .context(format!("defining {}", &trap_sym))?;
    }

    obj.define(&manifest_sym, manifest_buf.into_inner())
        .context(format!("defining {}", &manifest_sym))?;

    // iterate over tables:
    //   write empty relocation thunk
    //   link from traptable symbol + thunk offset to function symbol
    //   write trapsite count
    //
    //   iterate over trapsites:
    //     write offset
    //     write trapcode

    Ok(())
}

fn trap_sym_for_func(sym: &str) -> String {
    return format!("lucet_trap_table_{}", sym);
}

// Trapcodes can be thought of as a tuple of (type, subtype). Each are
// represented as a 16-bit unsigned integer. These are packed into a u32
// wherein the type occupies the low 16 bites and the subtype takes the
// high bits.
//
// Not all types have subtypes. Currently, only the user User type has a
// subtype.
fn serialize_trapcode(code: ir::TrapCode) -> u32 {
    match code {
        ir::TrapCode::StackOverflow => 0,
        ir::TrapCode::HeapOutOfBounds => 1,
        ir::TrapCode::OutOfBounds => 2,
        ir::TrapCode::IndirectCallToNull => 3,
        ir::TrapCode::BadSignature => 4,
        ir::TrapCode::IntegerOverflow => 5,
        ir::TrapCode::IntegerDivisionByZero => 6,
        ir::TrapCode::BadConversionToInteger => 7,
        ir::TrapCode::Interrupt => 8,
        ir::TrapCode::TableOutOfBounds => 9,
        ir::TrapCode::UnreachableCodeReached => (u16::max_value() - 1) as u32, // XXX this used to be User(0)
        ir::TrapCode::User(x) => ((u16::max_value() - 1) as u32) | ((x as u32) << 16),
    }
}
