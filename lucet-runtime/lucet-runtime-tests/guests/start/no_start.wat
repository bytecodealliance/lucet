;; make sure that `lucet_instance_run_start` is a noop for modules
;; without a `start` section

(module
 (type $v (func))
 (global $flossie (mut i32) (i32.const 16))
 (memory $0 1)
 (export "memory" (memory $0))
 (export "main" (func $main))
 (func $main (; 0 ;) (type $v)
  (call $inc)
  (i32.store (i32.const 0) (get_global $flossie))
 )
 (func $inc (; 1 ;) (type $v)
  (set_global $flossie
   (i32.add
    (get_global $flossie)
    (i32.const 1)
   )
  )
 )
)
