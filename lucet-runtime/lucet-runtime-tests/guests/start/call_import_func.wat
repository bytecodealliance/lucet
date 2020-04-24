;; sets the `start` section to a function that we also call from
;; normal user code

(module
 (type $v (func))
 (func $not_allowed_during_start (import "env" "not_allowed_during_start"))
 (start $bad_start)
 (func $bad_start (; 1 ;) (type $v)
   (call $not_allowed_during_start)
 )
)
