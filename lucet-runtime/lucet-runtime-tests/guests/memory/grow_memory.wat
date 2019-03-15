(module
  (import "env" "memory" (memory 4))
  (func $main (export "main")
    (i32.store (i32.const 0) (grow_memory (i32.const 1)))
    (i32.store (i32.const 4) (current_memory))
  )
)
