(module
  (type $t0 (func (param i32 i32 i32) (result i32)))
  (type $t1 (func (param i32 i64 i32) (result i64)))
  (type $t2 (func (param i32)))
  (type $t3 (func (param i32) (result i32)))
  (type $t4 (func (param i32 i32) (result i32)))
  (type $t5 (func (param i32 i64 i32 i32) (result i32)))
  (type $t6 (func (param i32 i32 i32 i32) (result i32)))
  (type $t7 (func))
  (type $t8 (func (result i32)))
  (import "env" "__wasi_proc_exit" (func $env.__wasi_proc_exit (type $t2)))
  (import "env" "__wasi_fd_close" (func $env.__wasi_fd_close (type $t3)))
  (import "env" "__wasi_fd_stat_get" (func $env.__wasi_fd_stat_get (type $t4)))
  (import "env" "__wasi_fd_seek" (func $env.__wasi_fd_seek (type $t5)))
  (import "env" "__wasi_fd_write" (func $env.__wasi_fd_write (type $t6)))
  (func $f5 (type $t7))
  (func $_start (type $t7)
    (local $l0 i32)
    (call $f5)
    (set_local $l0
      (call $f7
        (i32.const 0)
        (i32.const 0)))
    (call $f10)
    (block $B0
      (br_if $B0
        (get_local $l0))
      (return))
    (call $f8
      (get_local $l0))
    (unreachable))
  (func $f7 (type $t4) (param $p0 i32) (param $p1 i32) (result i32)
    (drop
      (call $f23
        (i32.const 1024)))
    (i32.const 0))
  (func $f8 (type $t2) (param $p0 i32)
    (call $env.__wasi_proc_exit
      (get_local $p0))
    (unreachable))
  (func $f9 (type $t7))
  (func $f10 (type $t7)
    (call $f9)
    (call $f20))
  (func $f11 (type $t3) (param $p0 i32) (result i32)
    (block $B0
      (br_if $B0
        (i32.eqz
          (tee_local $p0
            (call $env.__wasi_fd_close
              (get_local $p0)))))
      (i32.store offset=1040
        (i32.const 0)
        (get_local $p0))
      (return
        (i32.const -1)))
    (i32.const 0))
  (func $f12 (type $t3) (param $p0 i32) (result i32)
    (get_local $p0))
  (func $f13 (type $t3) (param $p0 i32) (result i32)
    (call $f11
      (call $f12
        (i32.load offset=60
          (get_local $p0)))))
  (func $f14 (type $t3) (param $p0 i32) (result i32)
    (local $l0 i32) (local $l1 i32)
    (set_global $g0
      (tee_local $l0
        (i32.sub
          (get_global $g0)
          (i32.const 32))))
    (block $B0
      (block $B1
        (block $B2
          (br_if $B2
            (tee_local $p0
              (call $env.__wasi_fd_stat_get
                (get_local $p0)
                (i32.add
                  (get_local $l0)
                  (i32.const 8)))))
          (set_local $p0
            (i32.const 59))
          (br_if $B2
            (i32.ne
              (i32.load8_u offset=8
                (get_local $l0))
              (i32.const 2)))
          (br_if $B1
            (i32.eqz
              (i32.and
                (i32.load8_u offset=16
                  (get_local $l0))
                (i32.const 36)))))
        (set_local $l1
          (i32.const 0))
        (i32.store offset=1040
          (i32.const 0)
          (get_local $p0))
        (br $B0))
      (set_local $l1
        (i32.const 1)))
    (set_global $g0
      (i32.add
        (get_local $l0)
        (i32.const 32)))
    (get_local $l1))
  (func $f15 (type $t0) (param $p0 i32) (param $p1 i32) (param $p2 i32) (result i32)
    (i32.store offset=36
      (get_local $p0)
      (i32.const 1))
    (block $B0
      (block $B1
        (br_if $B1
          (i32.and
            (i32.load8_u
              (get_local $p0))
            (i32.const 64)))
        (br_if $B0
          (i32.eqz
            (call $f14
              (i32.load offset=60
                (get_local $p0))))))
      (return
        (call $f27
          (get_local $p0)
          (get_local $p1)
          (get_local $p2))))
    (i32.store offset=80
      (get_local $p0)
      (i32.const -1))
    (call $f27
      (get_local $p0)
      (get_local $p1)
      (get_local $p2)))
  (func $f16 (type $t1) (param $p0 i32) (param $p1 i64) (param $p2 i32) (result i64)
    (local $l0 i32)
    (set_global $g0
      (tee_local $l0
        (i32.sub
          (get_global $g0)
          (i32.const 16))))
    (block $B0
      (block $B1
        (br_if $B1
          (i32.eqz
            (tee_local $p0
              (call $env.__wasi_fd_seek
                (get_local $p0)
                (get_local $p1)
                (i32.and
                  (get_local $p2)
                  (i32.const 255))
                (i32.add
                  (get_local $l0)
                  (i32.const 8))))))
        (i32.store offset=1040
          (i32.const 0)
          (select
            (i32.const 70)
            (get_local $p0)
            (i32.eq
              (get_local $p0)
              (i32.const 76))))
        (set_local $p1
          (i64.const -1))
        (br $B0))
      (set_local $p1
        (i64.load offset=8
          (get_local $l0))))
    (set_global $g0
      (i32.add
        (get_local $l0)
        (i32.const 16)))
    (get_local $p1))
  (func $f17 (type $t1) (param $p0 i32) (param $p1 i64) (param $p2 i32) (result i64)
    (call $f16
      (i32.load offset=60
        (get_local $p0))
      (get_local $p1)
      (get_local $p2)))
  (func $f18 (type $t4) (param $p0 i32) (param $p1 i32) (result i32)
    (local $l0 i32)
    (select
      (i32.const -1)
      (i32.const 0)
      (i32.ne
        (tee_local $l0
          (call $f28
            (get_local $p0)))
        (call $f25
          (get_local $p0)
          (i32.const 1)
          (get_local $l0)
          (get_local $p1)))))
  (func $f19 (type $t8) (result i32)
    (i32.const 2088))
  (func $f20 (type $t7)
    (local $l0 i32) (local $l1 i32) (local $l2 i32)
    (block $B0
      (br_if $B0
        (i32.eqz
          (tee_local $l0
            (i32.load
              (call $f19)))))
      (loop $L1
        (block $B2
          (br_if $B2
            (i32.eq
              (i32.load offset=20
                (get_local $l0))
              (i32.load offset=28
                (get_local $l0))))
          (drop
            (call_indirect (type $t0)
              (get_local $l0)
              (i32.const 0)
              (i32.const 0)
              (i32.load offset=36
                (get_local $l0)))))
        (block $B3
          (br_if $B3
            (i32.eq
              (tee_local $l1
                (i32.load offset=4
                  (get_local $l0)))
              (tee_local $l2
                (i32.load offset=8
                  (get_local $l0)))))
          (drop
            (call_indirect (type $t1)
              (get_local $l0)
              (i64.extend_s/i32
                (i32.sub
                  (get_local $l1)
                  (get_local $l2)))
              (i32.const 0)
              (i32.load offset=40
                (get_local $l0)))))
        (br_if $L1
          (tee_local $l0
            (i32.load offset=56
              (get_local $l0))))))
    (block $B4
      (br_if $B4
        (i32.eqz
          (tee_local $l0
            (i32.load offset=2092
              (i32.const 0)))))
      (block $B5
        (br_if $B5
          (i32.eq
            (i32.load offset=20
              (get_local $l0))
            (i32.load offset=28
              (get_local $l0))))
        (drop
          (call_indirect (type $t0)
            (get_local $l0)
            (i32.const 0)
            (i32.const 0)
            (i32.load offset=36
              (get_local $l0)))))
      (br_if $B4
        (i32.eq
          (tee_local $l1
            (i32.load offset=4
              (get_local $l0)))
          (tee_local $l2
            (i32.load offset=8
              (get_local $l0)))))
      (drop
        (call_indirect (type $t1)
          (get_local $l0)
          (i64.extend_s/i32
            (i32.sub
              (get_local $l1)
              (get_local $l2)))
          (i32.const 0)
          (i32.load offset=40
            (get_local $l0)))))
    (block $B6
      (br_if $B6
        (i32.eqz
          (tee_local $l0
            (i32.load offset=2240
              (i32.const 0)))))
      (block $B7
        (br_if $B7
          (i32.eq
            (i32.load offset=20
              (get_local $l0))
            (i32.load offset=28
              (get_local $l0))))
        (drop
          (call_indirect (type $t0)
            (get_local $l0)
            (i32.const 0)
            (i32.const 0)
            (i32.load offset=36
              (get_local $l0)))))
      (br_if $B6
        (i32.eq
          (tee_local $l1
            (i32.load offset=4
              (get_local $l0)))
          (tee_local $l2
            (i32.load offset=8
              (get_local $l0)))))
      (drop
        (call_indirect (type $t1)
          (get_local $l0)
          (i64.extend_s/i32
            (i32.sub
              (get_local $l1)
              (get_local $l2)))
          (i32.const 0)
          (i32.load offset=40
            (get_local $l0)))))
    (block $B8
      (br_if $B8
        (i32.eqz
          (tee_local $l0
            (i32.load offset=2092
              (i32.const 0)))))
      (block $B9
        (br_if $B9
          (i32.eq
            (i32.load offset=20
              (get_local $l0))
            (i32.load offset=28
              (get_local $l0))))
        (drop
          (call_indirect (type $t0)
            (get_local $l0)
            (i32.const 0)
            (i32.const 0)
            (i32.load offset=36
              (get_local $l0)))))
      (br_if $B8
        (i32.eq
          (tee_local $l1
            (i32.load offset=4
              (get_local $l0)))
          (tee_local $l2
            (i32.load offset=8
              (get_local $l0)))))
      (drop
        (call_indirect (type $t1)
          (get_local $l0)
          (i64.extend_s/i32
            (i32.sub
              (get_local $l1)
              (get_local $l2)))
          (i32.const 0)
          (i32.load offset=40
            (get_local $l0))))))
  (func $f21 (type $t3) (param $p0 i32) (result i32)
    (local $l0 i32)
    (i32.store offset=72
      (get_local $p0)
      (i32.or
        (i32.add
          (tee_local $l0
            (i32.load offset=72
              (get_local $p0)))
          (i32.const -1))
        (get_local $l0)))
    (block $B0
      (br_if $B0
        (i32.and
          (tee_local $l0
            (i32.load
              (get_local $p0)))
          (i32.const 8)))
      (i64.store offset=4 align=4
        (get_local $p0)
        (i64.const 0))
      (i32.store offset=28
        (get_local $p0)
        (tee_local $l0
          (i32.load offset=44
            (get_local $p0))))
      (i32.store offset=20
        (get_local $p0)
        (get_local $l0))
      (i32.store offset=16
        (get_local $p0)
        (i32.add
          (get_local $l0)
          (i32.load offset=48
            (get_local $p0))))
      (return
        (i32.const 0)))
    (i32.store
      (get_local $p0)
      (i32.or
        (get_local $l0)
        (i32.const 32)))
    (i32.const -1))
  (func $f22 (type $t4) (param $p0 i32) (param $p1 i32) (result i32)
    (local $l0 i32) (local $l1 i32) (local $l2 i32)
    (set_global $g0
      (tee_local $l0
        (i32.sub
          (get_global $g0)
          (i32.const 16))))
    (i32.store8 offset=15
      (get_local $l0)
      (get_local $p1))
    (block $B0
      (block $B1
        (br_if $B1
          (tee_local $l1
            (i32.load offset=16
              (get_local $p0))))
        (set_local $l1
          (i32.const -1))
        (br_if $B0
          (call $f21
            (get_local $p0)))
        (set_local $l1
          (i32.load
            (i32.add
              (get_local $p0)
              (i32.const 16)))))
      (block $B2
        (block $B3
          (br_if $B3
            (i32.eq
              (tee_local $l2
                (i32.load offset=20
                  (get_local $p0)))
              (get_local $l1)))
          (br_if $B2
            (i32.ne
              (i32.load offset=80
                (get_local $p0))
              (tee_local $l1
                (i32.and
                  (get_local $p1)
                  (i32.const 255))))))
        (set_local $l1
          (i32.const -1))
        (br_if $B0
          (i32.ne
            (call_indirect (type $t0)
              (get_local $p0)
              (i32.add
                (get_local $l0)
                (i32.const 15))
              (i32.const 1)
              (i32.load offset=36
                (get_local $p0)))
            (i32.const 1)))
        (set_local $l1
          (i32.load8_u offset=15
            (get_local $l0)))
        (br $B0))
      (i32.store
        (i32.add
          (get_local $p0)
          (i32.const 20))
        (i32.add
          (get_local $l2)
          (i32.const 1)))
      (i32.store8
        (get_local $l2)
        (get_local $p1)))
    (set_global $g0
      (i32.add
        (get_local $l0)
        (i32.const 16)))
    (get_local $l1))
  (func $f23 (type $t3) (param $p0 i32) (result i32)
    (block $B0
      (br_if $B0
        (i32.lt_s
          (call $f18
            (get_local $p0)
            (i32.const 2096))
          (i32.const 0)))
      (block $B1
        (br_if $B1
          (i32.eq
            (i32.load offset=2176
              (i32.const 0))
            (i32.const 10)))
        (br_if $B1
          (i32.eq
            (tee_local $p0
              (i32.load offset=2116
                (i32.const 0)))
            (i32.load offset=2112
              (i32.const 0))))
        (i32.store offset=2116
          (i32.const 0)
          (i32.add
            (get_local $p0)
            (i32.const 1)))
        (i32.store8
          (get_local $p0)
          (i32.const 10))
        (return
          (i32.const 0)))
      (return
        (i32.shr_s
          (call $f22
            (i32.const 2096)
            (i32.const 10))
          (i32.const 31))))
    (i32.const -1))
  (func $f24 (type $t0) (param $p0 i32) (param $p1 i32) (param $p2 i32) (result i32)
    (local $l0 i32) (local $l1 i32) (local $l2 i32) (local $l3 i32) (local $l4 i32) (local $l5 i32)
    (block $B0
      (block $B1
        (br_if $B1
          (tee_local $l0
            (i32.load offset=16
              (get_local $p2))))
        (set_local $l3
          (i32.const 0))
        (br_if $B0
          (call $f21
            (get_local $p2)))
        (set_local $l0
          (i32.load
            (i32.add
              (get_local $p2)
              (i32.const 16)))))
      (block $B2
        (br_if $B2
          (i32.ge_u
            (i32.sub
              (get_local $l0)
              (tee_local $l1
                (i32.load offset=20
                  (get_local $p2))))
            (get_local $p1)))
        (return
          (call_indirect (type $t0)
            (get_local $p2)
            (get_local $p0)
            (get_local $p1)
            (i32.load offset=36
              (get_local $p2)))))
      (set_local $l2
        (i32.const 0))
      (block $B3
        (br_if $B3
          (i32.lt_s
            (i32.load offset=80
              (get_local $p2))
            (i32.const 0)))
        (set_local $l2
          (i32.const 0))
        (set_local $l3
          (get_local $p0))
        (set_local $l0
          (i32.const 0))
        (loop $L4
          (br_if $B3
            (i32.eq
              (get_local $p1)
              (get_local $l0)))
          (set_local $l0
            (i32.add
              (get_local $l0)
              (i32.const 1)))
          (set_local $l4
            (i32.add
              (get_local $l3)
              (get_local $p1)))
          (set_local $l3
            (tee_local $l5
              (i32.add
                (get_local $l3)
                (i32.const -1))))
          (br_if $L4
            (i32.ne
              (i32.load8_u
                (i32.add
                  (get_local $l4)
                  (i32.const -1)))
              (i32.const 10))))
        (br_if $B0
          (i32.lt_u
            (tee_local $l3
              (call_indirect (type $t0)
                (get_local $p2)
                (get_local $p0)
                (tee_local $l2
                  (i32.add
                    (i32.sub
                      (get_local $p1)
                      (get_local $l0))
                    (i32.const 1)))
                (i32.load offset=36
                  (get_local $p2))))
            (get_local $l2)))
        (set_local $p0
          (i32.add
            (i32.add
              (get_local $l5)
              (get_local $p1))
            (i32.const 1)))
        (set_local $l1
          (i32.load
            (i32.add
              (get_local $p2)
              (i32.const 20))))
        (set_local $p1
          (i32.add
            (get_local $l0)
            (i32.const -1))))
      (drop
        (call $f29
          (get_local $l1)
          (get_local $p0)
          (get_local $p1)))
      (i32.store
        (tee_local $l0
          (i32.add
            (get_local $p2)
            (i32.const 20)))
        (i32.add
          (i32.load
            (get_local $l0))
          (get_local $p1)))
      (return
        (i32.add
          (get_local $l2)
          (get_local $p1))))
    (get_local $l3))
  (func $f25 (type $t6) (param $p0 i32) (param $p1 i32) (param $p2 i32) (param $p3 i32) (result i32)
    (local $l0 i32)
    (block $B0
      (br_if $B0
        (i32.ne
          (tee_local $p0
            (call $f24
              (get_local $p0)
              (tee_local $l0
                (i32.mul
                  (get_local $p2)
                  (get_local $p1)))
              (get_local $p3)))
          (get_local $l0)))
      (return
        (select
          (get_local $p2)
          (i32.const 0)
          (get_local $p1))))
    (i32.div_u
      (get_local $p0)
      (get_local $p1)))
  (func $f26 (type $t0) (param $p0 i32) (param $p1 i32) (param $p2 i32) (result i32)
    (local $l0 i32) (local $l1 i32)
    (set_global $g0
      (tee_local $l0
        (i32.sub
          (get_global $g0)
          (i32.const 16))))
    (set_local $l1
      (i32.const -1))
    (block $B0
      (block $B1
        (block $B2
          (br_if $B2
            (i32.le_s
              (get_local $p2)
              (i32.const -1)))
          (br_if $B1
            (i32.eqz
              (tee_local $p2
                (call $env.__wasi_fd_write
                  (get_local $p0)
                  (get_local $p1)
                  (get_local $p2)
                  (i32.add
                    (get_local $l0)
                    (i32.const 12))))))
          (i32.store offset=1040
            (i32.const 0)
            (get_local $p2))
          (set_local $l1
            (i32.const -1))
          (br $B0))
        (i32.store offset=1040
          (i32.const 0)
          (i32.const 28))
        (br $B0))
      (set_local $l1
        (i32.load offset=12
          (get_local $l0))))
    (set_global $g0
      (i32.add
        (get_local $l0)
        (i32.const 16)))
    (get_local $l1))
  (func $f27 (type $t0) (param $p0 i32) (param $p1 i32) (param $p2 i32) (result i32)
    (local $l0 i32) (local $l1 i32) (local $l2 i32) (local $l3 i32) (local $l4 i32) (local $l5 i32) (local $l6 i32)
    (set_global $g0
      (tee_local $l0
        (i32.sub
          (get_global $g0)
          (i32.const 16))))
    (i32.store offset=12
      (get_local $l0)
      (get_local $p2))
    (i32.store offset=8
      (get_local $l0)
      (get_local $p1))
    (i32.store
      (get_local $l0)
      (tee_local $p1
        (i32.load offset=28
          (get_local $p0))))
    (i32.store offset=4
      (get_local $l0)
      (tee_local $p1
        (i32.sub
          (i32.load offset=20
            (get_local $p0))
          (get_local $p1))))
    (set_local $l1
      (i32.const 2))
    (block $B0
      (block $B1
        (block $B2
          (br_if $B2
            (i32.eq
              (tee_local $l2
                (i32.add
                  (get_local $p1)
                  (get_local $p2)))
              (tee_local $l3
                (call $f26
                  (i32.load offset=60
                    (get_local $p0))
                  (get_local $l0)
                  (i32.const 2)))))
          (set_local $p1
            (get_local $l0))
          (set_local $l4
            (i32.add
              (get_local $p0)
              (i32.const 60)))
          (loop $L3
            (br_if $B1
              (i32.le_s
                (get_local $l3)
                (i32.const -1)))
            (i32.store
              (tee_local $p1
                (select
                  (i32.add
                    (get_local $p1)
                    (i32.const 8))
                  (get_local $p1)
                  (tee_local $l6
                    (i32.gt_u
                      (get_local $l3)
                      (tee_local $l5
                        (i32.load offset=4
                          (get_local $p1)))))))
              (i32.add
                (i32.load
                  (get_local $p1))
                (tee_local $l5
                  (i32.sub
                    (get_local $l3)
                    (select
                      (get_local $l5)
                      (i32.const 0)
                      (get_local $l6))))))
            (i32.store offset=4
              (get_local $p1)
              (i32.sub
                (i32.load offset=4
                  (get_local $p1))
                (get_local $l5)))
            (set_local $l2
              (i32.sub
                (get_local $l2)
                (get_local $l3)))
            (set_local $l3
              (tee_local $l6
                (call $f26
                  (i32.load
                    (get_local $l4))
                  (get_local $p1)
                  (tee_local $l1
                    (i32.sub
                      (get_local $l1)
                      (get_local $l6))))))
            (br_if $L3
              (i32.ne
                (get_local $l2)
                (get_local $l6)))))
        (i32.store
          (i32.add
            (get_local $p0)
            (i32.const 28))
          (tee_local $p1
            (i32.load offset=44
              (get_local $p0))))
        (i32.store
          (i32.add
            (get_local $p0)
            (i32.const 20))
          (get_local $p1))
        (i32.store offset=16
          (get_local $p0)
          (i32.add
            (get_local $p1)
            (i32.load offset=48
              (get_local $p0))))
        (set_local $l3
          (get_local $p2))
        (br $B0))
      (i64.store offset=16
        (get_local $p0)
        (i64.const 0))
      (set_local $l3
        (i32.const 0))
      (i32.store
        (i32.add
          (get_local $p0)
          (i32.const 28))
        (i32.const 0))
      (i32.store
        (get_local $p0)
        (i32.or
          (i32.load
            (get_local $p0))
          (i32.const 32)))
      (br_if $B0
        (i32.eq
          (get_local $l1)
          (i32.const 2)))
      (set_local $l3
        (i32.sub
          (get_local $p2)
          (i32.load offset=4
            (get_local $p1)))))
    (set_global $g0
      (i32.add
        (get_local $l0)
        (i32.const 16)))
    (get_local $l3))
  (func $f28 (type $t3) (param $p0 i32) (result i32)
    (local $l0 i32) (local $l1 i32) (local $l2 i32)
    (set_local $l0
      (get_local $p0))
    (block $B0
      (block $B1
        (block $B2
          (br_if $B2
            (i32.eqz
              (i32.and
                (get_local $p0)
                (i32.const 3))))
          (br_if $B1
            (i32.eqz
              (i32.load8_u
                (get_local $p0))))
          (set_local $l0
            (i32.add
              (get_local $p0)
              (i32.const 1)))
          (loop $L3
            (br_if $B2
              (i32.eqz
                (i32.and
                  (get_local $l0)
                  (i32.const 3))))
            (set_local $l1
              (i32.load8_u
                (get_local $l0)))
            (set_local $l0
              (tee_local $l2
                (i32.add
                  (get_local $l0)
                  (i32.const 1))))
            (br_if $L3
              (get_local $l1)))
          (return
            (i32.sub
              (i32.add
                (get_local $l2)
                (i32.const -1))
              (get_local $p0))))
        (set_local $l0
          (i32.add
            (get_local $l0)
            (i32.const -4)))
        (loop $L4
          (br_if $L4
            (i32.eqz
              (i32.and
                (i32.and
                  (i32.xor
                    (tee_local $l1
                      (i32.load
                        (tee_local $l0
                          (i32.add
                            (get_local $l0)
                            (i32.const 4)))))
                    (i32.const -1))
                  (i32.add
                    (get_local $l1)
                    (i32.const -16843009)))
                (i32.const -2139062144)))))
        (br_if $B0
          (i32.eqz
            (i32.and
              (get_local $l1)
              (i32.const 255))))
        (loop $L5
          (set_local $l1
            (i32.load8_u offset=1
              (get_local $l0)))
          (set_local $l0
            (tee_local $l2
              (i32.add
                (get_local $l0)
                (i32.const 1))))
          (br_if $L5
            (get_local $l1)))
        (return
          (i32.sub
            (get_local $l2)
            (get_local $p0))))
      (return
        (i32.sub
          (get_local $p0)
          (get_local $p0))))
    (i32.sub
      (get_local $l0)
      (get_local $p0)))
  (func $f29 (type $t0) (param $p0 i32) (param $p1 i32) (param $p2 i32) (result i32)
    (local $l0 i32) (local $l1 i32) (local $l2 i32) (local $l3 i32) (local $l4 i32) (local $l5 i32) (local $l6 i32) (local $l7 i32)
    (block $B0
      (block $B1
        (block $B2
          (block $B3
            (br_if $B3
              (i32.eqz
                (get_local $p2)))
            (br_if $B3
              (i32.eqz
                (i32.and
                  (get_local $p1)
                  (i32.const 3))))
            (set_local $l0
              (get_local $p0))
            (block $B4
              (loop $L5
                (i32.store8
                  (get_local $l0)
                  (i32.load8_u
                    (get_local $p1)))
                (set_local $l1
                  (i32.add
                    (get_local $p2)
                    (i32.const -1)))
                (set_local $l0
                  (i32.add
                    (get_local $l0)
                    (i32.const 1)))
                (set_local $p1
                  (i32.add
                    (get_local $p1)
                    (i32.const 1)))
                (br_if $B4
                  (i32.eq
                    (get_local $p2)
                    (i32.const 1)))
                (set_local $p2
                  (get_local $l1))
                (br_if $L5
                  (i32.and
                    (get_local $p1)
                    (i32.const 3)))))
            (br_if $B2
              (i32.eqz
                (tee_local $p2
                  (i32.and
                    (get_local $l0)
                    (i32.const 3)))))
            (br $B1))
          (set_local $l1
            (get_local $p2))
          (br_if $B1
            (tee_local $p2
              (i32.and
                (tee_local $l0
                  (get_local $p0))
                (i32.const 3)))))
        (block $B6
          (block $B7
            (br_if $B7
              (i32.lt_u
                (get_local $l1)
                (i32.const 16)))
            (set_local $p2
              (i32.add
                (get_local $l1)
                (i32.const -16)))
            (loop $L8
              (i32.store
                (get_local $l0)
                (i32.load
                  (get_local $p1)))
              (i32.store
                (i32.add
                  (get_local $l0)
                  (i32.const 4))
                (i32.load
                  (i32.add
                    (get_local $p1)
                    (i32.const 4))))
              (i32.store
                (i32.add
                  (get_local $l0)
                  (i32.const 8))
                (i32.load
                  (i32.add
                    (get_local $p1)
                    (i32.const 8))))
              (i32.store
                (i32.add
                  (get_local $l0)
                  (i32.const 12))
                (i32.load
                  (i32.add
                    (get_local $p1)
                    (i32.const 12))))
              (set_local $l0
                (i32.add
                  (get_local $l0)
                  (i32.const 16)))
              (set_local $p1
                (i32.add
                  (get_local $p1)
                  (i32.const 16)))
              (br_if $L8
                (i32.gt_u
                  (tee_local $l1
                    (i32.add
                      (get_local $l1)
                      (i32.const -16)))
                  (i32.const 15)))
              (br $B6)))
          (set_local $p2
            (get_local $l1)))
        (block $B9
          (br_if $B9
            (i32.eqz
              (i32.and
                (get_local $p2)
                (i32.const 8))))
          (i64.store align=4
            (get_local $l0)
            (i64.load align=4
              (get_local $p1)))
          (set_local $p1
            (i32.add
              (get_local $p1)
              (i32.const 8)))
          (set_local $l0
            (i32.add
              (get_local $l0)
              (i32.const 8))))
        (block $B10
          (br_if $B10
            (i32.eqz
              (i32.and
                (get_local $p2)
                (i32.const 4))))
          (i32.store
            (get_local $l0)
            (i32.load
              (get_local $p1)))
          (set_local $p1
            (i32.add
              (get_local $p1)
              (i32.const 4)))
          (set_local $l0
            (i32.add
              (get_local $l0)
              (i32.const 4))))
        (block $B11
          (br_if $B11
            (i32.eqz
              (i32.and
                (get_local $p2)
                (i32.const 2))))
          (i32.store8
            (get_local $l0)
            (i32.load8_u
              (get_local $p1)))
          (i32.store8 offset=1
            (get_local $l0)
            (i32.load8_u offset=1
              (get_local $p1)))
          (set_local $l0
            (i32.add
              (get_local $l0)
              (i32.const 2)))
          (set_local $p1
            (i32.add
              (get_local $p1)
              (i32.const 2))))
        (br_if $B0
          (i32.eqz
            (i32.and
              (get_local $p2)
              (i32.const 1))))
        (i32.store8
          (get_local $l0)
          (i32.load8_u
            (get_local $p1)))
        (return
          (get_local $p0)))
      (block $B12
        (br_if $B12
          (i32.lt_u
            (get_local $l1)
            (i32.const 32)))
        (block $B13
          (block $B14
            (br_if $B14
              (i32.eq
                (get_local $p2)
                (i32.const 3)))
            (br_if $B13
              (i32.eq
                (get_local $p2)
                (i32.const 2)))
            (br_if $B12
              (i32.ne
                (get_local $p2)
                (i32.const 1)))
            (i32.store8 offset=1
              (get_local $l0)
              (i32.load8_u offset=1
                (get_local $p1)))
            (i32.store8
              (get_local $l0)
              (tee_local $l2
                (i32.load
                  (get_local $p1))))
            (i32.store8 offset=2
              (get_local $l0)
              (i32.load8_u offset=2
                (get_local $p1)))
            (set_local $l3
              (i32.add
                (get_local $l1)
                (i32.const -3)))
            (set_local $l4
              (i32.add
                (get_local $l0)
                (i32.const 3)))
            (set_local $l5
              (i32.and
                (i32.add
                  (get_local $l1)
                  (i32.const -20))
                (i32.const -16)))
            (set_local $p2
              (i32.const 0))
            (loop $L15
              (i32.store
                (tee_local $l0
                  (i32.add
                    (get_local $l4)
                    (get_local $p2)))
                (i32.or
                  (i32.shl
                    (tee_local $l7
                      (i32.load
                        (i32.add
                          (tee_local $l6
                            (i32.add
                              (get_local $p1)
                              (get_local $p2)))
                          (i32.const 4))))
                    (i32.const 8))
                  (i32.shr_u
                    (get_local $l2)
                    (i32.const 24))))
              (i32.store
                (i32.add
                  (get_local $l0)
                  (i32.const 4))
                (i32.or
                  (i32.shl
                    (tee_local $l2
                      (i32.load
                        (i32.add
                          (get_local $l6)
                          (i32.const 8))))
                    (i32.const 8))
                  (i32.shr_u
                    (get_local $l7)
                    (i32.const 24))))
              (i32.store
                (i32.add
                  (get_local $l0)
                  (i32.const 8))
                (i32.or
                  (i32.shl
                    (tee_local $l7
                      (i32.load
                        (i32.add
                          (get_local $l6)
                          (i32.const 12))))
                    (i32.const 8))
                  (i32.shr_u
                    (get_local $l2)
                    (i32.const 24))))
              (i32.store
                (i32.add
                  (get_local $l0)
                  (i32.const 12))
                (i32.or
                  (i32.shl
                    (tee_local $l2
                      (i32.load
                        (i32.add
                          (get_local $l6)
                          (i32.const 16))))
                    (i32.const 8))
                  (i32.shr_u
                    (get_local $l7)
                    (i32.const 24))))
              (set_local $p2
                (i32.add
                  (get_local $p2)
                  (i32.const 16)))
              (br_if $L15
                (i32.gt_u
                  (tee_local $l3
                    (i32.add
                      (get_local $l3)
                      (i32.const -16)))
                  (i32.const 16))))
            (set_local $l0
              (i32.add
                (get_local $l4)
                (get_local $p2)))
            (set_local $p1
              (i32.add
                (i32.add
                  (get_local $p1)
                  (get_local $p2))
                (i32.const 3)))
            (set_local $l1
              (i32.sub
                (i32.add
                  (get_local $l1)
                  (i32.const -19))
                (get_local $l5)))
            (br $B12))
          (i32.store8
            (get_local $l0)
            (tee_local $l2
              (i32.load
                (get_local $p1))))
          (set_local $l3
            (i32.add
              (get_local $l1)
              (i32.const -1)))
          (set_local $l4
            (i32.add
              (get_local $l0)
              (i32.const 1)))
          (set_local $l5
            (i32.and
              (i32.add
                (get_local $l1)
                (i32.const -20))
              (i32.const -16)))
          (set_local $p2
            (i32.const 0))
          (loop $L16
            (i32.store
              (tee_local $l0
                (i32.add
                  (get_local $l4)
                  (get_local $p2)))
              (i32.or
                (i32.shl
                  (tee_local $l7
                    (i32.load
                      (i32.add
                        (tee_local $l6
                          (i32.add
                            (get_local $p1)
                            (get_local $p2)))
                        (i32.const 4))))
                  (i32.const 24))
                (i32.shr_u
                  (get_local $l2)
                  (i32.const 8))))
            (i32.store
              (i32.add
                (get_local $l0)
                (i32.const 4))
              (i32.or
                (i32.shl
                  (tee_local $l2
                    (i32.load
                      (i32.add
                        (get_local $l6)
                        (i32.const 8))))
                  (i32.const 24))
                (i32.shr_u
                  (get_local $l7)
                  (i32.const 8))))
            (i32.store
              (i32.add
                (get_local $l0)
                (i32.const 8))
              (i32.or
                (i32.shl
                  (tee_local $l7
                    (i32.load
                      (i32.add
                        (get_local $l6)
                        (i32.const 12))))
                  (i32.const 24))
                (i32.shr_u
                  (get_local $l2)
                  (i32.const 8))))
            (i32.store
              (i32.add
                (get_local $l0)
                (i32.const 12))
              (i32.or
                (i32.shl
                  (tee_local $l2
                    (i32.load
                      (i32.add
                        (get_local $l6)
                        (i32.const 16))))
                  (i32.const 24))
                (i32.shr_u
                  (get_local $l7)
                  (i32.const 8))))
            (set_local $p2
              (i32.add
                (get_local $p2)
                (i32.const 16)))
            (br_if $L16
              (i32.gt_u
                (tee_local $l3
                  (i32.add
                    (get_local $l3)
                    (i32.const -16)))
                (i32.const 18))))
          (set_local $l0
            (i32.add
              (get_local $l4)
              (get_local $p2)))
          (set_local $p1
            (i32.add
              (i32.add
                (get_local $p1)
                (get_local $p2))
              (i32.const 1)))
          (set_local $l1
            (i32.sub
              (i32.add
                (get_local $l1)
                (i32.const -17))
              (get_local $l5)))
          (br $B12))
        (i32.store8
          (get_local $l0)
          (tee_local $l2
            (i32.load
              (get_local $p1))))
        (i32.store8 offset=1
          (get_local $l0)
          (i32.load8_u offset=1
            (get_local $p1)))
        (set_local $l3
          (i32.add
            (get_local $l1)
            (i32.const -2)))
        (set_local $l4
          (i32.add
            (get_local $l0)
            (i32.const 2)))
        (set_local $l5
          (i32.and
            (i32.add
              (get_local $l1)
              (i32.const -20))
            (i32.const -16)))
        (set_local $p2
          (i32.const 0))
        (loop $L17
          (i32.store
            (tee_local $l0
              (i32.add
                (get_local $l4)
                (get_local $p2)))
            (i32.or
              (i32.shl
                (tee_local $l7
                  (i32.load
                    (i32.add
                      (tee_local $l6
                        (i32.add
                          (get_local $p1)
                          (get_local $p2)))
                      (i32.const 4))))
                (i32.const 16))
              (i32.shr_u
                (get_local $l2)
                (i32.const 16))))
          (i32.store
            (i32.add
              (get_local $l0)
              (i32.const 4))
            (i32.or
              (i32.shl
                (tee_local $l2
                  (i32.load
                    (i32.add
                      (get_local $l6)
                      (i32.const 8))))
                (i32.const 16))
              (i32.shr_u
                (get_local $l7)
                (i32.const 16))))
          (i32.store
            (i32.add
              (get_local $l0)
              (i32.const 8))
            (i32.or
              (i32.shl
                (tee_local $l7
                  (i32.load
                    (i32.add
                      (get_local $l6)
                      (i32.const 12))))
                (i32.const 16))
              (i32.shr_u
                (get_local $l2)
                (i32.const 16))))
          (i32.store
            (i32.add
              (get_local $l0)
              (i32.const 12))
            (i32.or
              (i32.shl
                (tee_local $l2
                  (i32.load
                    (i32.add
                      (get_local $l6)
                      (i32.const 16))))
                (i32.const 16))
              (i32.shr_u
                (get_local $l7)
                (i32.const 16))))
          (set_local $p2
            (i32.add
              (get_local $p2)
              (i32.const 16)))
          (br_if $L17
            (i32.gt_u
              (tee_local $l3
                (i32.add
                  (get_local $l3)
                  (i32.const -16)))
              (i32.const 17))))
        (set_local $l0
          (i32.add
            (get_local $l4)
            (get_local $p2)))
        (set_local $p1
          (i32.add
            (i32.add
              (get_local $p1)
              (get_local $p2))
            (i32.const 2)))
        (set_local $l1
          (i32.sub
            (i32.add
              (get_local $l1)
              (i32.const -18))
            (get_local $l5))))
      (block $B18
        (br_if $B18
          (i32.eqz
            (i32.and
              (get_local $l1)
              (i32.const 16))))
        (i32.store16 align=1
          (get_local $l0)
          (i32.load16_u align=1
            (get_local $p1)))
        (i32.store8 offset=2
          (get_local $l0)
          (i32.load8_u offset=2
            (get_local $p1)))
        (i32.store8 offset=3
          (get_local $l0)
          (i32.load8_u offset=3
            (get_local $p1)))
        (i32.store8 offset=4
          (get_local $l0)
          (i32.load8_u offset=4
            (get_local $p1)))
        (i32.store8 offset=5
          (get_local $l0)
          (i32.load8_u offset=5
            (get_local $p1)))
        (i32.store8 offset=6
          (get_local $l0)
          (i32.load8_u offset=6
            (get_local $p1)))
        (i32.store8 offset=7
          (get_local $l0)
          (i32.load8_u offset=7
            (get_local $p1)))
        (i32.store8 offset=8
          (get_local $l0)
          (i32.load8_u offset=8
            (get_local $p1)))
        (i32.store8 offset=9
          (get_local $l0)
          (i32.load8_u offset=9
            (get_local $p1)))
        (i32.store8 offset=10
          (get_local $l0)
          (i32.load8_u offset=10
            (get_local $p1)))
        (i32.store8 offset=11
          (get_local $l0)
          (i32.load8_u offset=11
            (get_local $p1)))
        (i32.store8 offset=12
          (get_local $l0)
          (i32.load8_u offset=12
            (get_local $p1)))
        (i32.store8 offset=13
          (get_local $l0)
          (i32.load8_u offset=13
            (get_local $p1)))
        (i32.store8 offset=14
          (get_local $l0)
          (i32.load8_u offset=14
            (get_local $p1)))
        (i32.store8 offset=15
          (get_local $l0)
          (i32.load8_u offset=15
            (get_local $p1)))
        (set_local $l0
          (i32.add
            (get_local $l0)
            (i32.const 16)))
        (set_local $p1
          (i32.add
            (get_local $p1)
            (i32.const 16))))
      (block $B19
        (br_if $B19
          (i32.eqz
            (i32.and
              (get_local $l1)
              (i32.const 8))))
        (i32.store8
          (get_local $l0)
          (i32.load8_u
            (get_local $p1)))
        (i32.store8 offset=1
          (get_local $l0)
          (i32.load8_u offset=1
            (get_local $p1)))
        (i32.store8 offset=2
          (get_local $l0)
          (i32.load8_u offset=2
            (get_local $p1)))
        (i32.store8 offset=3
          (get_local $l0)
          (i32.load8_u offset=3
            (get_local $p1)))
        (i32.store8 offset=4
          (get_local $l0)
          (i32.load8_u offset=4
            (get_local $p1)))
        (i32.store8 offset=5
          (get_local $l0)
          (i32.load8_u offset=5
            (get_local $p1)))
        (i32.store8 offset=6
          (get_local $l0)
          (i32.load8_u offset=6
            (get_local $p1)))
        (i32.store8 offset=7
          (get_local $l0)
          (i32.load8_u offset=7
            (get_local $p1)))
        (set_local $l0
          (i32.add
            (get_local $l0)
            (i32.const 8)))
        (set_local $p1
          (i32.add
            (get_local $p1)
            (i32.const 8))))
      (block $B20
        (br_if $B20
          (i32.eqz
            (i32.and
              (get_local $l1)
              (i32.const 4))))
        (i32.store8
          (get_local $l0)
          (i32.load8_u
            (get_local $p1)))
        (i32.store8 offset=1
          (get_local $l0)
          (i32.load8_u offset=1
            (get_local $p1)))
        (i32.store8 offset=2
          (get_local $l0)
          (i32.load8_u offset=2
            (get_local $p1)))
        (i32.store8 offset=3
          (get_local $l0)
          (i32.load8_u offset=3
            (get_local $p1)))
        (set_local $l0
          (i32.add
            (get_local $l0)
            (i32.const 4)))
        (set_local $p1
          (i32.add
            (get_local $p1)
            (i32.const 4))))
      (block $B21
        (br_if $B21
          (i32.eqz
            (i32.and
              (get_local $l1)
              (i32.const 2))))
        (i32.store8
          (get_local $l0)
          (i32.load8_u
            (get_local $p1)))
        (i32.store8 offset=1
          (get_local $l0)
          (i32.load8_u offset=1
            (get_local $p1)))
        (set_local $l0
          (i32.add
            (get_local $l0)
            (i32.const 2)))
        (set_local $p1
          (i32.add
            (get_local $p1)
            (i32.const 2))))
      (br_if $B0
        (i32.eqz
          (i32.and
            (get_local $l1)
            (i32.const 1))))
      (i32.store8
        (get_local $l0)
        (i32.load8_u
          (get_local $p1))))
    (get_local $p0))
  (table $T0 5 5 anyfunc)
  (memory $memory 2)
  (global $g0 (mut i32) (i32.const 67792))
  (global $__heap_base i32 (i32.const 67792))
  (global $__data_end i32 (i32.const 2244))
  (export "memory" (memory 0))
  (export "__heap_base" (global 1))
  (export "__data_end" (global 2))
  (export "_start" (func $_start))
  (elem (i32.const 1) $f27 $f13 $f15 $f17)
  (data (i32.const 1024) "hello, wasi!\00")
  (data (i32.const 1040) "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00")
  (data (i32.const 2096) "\05\00\00\00\00\00\00\00\00\00\00\00\02\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\03\00\00\00\04\00\00\00(\04\00\00\00\04\00\00\00\00\00\00\00\00\00\00\01\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\ff\ff\ff\ff\0a\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\000\08\00\00"))
