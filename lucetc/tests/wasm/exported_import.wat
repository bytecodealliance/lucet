(module
  (func (import "env" "inc"))
  (func $main (export "exported_main") (import "env" "imported_main"))

  ;; cranelift_wasm bundles up import/export/declaration statements and
  ;; declares them together. lucetc depends on function declaration
  ;; components not being interwoven, so test that this is still bundled
  ;; up by exporting after declaring a new function ("$main", above)
  (export "exported_inc" (func 0))

  (start $main)
)
