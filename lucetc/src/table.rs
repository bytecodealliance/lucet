use crate::decls::{ModuleDecls, TableDecl};
use crate::error::{LucetcError, LucetcErrorKind};
use crate::module::UniqueFuncIndex;
use crate::pointer::NATIVE_POINTER_SIZE;
use byteorder::{LittleEndian, WriteBytesExt};
use cranelift_codegen::entity::EntityRef;
use cranelift_module::{Backend as ClifBackend, DataContext, Module as ClifModule};
use cranelift_wasm::{TableElementType, TableIndex};
use failure::{format_err, ResultExt};
use std::io::Cursor;

/// This symbol will be used to reference the `tables` field in `Module` - a sequence of tables.
/// At the moment it will either be one or no tables, but in the future may grow.
pub const TABLE_SYM: &str = "lucet_tables";
/// This is functionally the size of `&[TableEntry]`, but defined here because it may not
/// necessarily have the same field ordering.
pub const TABLE_REF_SIZE: usize = NATIVE_POINTER_SIZE * 2;

#[derive(Debug, Clone)]
enum Elem {
    Func(UniqueFuncIndex),
    Empty,
}

fn table_elements(decl: &TableDecl<'_>) -> Result<Vec<Elem>, LucetcError> {
    match decl.table.ty {
        TableElementType::Func => Ok(()),
        _ => Err(format_err!("table with non-function elements: {:?}", decl))
            .context(LucetcErrorKind::Unsupported),
    }?;

    let mut elems = Vec::new();

    for initializer in decl.elems.iter() {
        if initializer.base.is_some() {
            Err(format_err!(
                "table elements with global index base: {:?}",
                initializer
            ))
            .context(LucetcErrorKind::Unsupported)?
        }
        let final_len = initializer.offset + initializer.elements.len();
        if final_len > elems.len() {
            elems.resize(final_len, Elem::Empty);
        }
        for (ix, func_ix) in initializer.elements.iter().enumerate() {
            elems[initializer.offset + ix] = Elem::Func(*func_ix);
        }
    }

    Ok(elems)
}

pub fn write_table_data<B: ClifBackend>(
    clif_module: &mut ClifModule<B>,
    decls: &ModuleDecls<'_>,
) -> Result<usize, LucetcError> {
    let mut table_data_ctx = DataContext::new();
    let mut tables_vec = Cursor::new(Vec::new());

    if let Ok(table_decl) = decls.get_table(TableIndex::new(0)) {
        // Indirect calls are performed by looking up the callee function and type in a table that
        // is present in the same object file.
        // The table is an array of pairs of (type index, function pointer). Both elements in the
        // pair are the size of a pointer.
        // This function creates that table as a section in the object.
        let elements = table_elements(&table_decl)?;

        let mut table_data =
            Cursor::new(Vec::with_capacity(elements.len() * 2 * NATIVE_POINTER_SIZE));
        fn putelem(table: &mut Cursor<Vec<u8>>, elem: u64) {
            table.write_u64::<LittleEndian>(elem).unwrap()
        }

        let mut table_ctx = DataContext::new();

        // table.elems is a vector that gives every entry of the table, either specifying the
        // wasm function index or that no element was given for that table entry.
        for elem in elements.iter() {
            match elem {
                Elem::Func(func_index) => {
                    // Note: this is the only place we validate that the table entry points to a valid
                    // function. If this is ever removed, make sure this check happens elsewhere.
                    let func = decls
                        .get_func(*func_index)
                        .context(LucetcErrorKind::Table)?;
                    // First element in row is the SignatureIndex for the function
                    putelem(&mut table_data, func.signature_index.as_u32() as u64);

                    // Second element in row is the pointer to the function. The Reloc is doing the work
                    // here. We put a 0 in the table data itself to be overwritten at link time.
                    let funcref = table_ctx.import_function(func.name.into());
                    let position = table_data.position();
                    assert!(position < <u32>::max_value() as u64);
                    table_ctx.write_function_addr(position as u32, funcref);
                    putelem(&mut table_data, 0);
                }
                // EMPTY:
                Elem::Empty => {
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
        let table_id = table_decl
            .contents_name
            .as_dataid()
            .expect("tables are data");
        clif_module
            .define_data(table_id, &table_ctx)
            .context(LucetcErrorKind::Table)?;

        // have to link TABLE_SYM, table_id,
        let data_id = clif_module.declare_data_in_data(table_id, &mut table_data_ctx);
        table_data_ctx.write_data_addr(tables_vec.get_ref().len() as u32, data_id, 0);
        // add space for the TABLE_SYM pointer
        tables_vec.write_u64::<LittleEndian>(0).unwrap();

        // Define the length of the table as a u64:
        tables_vec
            .write_u64::<LittleEndian>(elements.len() as u64)
            .unwrap();
    }

    let tables_len = tables_vec.get_ref().len() / TABLE_REF_SIZE;
    table_data_ctx.define(tables_vec.into_inner().into_boxed_slice());

    clif_module
        .define_data(
            decls
                .get_tables_list_name()
                .as_dataid()
                .expect("lucet_tables is declared as data"),
            &table_data_ctx,
        )
        .context(LucetcErrorKind::Table)?;
    Ok(tables_len)
}
