;; Used as part of testing that lucetc outputs expected data segment
;; initialization info to the ELF output file it produces

(module
  ;; linear memory is min of 1 page, no max specified
  (memory 1)
  (func $main (export "main") (local i32)
    ;; Try loading a value from memory at 0, which should be set by the data 
	;; initializers, but they don't work, so this is 0
    (i32.store (i32.const 0) (i32.load (i32.const 0) ) )
  )
  (start $main)
  ;; This will store the bytes starting at offset 0
  (data (i32.const 0) "99999")
  ;; This will store some other bytes starting at offset 0, which would
  ;; overwrite the some of the bytes above at instantiation time
  (data (i32.const 0) "\aa\bb")
  ;; This will store some other bytes starting at offset 1
  (data (i32.const 1) "\cc\dd")
)
