use lucet_module::bindings::Bindings;
use std::collections::HashMap;
use std::path::PathBuf;

fn test_file(f: &str) -> PathBuf {
    PathBuf::from(format!("tests/bindings/{}", f))
}

#[test]
fn explicit() {
    let mut explicit_map = HashMap::new();
    explicit_map.insert(String::from("hello"), String::from("goodbye"));
    let map = Bindings::env(explicit_map);

    let result = map.translate("env", "hello").unwrap();
    assert!(result == "goodbye");

    let result = map.translate("env", "nonexistent");
    assert!(
        result.is_err(),
        "explicit import map returned value for non-existent symbol"
    );
}

#[test]
fn explicit_from_nonexistent_file() {
    let fail_map = Bindings::from_file(&test_file("nonexistent_bindings.json"));
    assert!(
        fail_map.is_err(),
        "ImportMap::explicit_from_file did not fail on a non-existent file"
    );
}

#[test]
fn explicit_from_garbage_file() {
    let fail_map = Bindings::from_file(&test_file("garbage.json"));
    assert!(
        fail_map.is_err(),
        "ImportMap::explicit_from_file did not fail on a garbage file"
    );
}

#[test]
fn explicit_from_file() {
    let map = Bindings::from_file(&test_file("bindings_test.json"))
        .expect("load valid bindings from file");
    let result = map.translate("env", "hello").expect("hello has a binding");
    assert!(result == "json is cool");

    assert!(
        map.translate("env", "nonexistent").is_err(),
        "bindings from file returned value for non-existent symbol"
    );
}
