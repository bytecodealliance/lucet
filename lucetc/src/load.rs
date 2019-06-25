use crate::error::LucetcErrorKind;
use crate::signature::{self, PublicKey};
use failure::*;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use wabt::wat2wasm;

pub fn read_module<P: AsRef<Path>>(
    path: P,
    pk: &Option<PublicKey>,
    verify: bool,
) -> Result<Vec<u8>, Error> {
    let signature_box = if verify {
        Some(signature::signature_box_for_module_path(&path)?)
    } else {
        None
    };
    let contents = read_to_u8s(path)?;
    if let Some(signature_box) = signature_box {
        signature::verify_source_code(
            &contents,
            &signature_box,
            pk.as_ref()
                .ok_or(format_err!("public key is missing").context(LucetcErrorKind::Signature))?,
        )?;
    }
    read_bytes(contents)
}

pub fn read_bytes(bytes: Vec<u8>) -> Result<Vec<u8>, Error> {
    let converted = if wasm_preamble(&bytes) {
        bytes
    } else {
        wat2wasm(bytes).map_err(|_| {
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
