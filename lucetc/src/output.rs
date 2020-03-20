use crate::error::Error;
use crate::name::Name;
use crate::table::TABLE_SYM;
use byteorder::{LittleEndian, WriteBytesExt};
use cranelift_codegen::{ir, isa};
use cranelift_faerie::FaerieProduct;
use faerie::{Artifact, Decl, Link};
use lucet_module::{
    SerializedModule, VersionInfo, LUCET_MODULE_SYM, MODULE_DATA_SYM,
};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Write};
use std::path::Path;
use target_lexicon::BinaryFormat;

pub(crate) const FUNCTION_MANIFEST_SYM: &str = "lucet_function_manifest";

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
            write_function(&mut buffer, func, &Some(self.isa.as_ref()).into()).map_err(|e| {
                let message = format!("{:?}", n);
                Error::OutputFunction(e, message)
            })?
        }
        let mut file = File::create(path)?;
        file.write_all(buffer.as_bytes())?;
        Ok(())
    }
}

pub struct ObjectFile {
    artifact: Artifact,
}
impl ObjectFile {
    pub fn new(
        product: FaerieProduct,
        module_data_len: usize,
        function_manifest_len: usize,
        table_manifest_len: usize,
    ) -> Result<Self, Error> {
        let mut obj = Self {
            artifact: product.artifact,
        };

        // And now write out the actual structure tying together all the data in this module.
        write_module(
            module_data_len,
            table_manifest_len,
            function_manifest_len,
            &mut obj.artifact,
        )?;

        Ok(obj)
    }

    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let _ = path.as_ref().file_name().ok_or(|| {
            let message = format!("Path must be filename {:?}", path.as_ref());
            Error::Input(message);
        });
        let file = File::create(path)?;
        self.artifact
            .write(file)
            .map_err(|source| Error::FaerieArtifact(source, "Write error".to_owned()))?;
        Ok(())
    }
}

fn write_module(
    module_data_len: usize,
    table_manifest_len: usize,
    function_manifest_len: usize,
    obj: &mut Artifact,
) -> Result<(), Error> {
    let mut native_data = Cursor::new(Vec::with_capacity(std::mem::size_of::<SerializedModule>()));
    obj.declare(LUCET_MODULE_SYM, Decl::data().global())
        .map_err(|source| {
            let message = format!("Manifest error declaring {}", FUNCTION_MANIFEST_SYM);
            Error::FaerieArtifact(source, message)
        })?;

    let version =
        VersionInfo::current(include_str!(concat!(env!("OUT_DIR"), "/commit_hash")).as_bytes());

    version.write_to(&mut native_data)?;

    write_relocated_slice(
        obj,
        &mut native_data,
        LUCET_MODULE_SYM,
        Some(MODULE_DATA_SYM),
        module_data_len as u64,
    )?;
    write_relocated_slice(
        obj,
        &mut native_data,
        LUCET_MODULE_SYM,
        Some(TABLE_SYM),
        table_manifest_len as u64,
    )?;
    write_relocated_slice(
        obj,
        &mut native_data,
        LUCET_MODULE_SYM,
        Some(FUNCTION_MANIFEST_SYM),
        function_manifest_len as u64,
    )?;

    obj.define(LUCET_MODULE_SYM, native_data.into_inner())
        .map_err(|source| {
            let message = format!("Manifest error defining {}", FUNCTION_MANIFEST_SYM);
            Error::FaerieArtifact(source, message)
        })?;

    Ok(())
}

pub(crate) fn write_relocated_slice(
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
            .map_err(|source| {
                let message = format!("Manifest error linking {}", to);
                Error::FaerieArtifact(source, message)
            })?;
        }
        (Some(to), _len) => {
            // This is a local buffer of known size
            obj.link(Link {
                from, // the data at `from` + `at` (eg. FUNCTION_MANIFEST_SYM)
                to,   // is a reference to `to`    (eg. fn_name)
                at: buf.position(),
            })
            .map_err(|source| {
                let message = format!("Manifest error linking {}", to);
                Error::FaerieArtifact(source, message)
            })?;
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
