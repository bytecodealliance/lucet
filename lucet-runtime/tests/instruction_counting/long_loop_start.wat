(module
  (start $start)
  (func $start (export "_start") (local i32)
      loop
        local.get 0
        i32.const 1
	i32.add
	local.tee 0
	i32.const 10000
	i32.ne
	br_if 0
      end
  )
  (func $instruction_count (export "instruction_count") (result i64)
    i64.const 70000
  )
)
