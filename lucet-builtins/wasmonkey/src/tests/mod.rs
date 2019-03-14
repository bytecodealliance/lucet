use super::*;
use siphasher::sip::SipHasher13;
use std::hash::Hasher;
use std::path::{Path, PathBuf};

lazy_static! {
    static ref TESTS_DIR: PathBuf = Path::new(file!()).parent().unwrap().canonicalize().unwrap();
}

#[test]
fn patch_nothing() {
    let path_in = TESTS_DIR.join("test_1.wasm");
    let config = PatcherConfig::default();
    let patcher = Patcher::from_file(config, path_in).unwrap();
    let mut hasher = SipHasher13::new();
    hasher.write(&patcher.into_bytes().unwrap());
    assert_eq!(hasher.finish(), 1401932366200566186);
}

#[test]
fn patch_one() {
    let path_in = TESTS_DIR.join("test_1.wasm");
    let mut config = PatcherConfig::default();
    config.builtins_additional = ["builtin_memmove", "builtin_nonexistent", "not_a_builtin"]
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    let patcher = Patcher::from_file(config, path_in).unwrap();
    let mut hasher = SipHasher13::new();
    hasher.write(&patcher.into_bytes().unwrap());
    assert_eq!(hasher.finish(), 12884721342785729260);
}

#[test]
fn patch_some() {
    let path_in = TESTS_DIR.join("test_1.wasm");
    let mut config = PatcherConfig::default();
    config.builtins_additional = ["builtin_memmove", "builtin_memcpy", "builtin_strcmp"]
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    let patcher = Patcher::from_file(config, path_in).unwrap();
    let mut hasher = SipHasher13::new();
    hasher.write(&patcher.into_bytes().unwrap());
    assert_eq!(hasher.finish(), 13205801729184435761);
}
