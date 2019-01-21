use super::*;
use std::path::PathBuf;

#[test]
fn run() {
    let mut module_code = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    module_code.push("..");
    module_code.push("lucet-runtime-c");
    module_code.push("test");
    module_code.push("build");
    module_code.push("entrypoint");
    module_code.push("calculator.so");

    assert!(module_code.exists(), format!("test module is part of liblucet-runtime-c test suite build - run `make test` in lucet-runtime-c to make sure it exists at {}", module_code.display()));
    let module_code = std::fs::canonicalize(module_code).expect("absolute path");

    let pool = Pool::builder().build().expect("build");
    let module = Module::from_file(module_code).expect("module");
    let mut instance = pool.instantiate(&module).expect("instantiate");

    assert!(instance.grow_memory(1000).is_err());
    instance.grow_memory(pool.page_size()).expect("grow_memory");

    // it would be good to have more than a negative test, but this
    // example doesn't have a `start` section
    let status = instance.run_start();
    assert!(match status {
        Err(LucetError::SymbolNotFound(sym)) => sym == "guest_start".to_owned(),
        _ => false,
    });

    let status = instance.run("__nonexistent", &vec![]);
    assert!(match status {
        Err(LucetError::SymbolNotFound(sym)) => sym == "__nonexistent".to_owned(),
        _ => false,
    });

    let arg1 = 420;
    let arg2 = 410757864530;
    let res = instance
        .run("add_2", &vec![Val::U16(arg1), Val::U64(arg2)])
        .expect("run");
    assert!(res.as_u64() == arg1 as u64 + arg2);

    let res = instance
        .run("mul_2", &vec![Val::U16(arg1), Val::U64(arg2)])
        .expect("run");
    assert!(res.as_u64() == arg1 as u64 * arg2);

    let arg1 = -6.9f32;
    let arg2 = 4.2f32;
    let res = instance
        .run("add_f32_2", &vec![Val::F32(arg1), Val::F32(arg2)])
        .expect("run");
    assert!(res.as_f32() == arg1 + arg2);

    let arg1 = -6.9;
    let arg2 = 4.2;
    let res = instance
        .run("add_f64_2", &vec![Val::F64(arg1), Val::F64(arg2)])
        .expect("run");
    assert!(res.as_f64() == arg1 + arg2);

    let arg1 = 420;
    let arg2 = 410757864530;
    let res = instance
        .run_func_id(0, 0, &vec![Val::U16(arg1), Val::U64(arg2)])
        .expect("run_func_id");
    assert!(res.as_u64() == arg1 as u64 + arg2);
}
