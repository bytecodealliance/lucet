use crate::error::LucetcErrorKind;
use crate::name::Name;
use crate::output::FUNCTION_MANIFEST_SYM;
use crate::pointer::NATIVE_POINTER_SIZE;
use crate::stack_probe::{STACK_PROBE_BINARY, STACK_PROBE_SYM};
use crate::table::{TABLE_REF_SIZE, TABLE_SYM};
use crate::traps::{translate_trapcode, trap_sym_for_func};
use lucet_module::{FunctionSpec, TrapSite, LUCET_MODULE_SYM, MODULE_DATA_SYM};

use byteorder::{LittleEndian, WriteBytesExt};
use cranelift_codegen::{binemit::TrapSink, ir};
use cranelift_faerie::{
    traps::{FaerieTrapManifest, FaerieTrapSink},
    FaerieProduct,
};
use faerie::{Artifact, Decl, Link};
use failure::{format_err, Error, ResultExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::Cursor;
use std::path::Path;
use target_lexicon::BinaryFormat;

pub struct FaerieFile {
    obj: Artifact,
}
impl FaerieFile {
    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let _ = path.as_ref().file_name().ok_or(format_err!(
            "path {:?} needs to have filename",
            path.as_ref()
        ));
        let file = File::create(path)?;
        self.obj.write(file)?;
        Ok(())
    }

    pub fn new(
        product: FaerieProduct,
        module_data_len: usize,
        mut function_manifest: Vec<(String, FunctionSpec)>,
        table_manifest: Vec<Name>,
    ) -> Result<Self, Error> {
        let mut obj = Self {
            obj: product.artifact,
        };

        let mut trap_manifest: FaerieTrapManifest = product
            .trap_manifest
            .expect("trap manifest will be present");

        obj.write_stack_probe(&mut trap_manifest, &mut function_manifest)?;

        // Now that we have trap information, we can fix up FunctionSpec entries to have
        // correct `trap_length` values
        let mut function_map: HashMap<String, u32> = HashMap::new();
        for (i, (name, _)) in function_manifest.iter().enumerate() {
            function_map.insert(name.clone(), i as u32);
        }

        for sink in trap_manifest.sinks.iter() {
            if let Some(idx) = function_map.get(&sink.name) {
                let (_, fn_spec) = &mut function_manifest
                    .get_mut(*idx as usize)
                    .expect("index is valid");

                std::mem::replace::<FunctionSpec>(
                    fn_spec,
                    FunctionSpec::new(0, fn_spec.code_len(), 0, sink.sites.len() as u64),
                );
            } else {
                Err(format_err!("Inconsistent state: trap records present for function {} but the function does not exist?", sink.name))
                    .context(LucetcErrorKind::TranslatingModule)?;
            }
        }

        obj.write_trap_tables(trap_manifest)?;
        obj.write_function_manifest(function_manifest.as_slice())?;
        obj.link_tables(table_manifest.as_slice())?;

        // And now write out the actual structure tying together all the data in this module.
        obj.write_module(
            module_data_len,
            table_manifest.len(),
            function_manifest.len(),
        )?;

        Ok(obj)
    }

    fn write_stack_probe(
        &mut self,
        traps: &mut FaerieTrapManifest,
        function_manifest: &mut Vec<(String, FunctionSpec)>,
    ) -> Result<(), Error> {
        self.obj.declare_with(
            STACK_PROBE_SYM,
            Decl::function(),
            STACK_PROBE_BINARY.to_vec(),
        )?;

        {
            let mut stack_probe_trap_sink =
                FaerieTrapSink::new(STACK_PROBE_SYM, STACK_PROBE_BINARY.len() as u32);
            stack_probe_trap_sink.trap(
                10, /* test %rsp,0x8(%rsp) */
                ir::SourceLoc::default(),
                ir::TrapCode::StackOverflow,
            );
            stack_probe_trap_sink.trap(
                34, /* test %rsp,0x8(%rsp) */
                ir::SourceLoc::default(),
                ir::TrapCode::StackOverflow,
            );
            traps.add_sink(stack_probe_trap_sink);
        }

        // the stack probe machine code never exists as clif, and as a result never exists a
        // cranelift-compiled function. As a result, the declared length of the stack probe's
        // "code" is 0. This is incorrect, and must be fixed up before writing out the function
        // manifest.

        // because the stack probe is the last declared function...
        let last_idx = function_manifest.len() - 1;
        let stack_probe_entry = function_manifest
            .get_mut(last_idx)
            .expect("function manifest has entries");
        debug_assert!(stack_probe_entry.0 == STACK_PROBE_SYM);
        debug_assert!(stack_probe_entry.1.code_len() == 0);
        std::mem::swap(
            &mut stack_probe_entry.1,
            &mut FunctionSpec::new(
                0, // there is no real address for the function until written to an object file
                STACK_PROBE_BINARY.len() as u32,
                0,
                0, // fix up this FunctionSpec with trap info like any other
            ),
        );

        Ok(())
    }

    fn write_module(
        &mut self,
        module_data_len: usize,
        table_manifest_len: usize,
        function_manifest_len: usize,
    ) -> Result<(), Error> {
        let mut native_data = Cursor::new(Vec::with_capacity(NATIVE_POINTER_SIZE * 4));
        self.obj
            .declare(LUCET_MODULE_SYM, Decl::data().global())
            .context(format!("declaring {}", LUCET_MODULE_SYM))?;

        self.write_relocated_slice(
            &mut native_data,
            LUCET_MODULE_SYM,
            Some(MODULE_DATA_SYM),
            module_data_len as u64,
        )?;
        self.write_relocated_slice(
            &mut native_data,
            LUCET_MODULE_SYM,
            Some(TABLE_SYM),
            table_manifest_len as u64,
        )?;
        self.write_relocated_slice(
            &mut native_data,
            LUCET_MODULE_SYM,
            Some(FUNCTION_MANIFEST_SYM),
            function_manifest_len as u64,
        )?;

        self.obj
            .define(LUCET_MODULE_SYM, native_data.into_inner())
            .context(format!("defining {}", LUCET_MODULE_SYM))?;

        Ok(())
    }

    fn write_relocated_slice(
        &mut self,
        buf: &mut Cursor<Vec<u8>>,
        from: &str,
        to: Option<&str>,
        len: u64,
    ) -> Result<(), Error> {
        match (to, len) {
            (Some(to), 0) => {
                // This is an imported slice of unknown size
                let absolute_reloc = match self.obj.target.binary_format {
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

                self.obj
                    .link_with(
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
                self.obj
                    .link(Link {
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
        &mut self,
        functions: &[(String, FunctionSpec)],
    ) -> Result<(), Error> {
        self.obj
            .declare(FUNCTION_MANIFEST_SYM, Decl::data())
            .context(format!("declaring {}", FUNCTION_MANIFEST_SYM))?;

        let mut manifest_buf: Cursor<Vec<u8>> = Cursor::new(Vec::with_capacity(
            functions.len() * std::mem::size_of::<FunctionSpec>(),
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
            self.write_relocated_slice(
                &mut manifest_buf,
                FUNCTION_MANIFEST_SYM,
                Some(fn_name),
                fn_spec.code_len() as u64,
            )?;
            // Writes a (ptr, len) pair with relocation for this function's trap table
            let trap_sym = trap_sym_for_func(fn_name);
            self.write_relocated_slice(
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

        self.obj
            .define(FUNCTION_MANIFEST_SYM, manifest_buf.into_inner())
            .context(format!("defining {}", FUNCTION_MANIFEST_SYM))?;

        Ok(())
    }

    fn write_trap_tables(&mut self, manifest: FaerieTrapManifest) -> Result<(), Error> {
        for sink in manifest.sinks.iter() {
            let func_sym = &sink.name;
            let trap_sym = trap_sym_for_func(func_sym);

            self.obj
                .declare(&trap_sym, Decl::data())
                .context(format!("declaring {}", &trap_sym))?;

            // write the actual function-level trap table
            let traps: Vec<TrapSite> = sink
                .sites
                .iter()
                .map(|site| TrapSite {
                    offset: site.offset,
                    code: translate_trapcode(site.code),
                })
                .collect();

            let trap_site_bytes = unsafe {
                std::slice::from_raw_parts(
                    traps.as_ptr() as *const u8,
                    traps.len() * std::mem::size_of::<TrapSite>(),
                )
            };

            // and write the function trap table into the object
            self.obj
                .define(&trap_sym, trap_site_bytes.to_vec())
                .context(format!("defining {}", &trap_sym))?;
        }

        Ok(())
    }

    fn link_tables(&mut self, tables: &[Name]) -> Result<(), Error> {
        for (idx, table) in tables.iter().enumerate() {
            self.obj
                .link(Link {
                    from: TABLE_SYM,
                    to: table.symbol(),
                    at: (TABLE_REF_SIZE * idx) as u64,
                })
                .context(LucetcErrorKind::Table)?;
        }
        Ok(())
    }
}
