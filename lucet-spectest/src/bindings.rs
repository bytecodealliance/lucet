use lucet_module_data::bindings::Bindings;
use serde_json::json;

use lucet_runtime::lucet_hostcalls;

lucet_hostcalls! {
    #[no_mangle]
    pub unsafe extern "C" fn print(&mut _vmctx,) -> () {
        println!("hello, world!");
    }
}

lucet_hostcalls! {
    #[no_mangle]
    pub unsafe extern "C" fn print_i32(&mut _vmctx, x: i32,) -> () {
        println!("{}", x);
    }
}

lucet_hostcalls! {
    #[no_mangle]
    pub unsafe extern "C" fn print_f32(&mut _vmctx, x: i32,) -> () {
        println!("{}", x);
    }
}

pub fn spec_test_bindings() -> Bindings {
    let imports: serde_json::Value = json!({
        "test": {
            "func": "func",
            "unknown": "unknown",
            "func-i32": "func_i32",
            "func-f32": "func_f32",
            "func->i32": "func_to_i32",
            "func->f32": "func_to_f32",
            "func-i32->i32": "func_i32_to_i32",
            "func-i64->i64": "func_i64_to_i64",
            "table-10-inf": "table_10_inf",
            "memory-2-inf": "memory_2_inf",
            "global-i32": "global_i32",
        },
        "spectest": {
            "memory": "memory",
            "print": "print",
            "print_i32": "print_i32",
            "print_f32": "print_f32",
            "print_i32_f32": "print_i32_f32",
            "print_f64": "print_f64",
            "print_f64_f64": "print_f64_f64",
            "unknown": "unknown",
            "table": "table",
            "global_i32": "global_i32",
        },
        "Mt": {
            "h": "h",
            "call": "mt_call",
        },
        "Mf": {
            "call": "mf_call",
        },
        "Mg": {
            "get": "mg_get",
        },
        "Mm": {
            "load": "mm_load",
        },
        "reexport_f": {
            "print": "print",
        },
        "not wasm": {
            "overloaded": "overloaded",
        }
    });

    Bindings::from_json(&imports).expect("bindings valid")
}
