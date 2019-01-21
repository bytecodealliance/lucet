(module
  (func $main (export "main") (result i32) (local i32)
    (set_local 0 (i32.const 10))
    (i32.add (call $localpalooza) (get_local 0))
  )
  (func $localpalooza (export "localpalooza") (result i32) (local i32 i32 i32 i32 i32 i32 i32 i32)

    (set_local 0 (i32.const 1))
    (set_local 1 (i32.const 1))
    (set_local 2 (i32.const 1))
    (set_local 3 (i32.const 1))
    (set_local 4 (i32.const 1))
    (set_local 5 (i32.const 1))
    (set_local 6 (i32.const 1))
    (set_local 7 (i32.const 1))

    (set_local 0
      (i32.add (call $localpalooza2) (get_local 0)))

    (set_local 1 (i32.add (get_local 0) (get_local 1)))
    (set_local 2 (i32.add (get_local 1) (get_local 2)))
    (set_local 3 (i32.add (get_local 2) (get_local 3)))
    (set_local 4 (i32.add (get_local 3) (get_local 4)))
    (set_local 5 (i32.add (get_local 4) (get_local 5)))
    (set_local 6 (i32.add (get_local 5) (get_local 6)))
    (set_local 7 (i32.add (get_local 6) (get_local 7)))

    (get_local 7)
  )

  (func $localpalooza2 (export "localpalooza2") (result i32) (local i32 i32 i32 i32 i32 i32 i32 i32)

    (set_local 0 (i32.const 2))
    (set_local 1 (i32.const 2))
    (set_local 2 (i32.const 2))
    (set_local 3 (i32.const 2))
    (set_local 4 (i32.const 2))
    (set_local 5 (i32.const 2))
    (set_local 6 (i32.const 2))
    (set_local 7 (i32.const 2))

    (set_local 1 (i32.add (get_local 0) (get_local 1)))
    (set_local 2 (i32.add (get_local 1) (get_local 2)))
    (set_local 3 (i32.add (get_local 2) (get_local 3)))
    (set_local 4 (i32.add (get_local 3) (get_local 4)))
    (set_local 5 (i32.add (get_local 4) (get_local 5)))
    (set_local 6 (i32.add (get_local 5) (get_local 6)))
    (set_local 7 (i32.add (get_local 6) (get_local 7)))

    (get_local 7)
  )

)
