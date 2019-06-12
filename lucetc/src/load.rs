use crate::error::LucetcErrorKind;
use failure::*;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use wabt::wat2wasm;

pub fn read_module<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, Error> {
    let contents = read_to_u8s(path)?;
    let converted = if wasm_preamble(&contents) {
        contents
    } else {
        wat2wasm(contents).map_err(|_| {
            format_err!("Input is neither valid WASM nor WAT").context(LucetcErrorKind::Input)
        })?
    };
    Ok(converted)
}

pub fn read_to_u8s<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, Error> {
    let mut buf: Vec<u8> = Vec::new();
    let mut file = File::open(path)?;
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn wasm_preamble(buf: &[u8]) -> bool {
    if buf.len() > 4 {
        buf[0..4] == [0, 97, 115, 109]
    } else {
        false
    }
}
