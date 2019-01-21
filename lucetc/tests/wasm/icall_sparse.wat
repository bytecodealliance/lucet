(module
  (type $ft (func (result i32)))
  (type $ft2 (func (param f32) (result f32)))
  (func $foo (export "foo") (param i32) (result i32)
    (call_indirect (type $ft) (get_local 0))
  )
  (func $righttype1 (type $ft) (i32.const 1))
  (func $righttype2 (type $ft) (i32.const 2))
  (func $wrongtype (type $ft2) (f32.const 0.12345))
  ;; Declare table with minimum 6 elements, maximum 7. since we dont actually insert a [6] element, it should end up 6 in size.
  (table 6 7 anyfunc)
  (elem (i32.const 1) $righttype1 $righttype2 $wrongtype)
)
