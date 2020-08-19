(module
  (import "wasi_snapshot_preview1" "args_get" (func $args_get (param i32 i32) (result i32)))
  (memory (export "memory") 1)
  (func $not_start (export "not_start") (local i32)
      (set_local 0 (i32.sub (i32.const 4) (i32.const 4)))
      (if
          (get_local 0)
          (then unreachable)
          (else (i32.store (i32.const 0) (i32.mul (i32.const 6) (get_local 0))))
      )
  )
  (data (i32.const 0) "abcdefgh")
)
