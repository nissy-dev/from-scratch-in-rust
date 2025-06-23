(module
  (func $a (param i32 i64))
  (func $b (param i64 i32) (result i32 i64)
    (local.get 1)
    (local.get 0)
  )
  (func $c (param i32 i64))
)
