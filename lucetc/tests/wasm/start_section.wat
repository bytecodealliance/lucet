(module
 (type $v (func))
 (global $flossie (mut i32) (i32.const 0))
 (memory $0 1)
 (export "memory" (memory $0))
 (export "main" (func $main))
 (start $start)
 (func $main (; 0 ;) (type $v)
  (i32.store (i32.const 0) (get_global $flossie))
 )
 (func $start (; 1 ;) (type $v)
  (set_global $flossie
   (i32.add
    (i32.const 16)
    (i32.const 1)
   )
  )
 )
)

