use crate::error::Error;
use crate::signature::{self, PublicKey};
use std::path::Path;
use wabt::wat2wasm;

pub fn read_module(
    path: impl AsRef<Path>,
    pk: &Option<PublicKey>,
    verify: bool,
    translate_wat: bool,
) -> Result<Vec<u8>, Error> {
    let contents = std::fs::read(&path)?;
    if verify {
        let signature_box = signature::signature_box_for_module_path(&path)?;
        signature::verify_source_code(
            &contents,
            &signature_box,
            pk.as_ref()
                .ok_or(Error::Signature("public key is missing".to_string()))?,
        )?;
    }
    if translate_wat {
        read_bytes(contents)
    } else {
        if wasm_preamble(&contents) {
            Ok(contents)
        } else {
            Err(Error::MissingWasmPreamble)
        }
    }
}

pub fn read_bytes(bytes: Vec<u8>) -> Result<Vec<u8>, Error> {
    if wasm_preamble(&bytes) {
        Ok(bytes)
    } else {
        wat2wasm(bytes).map_err(|err| {
            let result = format!("wat2wasm {}", err);
            crate::error::Error::Input(result)
        })
    }
}

pub fn wasm_preamble(buf: &[u8]) -> bool {
    if buf.len() > 4 {
        buf[0..4] == [0, 97, 115, 109]
    } else {
        false
    }
}
