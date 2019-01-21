use failure::*;
use parity_wasm::deserialize_buffer;
pub use parity_wasm::elements::Module;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use wabt::wat2wasm;

pub fn read_module(path: &PathBuf) -> Result<Module, Error> {
    let contents = read_to_u8s(path)?;
    let wasm = if wasm_preamble(&contents) {
        contents
    } else {
        wat2wasm(contents)?
    };
    let module_res = deserialize_buffer(&wasm);
    module_res.map_err(|e| format_err!("deserializing wasm module: {}", e))
}

pub fn read_to_u8s(path: &PathBuf) -> Result<Vec<u8>, Error> {
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
