(module
  (func $inc (import "env" "inc"))
  (func $inc_duplicate (import "env" "inc"))
  (func $foo (import "env" "imported_main"))
  (func $inc_another_duplicate (export "exported_inc") (import "env" "inc"))
  (start $inc)
)
