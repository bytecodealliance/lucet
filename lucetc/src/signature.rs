use crate::error::Error;
use lucet_module::ModuleSignature;
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
        None => SecretKey::from_file(sk_path, None).map_err(|e| {
            let message = format!("Unable to read the secret key: {}", e);
            Error::Signature(message)
        }),
        Some(sk_path) => {
            let mut sk_bin: Vec<u8> = Vec::new();
            File::open(sk_path)?.read_to_end(&mut sk_bin)?;
            SecretKey::from_bytes(&sk_bin).map_err(|e| {
                let message = format!("Unable to read the secret key: {}", e);
                Error::Signature(message)
            })
        }
    }
}

fn signature_path<P: AsRef<Path>>(path: P) -> Result<PathBuf, Error> {
    let path = path.as_ref().to_str().ok_or_else(|| {
        let message = format!("Invalid signature path {:?}", path.as_ref());
        Error::Input(message)
    })?;
    Ok(PathBuf::from(format!("{}.minisig", path)))
}

pub fn signature_box_for_module_path<P: AsRef<Path>>(path: P) -> Result<SignatureBox, Error> {
    let signature_path = signature_path(path)?;
    SignatureBox::from_file(&signature_path).map_err(|e| {
        let message = format!("Unable to load the signature file: {}", e);
        Error::Signature(message)
    })
}

pub fn keygen<P: AsRef<Path>, Q: AsRef<Path>>(pk_path: P, sk_path: Q) -> Result<KeyPair, Error> {
    match raw_key_path(&sk_path) {
        None => {
            let pk_writer = File::create(pk_path)?;
            let sk_writer = File::create(sk_path)?;
            KeyPair::generate_and_write_encrypted_keypair(pk_writer, sk_writer, None, None).map_err(
                |e| {
                    let message = format!("Unable to generate the key pair: {}", e);
                    Error::Signature(message)
                },
            )
        }
        Some(sk_path_raw) => {
            let kp = KeyPair::generate_unencrypted_keypair().map_err(|e| {
                let message = format!("Unable to generate the key pair: {}", e);
                Error::Signature(message)
            })?;
            let mut pk_writer = File::create(pk_path)?;
            let mut sk_writer = File::create(sk_path_raw)?;

            pk_writer.write_all(&kp.pk.to_box().unwrap().to_bytes())?;
            sk_writer.write_all(&kp.sk.to_bytes())?;

            Ok(kp)
        }
    }
}

// Verify the source code (WASM / WAT)
pub fn verify_source_code(
    buf: &[u8],
    signature_box: &SignatureBox,
    pk: &PublicKey,
) -> Result<(), Error> {
    minisign::verify(pk, signature_box, Cursor::new(buf), false, false)
        .map_err(|e| Error::Signature(e.to_string()))
}

// Sign the compiled code
pub fn sign_module<P: AsRef<Path>>(path: P, sk: &SecretKey) -> Result<(), Error> {
    ModuleSignature::sign(path, sk).map_err(|e| e.into())
}
