(module
  (global $x (import "env" "x") i32)
  (memory 1)
  (func $main (export "main") (local i32)
    (i32.store (i32.const 0) (get_global $x))
  )
  (start $main)
)
