(module
  (func $local_set (result i32)
    (local $x i32)
    (local.set $x (i32.const 42))
    (local.get $x)
  )
  (export "local_set" (func $local_set))
)
