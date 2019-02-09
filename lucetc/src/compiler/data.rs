use crate::compiler::Compiler;
use byteorder::{LittleEndian, WriteBytesExt};
use cranelift_module::{DataContext, Linkage};
use failure::Error;

/// Formats and stores WASM data segment initializers[0] from the program to
/// symbols in the data section of obj.
///
/// - The segment initialization data is stored to the "wasm_data_segment"
///     symbol
/// - The total number of bytes stored to "wasm_data_segment" symbol is
///     stored in the "wasm_data_segment_length" symbol
///
/// The program that consumes the resulting ELF object is responsible for
/// using it to initialize WASM linear memory regions.
///
/// [0] https://webassembly.github.io/spec/syntax/modules.html#data-segments
///
/// WARNING: At present, this code
/// - Does limited validation of data segments
/// - Does not coalesce data segments
/// - Uses an implicit data format for the serialized segment data, defined
///    only in the code below
pub fn compile_data_initializers(compiler: &mut Compiler) -> Result<(), Error> {
    let mut serialized: Vec<u8> = Vec::new();

    for initializer in compiler.prog.data_initializers()? {
        // Data segment has been validated in program::data.
        // memory_index is always 0 per spec, so we dont put it in data.
        let memory_index: u32 = 0;
        serialized.write_u32::<LittleEndian>(memory_index).unwrap();
        serialized
            .write_u32::<LittleEndian>(initializer.offset)
            .unwrap();
        serialized
            .write_u32::<LittleEndian>(initializer.data.len() as u32)
            .unwrap();
        serialized.extend_from_slice(initializer.data);

        // Pad to 8 bytes: this aligns the data for architectures with 4 or
        // 8 byte word sizes (i.e. everything we are likely to support at
        // least until we replaced this with an engineered serialization
        // format)

        let pad = vec![0u8; (8 - serialized.len() % 8) % 8];
        serialized.extend(pad);
    }

    let mut serialized_len: Vec<u8> = Vec::new();
    serialized_len
        .write_u32::<LittleEndian>(serialized.len() as u32)
        .unwrap();
    let mut seg_len_ctx = DataContext::new();
    seg_len_ctx.define(serialized_len.into_boxed_slice());

    let writeable = false;
    let seg_len_decl =
        compiler
            .module
            .declare_data("wasm_data_segments_len", Linkage::Export, writeable)?;
    compiler.module.define_data(seg_len_decl, &seg_len_ctx)?;

    let mut seg_ctx = DataContext::new();
    seg_ctx.define(serialized.into_boxed_slice());
    let seg_decl =
        compiler
            .module
            .declare_data("wasm_data_segments", Linkage::Export, writeable)?;
    compiler.module.define_data(seg_decl, &seg_ctx)?;

    Ok(())
}

use std::io::Cursor;

pub fn compile_sparse_page_data(compiler: &mut Compiler) -> Result<(), Error> {
    use crate::program::data::sparse::CompiledSparseData;
    let compiled_data = CompiledSparseData::new(
        &compiler.prog.data_initializers()?,
        compiler.prog.heap_spec(),
    );
    let sparse_data = compiled_data.sparse_data();

    let mut table_ctx = DataContext::new();
    let mut table_data: Cursor<Vec<u8>> =
        Cursor::new(Vec::with_capacity(sparse_data.chunks().len() * 8 + 8));

    // The table is an array of 64-bit elements:
    //  [0] the number subsequent elements
    //  [1..] a pointer to a 4096-byte array of the contents of the page,
    //        or null if it is initialized as zero.

    table_data
        .write_u64::<LittleEndian>(sparse_data.chunks().len() as u64)
        .unwrap();
    for (dix, d) in sparse_data.chunks().iter().enumerate() {
        if let Some(vs) = d {
            // Define the 4096-byte array for the contents of the page
            let seg_decl = compiler.module.declare_data(
                &format!("guest_sparse_page_data_{}", dix),
                Linkage::Local,
                false,
            )?;
            let mut seg_ctx = DataContext::new();
            seg_ctx.define(vs.to_vec().into_boxed_slice());
            compiler.module.define_data(seg_decl, &seg_ctx)?;

            // Put a relocation to that array into the table:
            let seg_gv = compiler
                .module
                .declare_data_in_data(seg_decl, &mut table_ctx);
            table_ctx.write_data_addr(table_data.position() as u32, seg_gv, 0);
        }
        table_data.write_u64::<LittleEndian>(0)?;
    }

    table_ctx.define(table_data.into_inner().into_boxed_slice());
    let table_decl =
        compiler
            .module
            .declare_data("guest_sparse_page_data", Linkage::Export, false)?;
    compiler.module.define_data(table_decl, &table_ctx)?;

    Ok(())
}
