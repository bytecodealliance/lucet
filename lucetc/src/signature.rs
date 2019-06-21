use failure::*;
use lucet_module_data::{ModuleData, RawModuleAndData};
pub use minisign::{KeyPair, PublicKey, SecretKey, SignatureBones, SignatureBox};
use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

pub const RAW_KEY_PREFIX: &str = "raw:";

fn raw_key_path<P: AsRef<Path>>(path: P) -> Option<PathBuf> {
    let path = path.as_ref();
    if let Some(path) = path.to_str() {
        if path.starts_with(RAW_KEY_PREFIX) {
            return Some(PathBuf::from(&path[RAW_KEY_PREFIX.len()..]));
        }
    }
    None
}

pub fn sk_from_file<P: AsRef<Path>>(sk_path: P) -> Result<SecretKey, Error> {
    match raw_key_path(sk_path.as_ref()) {
        None => SecretKey::from_file(sk_path, None)
            .map_err(|_| format_err!("Unable to read the secret key")),
        Some(sk_path) => {
            let mut sk_bin: Vec<u8> = Vec::new();
            File::open(sk_path)?.read_to_end(&mut sk_bin)?;
            SecretKey::from_bytes(&sk_bin).map_err(|_| format_err!("Unable to read the secret key"))
        }
    }
}

fn signature_path<P: AsRef<Path>>(path: P) -> Result<PathBuf, Error> {
    let path = path
        .as_ref()
        .to_str()
        .ok_or_else(|| format_err!("Invalid path"))?;
    Ok(PathBuf::from(format!("{}.minisig", path)))
}

pub fn signature_box_for_module_path<P: AsRef<Path>>(path: P) -> Result<SignatureBox, Error> {
    let signature_path = signature_path(path)?;
    SignatureBox::from_file(&signature_path)
        .map_err(|_| format_err!("Unable to load the signature file"))
}

pub fn keygen<P: AsRef<Path>, Q: AsRef<Path>>(pk_path: P, sk_path: Q) -> Result<KeyPair, Error> {
    match raw_key_path(&sk_path) {
        None => {
            let pk_writer = File::create(pk_path)?;
            let sk_writer = File::create(sk_path)?;
            KeyPair::generate_and_write_encrypted_keypair(pk_writer, sk_writer, None, None)
                .map_err(|_| format_err!("Unable to generate the key pair"))
        }
        Some(sk_path_raw) => {
            let kp = KeyPair::generate_unencrypted_keypair()
                .map_err(|_| format_err!("Unable to generate the key pair"))?;
            let mut pk_writer = File::create(pk_path)?;
            let mut sk_writer = File::create(sk_path_raw)?;
            pk_writer.write_all(&kp.pk.to_box()?.to_bytes())?;
            sk_writer.write_all(&kp.sk.to_bytes())?;
            Ok(kp)
        }
    }
}

pub fn verify(buf: &[u8], signature_box: &SignatureBox, pk: &PublicKey) -> Result<(), Error> {
    minisign::verify(pk, signature_box, Cursor::new(buf), false, false)
        .map_err(|_| format_err!("Unable to verify the signature"))
}

pub fn sign<P: AsRef<Path>>(path: P, sk: &SecretKey) -> Result<(), Error> {
    let raw_module_and_data = RawModuleAndData::from_file(&path)?;
    let signature_box = minisign::sign(
        None,
        sk,
        Cursor::new(&raw_module_and_data.obj_bin),
        true,
        None,
        None,
    )?;
    let signature_bones: SignatureBones = signature_box.into();
    let patched_module_data_bin = ModuleData::patch_module_signature(
        raw_module_and_data.module_data_bin(),
        &signature_bones.to_bytes(),
    )?;
    raw_module_and_data.write_patched_module_data(&path, &patched_module_data_bin)?;
    Ok(())
}
