(module
  (type $ft (func (param i32 i32) (result i32)))
  (type $ft2 (func (param f32) (result f32)))
  (func $righttype_imported (import "env" "icalltarget")(type $ft))
  (func $launchpad (export "launchpad") (param i32 i32 i32) (result i32)
    (call_indirect (type $ft) (get_local 1) (get_local 2) (get_local 0))
  )
  (func $righttype1 (type $ft) (i32.add (get_local 0) (get_local 1)))
  (func $righttype2 (type $ft) (i32.sub (get_local 0) (get_local 1)))

  (func $wrongtype (type $ft2) (f32.sub (f32.const 0) (get_local 0)))
  (table anyfunc (elem $righttype1 $righttype2 $wrongtype $righttype_imported))
)
