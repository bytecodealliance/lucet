use crate::error::Error;
use crate::signature::{self, PublicKey};
use std::path::Path;
use wabt::{wat2wasm, ErrorKind};

pub fn read_module(
    path: impl AsRef<Path>,
    pk: &Option<PublicKey>,
    verify: bool,
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
    read_bytes(contents)
}

pub fn read_bytes(bytes: Vec<u8>) -> Result<Vec<u8>, Error> {
    if wasm_preamble(&bytes) {
        Ok(bytes)
    } else {
        wat2wasm(bytes).map_err(|err| {
            let mut result = format!("wat2wasm error: {}", err);
            match unsafe { std::mem::transmute::<wabt::Error, wabt::ErrorKind>(err) } {
                ErrorKind::Parse(msg) |
                // this shouldn't be reachable - we're going the other way
                ErrorKind::Deserialize(msg) |
                // not sure how this error comes up
                ErrorKind::ResolveNames(msg) |
                ErrorKind::Validate(msg) => {
                    result.push_str(":\n");
                    result.push_str(&msg);
                },
                _ => { }
            };
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
