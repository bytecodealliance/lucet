(module
  (type (func (param i32 i32 i32 i32) (result i32)))
  (type (func))

  ;; import fd_read, this is fine.
  (func $read (import "env" "read") (type 0))

  ;; import fd_write, this is also fine.
  (func $write (import "env" "write") (type 0))

  ;; import fd_read, again, under a different name!
  ;; this is to test that we join together the imports.
  ;; the .wat would be invalid if their types disagree, so there
  ;; is no observable difference between $read and $read_2
  (func $read_2 (import "env" "read") (type 0))

  ;; import fd_write again for grins.
  (import "env" "write" (func (type 0)))

  (func $_setup (type 1) return)

  ;; declare that, actually, one of the imported functions is exported
  (export "read_2" (func $read_2))
  ;; and declare that the *other* read function is also exported, by a
  ;; different name. This lets us check that when we merge the functions,
  ;; we also merge their export names properly.
  (export "read" (func $read))

  ;; and check that other exported functions still work, and are not affected
  (export "write" (func $write))

  ;; and that we can export local functions without issue
  (export "_start" (func $_setup))
)
