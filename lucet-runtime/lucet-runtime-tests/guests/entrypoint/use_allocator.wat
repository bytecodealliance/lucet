(module
  (memory 1)
  ;; Expand the wasm heap and initialize it to a value.
  ;; sets out-pointer to -1 if heap expansion fails.
  ;; Parameters are:
  ;;   0. initialization value (byte)
  ;;   1. size (in wasm pages)
  ;;   2. out-pointer to store start address of expanded heap
  ;;  Locals are:
  ;;   3. expanded heap start page
  (func $expand_and_memset (export "expand_and_memset") (param i32 i32 i32)
    (local i32)
    (local.set 3 (memory.grow (local.get 1)))
    (if (i32.eq (local.get 3) (i32.const -1))
        (then (i32.store (local.get 2) (i32.const -1)))
        (else
            (i32.store (local.get 2) (i32.mul (local.get 3) (i32.const 65536)))
            (call $memset (i32.load (local.get 2)) (local.get 0) (i32.mul (local.get 1) (i32.const 65536)))
        )
    )
  )


  ;; memset
  ;; parameters are
  ;;   0. pointer to start of region
  ;;   1. constant value (byte)
  ;;   2. size of region, in bytes
  ;; locals are
  ;;   3. current pointer
  (func $memset (export "memset") (param i32 i32 i32)
    (local i32)
    (local.set 3 (local.get 0))
    (loop
     (i32.store8 (local.get 3) (local.get 1))
     (local.set 3 (i32.add (local.get 3) (i32.const 1)))
     (br_if 0 (i32.lt_u (local.get 3) (i32.add (local.get 0) (local.get 2))))
    )
  )

  ;; increment_ptr
  ;; increment the byte at the pointer
  ;; parameters are
  ;;  0. pointer to byte
  (func $increment_ptr (export "increment_ptr") (param i32)
    (i32.store8 (local.get 0)
                (i32.add (i32.const 1) (i32.load (local.get 0))))
  )

)
