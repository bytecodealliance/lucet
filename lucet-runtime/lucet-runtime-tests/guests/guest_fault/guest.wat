(module
  (func $onetwothree_hostcall (import "env" "onetwothree") (result i64))
  (func $make_onetwothree_hostcall (export "make_onetwothree_hostcall") (result i64)
    (call $onetwothree_hostcall)
  )
  (func $onetwothree (export "onetwothree") (result i64)
    (i64.const 123)
  )
)
