(module
  (func $inc (import "env" "inc") (result i32))
  (func $main (export "main") (local i32)
    (set_local 0 (i32.const 0))
    (drop (call $inc))
  )
  (start $main)
)
