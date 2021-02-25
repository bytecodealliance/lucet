;; Make sure br_table backedges are handled properly.
(module
  (func $main (export "test_function") (local i32)
      block
        loop
          local.get 0
          i32.const 1
          i32.add
          local.tee 0
          i32.const 10000
          i32.eq
          br_table 0 1
        end
      end
  )
  (func $instruction_count (export "instruction_count") (result i64)
    i64.const 80000
  )
)
