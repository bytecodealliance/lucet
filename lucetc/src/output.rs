use crate::function_manifest::{write_function_manifest, FUNCTION_MANIFEST_SYM};
use crate::name::Name;
use crate::pointer::NATIVE_POINTER_SIZE;
use crate::stack_probe;
use crate::table::TABLE_SYM;
use crate::traps::write_trap_table;
use byteorder::{LittleEndian, WriteBytesExt};
use cranelift_codegen::{ir, isa};
use cranelift_object::ObjectProduct;
use failure::{format_err, Error, ResultExt};
use lucet_module::{LUCET_MODULE_SYM, MODULE_DATA_SYM};
use object::write::{Object, Relocation, StandardSection, Symbol, SymbolId};
use object::{RelocationEncoding, RelocationKind, SymbolKind, SymbolScope};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Write};
use std::path::Path;

pub struct CraneliftFuncs {
    funcs: HashMap<Name, ir::Function>,
    isa: Box<dyn isa::TargetIsa>,
}

impl CraneliftFuncs {
    pub fn new(funcs: HashMap<Name, ir::Function>, isa: Box<dyn isa::TargetIsa>) -> Self {
        Self { funcs, isa }
    }
    /// This outputs a .clif file
    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        use cranelift_codegen::write_function;
        let mut buffer = String::new();
        for (n, func) in self.funcs.iter() {
            buffer.push_str(&format!("; {}\n", n.symbol()));
            write_function(&mut buffer, func, &Some(self.isa.as_ref()).into())
                .context(format_err!("writing func {:?}", n))?
        }
        let mut file = File::create(path)?;
        file.write_all(buffer.as_bytes())?;
        Ok(())
    }
}

pub struct ObjectFile {
    object: Object,
}
impl ObjectFile {
    pub fn new(
        mut product: ObjectProduct,
        module_data_len: usize,
        table_manifest_len: usize,
    ) -> Result<Self, Error> {
        // stack_probe::define never exists as clif, and as a result never exists a
        // cranelift-compiled function, so the traps for it won't be in the product.
        let (stack_probe_func, stack_probe_traps) =
            stack_probe::define(&mut product.object).unwrap();

        // Write trap tables for every function, and collect the details needed for
        // the function manifest: function symbol, trap symbol, and number of traps.
        let mut function_manifest: Vec<(SymbolId, Option<SymbolId>, u32)> = Vec::new();
        for (func, symbol) in product.functions.iter() {
            let symbol = symbol.unwrap();
            let traps = if symbol == stack_probe_func {
                &stack_probe_traps
            } else {
                &product.traps[func]
            };
            if !traps.is_empty() {
                let trap_sym = write_trap_table(symbol, &traps, &mut product.object)?;
                function_manifest.push((symbol, Some(trap_sym), traps.len() as u32));
            } else {
                function_manifest.push((symbol, None, 0));
            }
        }

        write_function_manifest(function_manifest.as_slice(), &mut product.object)?;

        // And now write out the actual structure tying together all the data in this module.
        write_module(
            module_data_len,
            table_manifest_len,
            function_manifest.len(),
            &mut product.object,
        )?;

        Ok(Self {
            object: product.object,
        })
    }

    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let _ = path.as_ref().file_name().ok_or(format_err!(
            "path {:?} needs to have filename",
            path.as_ref()
        ));
        let mut file = File::create(path)?;
        let bytes = self.object.write().map_err(failure::err_msg)?;
        file.write_all(&bytes)?;
        Ok(())
    }
}

fn write_module(
    module_data_len: usize,
    table_manifest_len: usize,
    function_manifest_len: usize,
    obj: &mut Object,
) -> Result<(), Error> {
    let mut native_data = Cursor::new(Vec::with_capacity(NATIVE_POINTER_SIZE * 4));
    let mut relocs = Vec::new();
    write_relocated_slice(
        &mut native_data,
        &mut relocs,
        obj.symbol_id(MODULE_DATA_SYM.as_bytes()),
        module_data_len as u64,
    );
    write_relocated_slice(
        &mut native_data,
        &mut relocs,
        obj.symbol_id(TABLE_SYM.as_bytes()),
        table_manifest_len as u64,
    );
    write_relocated_slice(
        &mut native_data,
        &mut relocs,
        obj.symbol_id(FUNCTION_MANIFEST_SYM.as_bytes()),
        function_manifest_len as u64,
    );

    let section_id = obj.section_id(StandardSection::ReadOnlyDataWithRel);
    let native_data = native_data.into_inner();
    let lucet_module_offset = obj.append_section_data(section_id, &native_data, 8);
    obj.add_symbol(Symbol {
        name: LUCET_MODULE_SYM.as_bytes().to_vec(),
        value: lucet_module_offset,
        size: native_data.len() as u64,
        kind: SymbolKind::Data,
        scope: SymbolScope::Dynamic,
        weak: false,
        section: Some(section_id),
    });

    for mut reloc in relocs.drain(..) {
        reloc.offset += lucet_module_offset;
        obj.add_relocation(section_id, reloc)
            .map_err(failure::err_msg)
            .context("relocating module")?;
    }

    Ok(())
}

pub(crate) fn write_relocated_slice(
    buf: &mut Cursor<Vec<u8>>,
    relocs: &mut Vec<Relocation>,
    to: Option<SymbolId>,
    len: u64,
) {
    if let Some(to) = to {
        relocs.push(Relocation {
            offset: buf.position(),
            size: 64,
            kind: RelocationKind::Absolute,
            encoding: RelocationEncoding::Generic,
            symbol: to,
            addend: 0,
        });
    } else {
        // There's actually no relocation to add, because there's no slice to put here.
        //
        // Since there's no slice, its length must be zero.
        assert!(
            len == 0,
            "Invalid slice: no data, but there are more than zero bytes of it"
        );
    }

    buf.write_u64::<LittleEndian>(0).unwrap();
    buf.write_u64::<LittleEndian>(len).unwrap();
}
