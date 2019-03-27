use crate::compiler::Compiler;
use crate::program::table::{TableDef, TableElem};
use byteorder::{LittleEndian, WriteBytesExt};
use cranelift_module::DataContext;
use failure::{format_err, Error, ResultExt};
use std::io::Cursor;

pub fn compile_table<'p>(compiler: &mut Compiler<'p>, table: &TableDef) -> Result<(), Error> {
    // Indirect calls are performed by looking up the callee function and type in a table that
    // is present in the same object file.
    // The table is an array of pairs of (type index, function pointer). Both elements in the
    // pair are the size of a pointer.
    // This function creates that table as a section in the object.

    // For readability:
    let ptr_size = 8;

    let mut table_data = Cursor::new(Vec::with_capacity(table.elements().len() * 2 * ptr_size));
    let putelem =
        { |table: &mut Cursor<Vec<u8>>, elem: u64| table.write_u64::<LittleEndian>(elem).unwrap() };
    let mut table_ctx = DataContext::new();

    // table.elems is a vector that gives every entry of the table, either specifying the
    // wasm function index or that no element was given for that table entry.
    for table_elem in table.elements() {
        match table_elem {
            &TableElem::FunctionIx(ref func_index) => {
                // Note: this is the only place we validate that the table entry points to a valid
                // function. If this is ever removed, make sure this check happens elsewhere.
                let func = compiler
                    .prog
                    .get_function(*func_index)
                    .context("function index for table element")?;
                let sig_ix = func
                    .signature_index()
                    .ok_or(format_err!("table function should have a signature index"))?;

                // First element in row is the SignatureIndex for the function
                putelem(&mut table_data, sig_ix as u64);

                // Second element in row is the pointer to the function. The Reloc is doing the work
                // here. We put a 0 in the table data itself to be overwritten at link time.
                let funcname = compiler.get_function(func)?;
                let funcref = table_ctx.import_function(funcname.into());
                let position = table_data.position();
                assert!(position < <u32>::max_value() as u64);
                table_ctx.write_function_addr(position as u32, funcref);
                putelem(&mut table_data, 0);
            }
            &TableElem::Empty => {
                // First element is the signature index. These will always be 32 bits in wasm, so
                // u64::max will always be out of bounds.
                putelem(&mut table_data, <u64>::max_value());
                // Second element is the function pointer. No relocation here, it will always be
                // null.
                putelem(&mut table_data, 0);
            }
        }
    }
    table_ctx.define(table_data.into_inner().into_boxed_slice());
    let table_id = compiler
        .get_table(table)?
        .as_dataid()
        .expect("tables are data");
    compiler.module.define_data(table_id, &table_ctx)?;
    Ok(())
}
