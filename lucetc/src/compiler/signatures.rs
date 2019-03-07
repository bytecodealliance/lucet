use crate::compiler::Compiler;
use crate::error::LucetcErrorKind;
use byteorder::{LittleEndian, WriteBytesExt};
use cranelift_module::{DataContext, Linkage};
use failure::Error;
use parity_wasm::elements::{Serialize, Type};
use std::io::Cursor;
use std::{mem::size_of, u16};

/// Stores the table of function signatures into `lucet_signatures`
/// Signatures are a list of signatures individually encoded as
/// <signature length: u32> || <len(return_values): u16> || <return_values: [u8]>
///                         || <len(params): u16> || <params: [u8]>
fn compile_signatures(compiler: &mut Compiler) -> Result<(), Error> {
    let types = compiler
        .prog
        .module()
        .type_section()
        .ok_or_else(|| LucetcErrorKind::Other("no types in this module".to_owned()))?
        .types();
    let signatures_serialized_len = types
        .iter()
        .map(|type_| {
            let ftype = match type_ {
                &Type::Function(ref ftype) => ftype,
            };
            let return_values_len = match ftype.return_type() {
                None => 0,
                Some(_) => 1,
            };
            size_of::<u32>()
                + size_of::<u16>()
                + return_values_len
                + size_of::<u16>()
                + ftype.params().len()
        })
        .sum();
    let mut signatures_serialized: Cursor<Vec<u8>> =
        Cursor::new(Vec::with_capacity(signatures_serialized_len));
    for type_ in types {
        let ftype = match type_ {
            &Type::Function(ref ftype) => ftype,
        };
        let params = ftype.params();
        let return_values_len = match ftype.return_type() {
            None => 0,
            Some(_) => 1,
        };
        let signature_len = size_of::<u16>() + return_values_len + size_of::<u16>() + params.len();
        signatures_serialized.write_u32::<LittleEndian>(signature_len as u32)?;
        signatures_serialized.write_u16::<LittleEndian>(return_values_len as u16)?;
        if let Some(type_) = ftype.return_type() {
            type_.serialize(&mut signatures_serialized)?
        }
        signatures_serialized.write_u16::<LittleEndian>(params.len() as u16)?;
        for param in params {
            param.serialize(&mut signatures_serialized)?;
        }
    }
    let signatures_serialized = signatures_serialized.into_inner();

    let mut serialized_len: Vec<u8> = Vec::new();
    serialized_len
        .write_u32::<LittleEndian>(signatures_serialized.len() as u32)
        .unwrap();
    let mut signatures_len_ctx = DataContext::new();
    signatures_len_ctx.define(serialized_len.into_boxed_slice());
    let signatures_len_decl =
        compiler
            .module
            .declare_data("lucet_signatures_len", Linkage::Export, false)?;
    compiler
        .module
        .define_data(signatures_len_decl, &signatures_len_ctx)?;

    let mut signatures_ctx = DataContext::new();
    signatures_ctx.define(signatures_serialized.into_boxed_slice());
    let signatures_decl =
        compiler
            .module
            .declare_data("lucet_signatures", Linkage::Export, true)?;
    compiler
        .module
        .define_data(signatures_decl, &signatures_ctx)?;

    Ok(())
}

/// Stores the function->signature_index map for the defined functions into `lucet_defined_functions`
/// This is just a `u32` array of signature indices
fn compile_function_signatures_map(compiler: &mut Compiler) -> Result<(), Error> {
    let defined_functions = compiler.prog.defined_functions();
    let defined_functions_serialized_len = size_of::<u32>() * defined_functions.len();
    let mut defined_functions_serialized: Cursor<Vec<u8>> =
        Cursor::new(Vec::with_capacity(defined_functions_serialized_len));
    for defined_function in defined_functions {
        let sig_index = defined_function.signature_index();
        defined_functions_serialized.write_u32::<LittleEndian>(sig_index)?;
    }
    let defined_functions_serialized = defined_functions_serialized.into_inner();

    let mut serialized_len: Vec<u8> = Vec::new();
    serialized_len
        .write_u32::<LittleEndian>(defined_functions_serialized.len() as u32)
        .unwrap();
    let mut signatures_len_ctx = DataContext::new();
    signatures_len_ctx.define(serialized_len.into_boxed_slice());
    let signatures_len_decl =
        compiler
            .module
            .declare_data("lucet_defined_functions_len", Linkage::Export, false)?;
    compiler
        .module
        .define_data(signatures_len_decl, &signatures_len_ctx)?;

    let mut signatures_ctx = DataContext::new();
    signatures_ctx.define(defined_functions_serialized.into_boxed_slice());
    let signatures_decl =
        compiler
            .module
            .declare_data("lucet_defined_functions", Linkage::Export, true)?;
    compiler
        .module
        .define_data(signatures_decl, &signatures_ctx)?;

    Ok(())
}

/// Builds two table, the function signatures and the function->signature_index map
pub fn compile_signatures_and_function_signatures_map(
    compiler: &mut Compiler,
) -> Result<(), Error> {
    compile_signatures(compiler)?;
    compile_function_signatures_map(compiler)?;
    Ok(())
}
