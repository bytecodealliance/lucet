(module
    (import "atoms" "int_float_args" (func $int_float_args (param i32 f32) (result i32)))
    (import "atoms" "double_int_return_float" (func $double_int_return_float (param i32 i32) (result i32)))

    (memory 1)
    (export "memory" (memory 0))

    (func $int_float_args_shim (param i32 f32) (result i32)
        local.get 0
        local.get 1
        call $int_float_args
    )
    (func $double_int_return_float_shim (param i32 i32) (result i32)
        local.get 0
        local.get 1
        call $double_int_return_float
    )
    (export "int_float_args_shim" (func $int_float_args_shim))
    (export "double_int_return_float_shim" (func $double_int_return_float_shim))
)
