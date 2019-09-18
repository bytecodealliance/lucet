(module
  (func $add_4 (export "add_4_reexport") (import "env" "add_4_hostcall") (param i64 i64 i64 i64) (result i64))
  (memory 1)
  (func $add_2 (export "add_2") (param i64 i64) (result i64)
    (i64.add (get_local 0) (get_local 1))
  )
  (func $add_f32_2 (export "add_f32_2") (param f32 f32) (result f32)
    (f32.add (get_local 0) (get_local 1))
  )
  (func $add_f64_2 (export "add_f64_2") (param f64 f64) (result f64)
    (f64.add (get_local 0) (get_local 1))
  )
  (func $add_10 (export "add_10")
    (param i64 i64 i64 i64 i64
           i64 i64 i64 i64 i64)
    (result i64)
      (i64.add
        (i64.add
          (i64.add
            (i64.add
              (i64.add
                (i64.add
                  (i64.add
                    (i64.add
                      (i64.add
                        (get_local 9)
                        (get_local 8))
                      (get_local 7))
                    (get_local 6))
                  (get_local 5))
                (get_local 4))
              (get_local 3))
            (get_local 2))
          (get_local 1))
        (get_local 0))
  )
  (func $mul_2 (export "mul_2") (param i64 i64) (result i64)
    (i64.mul (get_local 0) (get_local 1))
  )

  (func $add_f32_10 (export "add_f32_10")
    (param f32 f32 f32 f32 f32
           f32 f32 f32 f32 f32)
    (result f32)
      (f32.add
        (f32.add
          (f32.add
            (f32.add
              (f32.add
                (f32.add
                  (f32.add
                    (f32.add
                      (f32.add
                        (get_local 9)
                        (get_local 8))
                      (get_local 7))
                    (get_local 6))
                  (get_local 5))
                (get_local 4))
              (get_local 3))
            (get_local 2))
          (get_local 1))
        (get_local 0))
  )
  (func $add_f64_10 (export "add_f64_10")
    (param f64 f64 f64 f64 f64
           f64 f64 f64 f64 f64)
    (result f64)
      (f64.add
        (f64.add
          (f64.add
            (f64.add
              (f64.add
                (f64.add
                  (f64.add
                    (f64.add
                      (f64.add
                        (get_local 9)
                        (get_local 8))
                      (get_local 7))
                    (get_local 6))
                  (get_local 5))
                (get_local 4))
              (get_local 3))
            (get_local 2))
          (get_local 1))
        (get_local 0))
  )
)
