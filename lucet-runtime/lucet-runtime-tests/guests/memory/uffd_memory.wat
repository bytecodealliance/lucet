(module
  (import "env" "memory" (memory 2))
  (func $main (export "main")
    (i32.store (i32.const 65536) (i32.const 1))
    (drop (grow_memory (i32.const 2)))
    (i32.store (i32.const 196608) (i32.const 2))
  )
)
