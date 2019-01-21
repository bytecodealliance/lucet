(module
  (memory 1)
  (func $main (export "main")
    (i32.store (i32.const 0) (current_memory))
  )
)
