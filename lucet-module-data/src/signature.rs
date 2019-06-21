use crate::error::Error::{self, IOError, ModuleSignatureError};
use crate::ModuleData;
use byteorder::{ByteOrder, LittleEndian};
pub use minisign::PublicKey;
use minisign::{SignatureBones, SignatureBox};
use object::*;
use std::fs::{File, OpenOptions};
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;

pub struct ModuleSignature;

impl ModuleSignature {
    pub fn verify<P: AsRef<Path>>(
        so_path: P,
        pk: &PublicKey,
        module_data: &ModuleData,
    ) -> Result<(), Error> {
        let signature_box: SignatureBox =
            SignatureBones::from_bytes(&module_data.get_module_signature())
                .map_err(|e| ModuleSignatureError(e))?
                .into();

        let mut raw_module_and_data =
            RawModuleAndData::from_file(&so_path).map_err(|e| IOError(e))?;
        let cleared_module_data_bin =
            ModuleData::clear_module_signature(raw_module_and_data.module_data_bin())?;
        raw_module_and_data.patch_module_data(&cleared_module_data_bin);

        minisign::verify(
            &pk,
            &signature_box,
            Cursor::new(&raw_module_and_data.obj_bin),
            true,
            false,
        )
        .map_err(|e| ModuleSignatureError(e))
    }
}

struct SymbolData {
    offset: usize,
    len: usize,
}

pub struct RawModuleAndData {
    pub obj_bin: Vec<u8>,
    pub module_data_offset: usize,
    pub module_data_len: usize,
}

impl RawModuleAndData {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, io::Error> {
        let mut obj_bin: Vec<u8> = Vec::new();
        File::open(&path)?.read_to_end(&mut obj_bin)?;

        let module_data_symbol_data = Self::symbol_data(&obj_bin, "lucet_module_data", true)?
            .ok_or(io::Error::new(
                io::ErrorKind::InvalidInput,
                "module data not found",
            ))?;
        let module_data_len_symbol_data =
            Self::symbol_data(&obj_bin, "lucet_module_data_len", true)?.ok_or(io::Error::new(
                io::ErrorKind::InvalidInput,
                "module length not found",
            ))?;
        assert_eq!(module_data_len_symbol_data.len, 4);
        assert_eq!(
            LittleEndian::read_u32(&obj_bin[module_data_len_symbol_data.offset..]) as usize,
            module_data_symbol_data.len
        );
        Ok(RawModuleAndData {
            obj_bin,
            module_data_offset: module_data_symbol_data.offset,
            module_data_len: module_data_symbol_data.len,
        })
    }

    pub fn module_data_bin(&self) -> &[u8] {
        &self.obj_bin[self.module_data_offset as usize
            ..self.module_data_offset as usize + self.module_data_len]
    }

    pub fn module_data_bin_mut(&mut self) -> &mut [u8] {
        &mut self.obj_bin[self.module_data_offset as usize
            ..self.module_data_offset as usize + self.module_data_len]
    }

    pub fn patch_module_data(&mut self, module_data_bin: &[u8]) {
        self.module_data_bin_mut().copy_from_slice(&module_data_bin);
    }

    pub fn write_patched_module_data<P: AsRef<Path>>(
        &self,
        path: P,
        patched_module_data_bin: &[u8],
    ) -> Result<(), io::Error> {
        let mut fp = OpenOptions::new()
            .write(true)
            .create_new(false)
            .open(&path)?;
        fp.seek(SeekFrom::Start(self.module_data_offset as u64))?;
        fp.write_all(&patched_module_data_bin)?;
        Ok(())
    }

    // Retrieving the offset of a symbol is not supported by the object crate.
    // In Mach-O, actual file offsets are encoded, whereas Elf encodes virtual
    // addresses, requiring extra steps to retrieve the section, its base
    // address as well as the section offset.

    // Elf
    #[cfg(all(target_family = "unix", not(target_os = "macos")))]
    fn symbol_data(
        obj_bin: &[u8],
        symbol_name: &str,
        _mangle: bool,
    ) -> Result<Option<SymbolData>, io::Error> {
        let obj = object::ElfFile::parse(obj_bin)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let symbol_map = obj.symbol_map();
        for symbol in symbol_map.symbols() {
            let kind = symbol.kind();
            if kind != SymbolKind::Data {
                continue;
            }
            if symbol.name() != Some(symbol_name) {
                continue;
            }
            let section_index = match symbol.section_index() {
                Some(section_index) => section_index,
                None => continue,
            };
            let section = &obj.elf().section_headers[section_index.0];
            let offset = (symbol.address() - section.sh_addr + section.sh_offset) as usize;
            let len = symbol.size() as usize;
            return Ok(Some(SymbolData { offset, len }));
        }
        Ok(None)
    }

    // Mach-O
    #[cfg(target_os = "macos")]
    fn symbol_data(
        obj_bin: &[u8],
        symbol_name: &str,
        mangle: bool,
    ) -> Result<Option<SymbolData>, io::Error> {
        let obj = object::File::parse(obj_bin)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let symbol_map = obj.symbol_map();
        let mangled_symbol_name = format!("_{}", symbol_name);
        let symbol_name = if mangle {
            &mangled_symbol_name
        } else {
            symbol_name
        };
        for symbol in symbol_map.symbols() {
            let kind = symbol.kind();
            if kind != SymbolKind::Data && kind != SymbolKind::Unknown {
                continue;
            }
            if symbol.name() != Some(symbol_name) {
                continue;
            }
            let offset = symbol.address() as usize;
            let len = symbol.size() as usize;
            return Ok(Some(SymbolData { offset, len }));
        }
        Ok(None)
    }
}
