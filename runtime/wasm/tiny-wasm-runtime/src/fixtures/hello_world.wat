(module
  (import "wasi_snapshot_preview1" "fd_write"
    ;; 引数１つ目: ファイルディスクリプタ (1: stdout) (2: stderr)
    ;; 引数２つ目: メモリの読み取り開始位置
    ;; 引数３つ目: メモリの読み取り回数 (バイト単位で読み出す)
    ;; 引数４つ目: 書き込みバイト数を保存先を示すメモリの位置
    (func $fd_write (param i32 i32 i32 i32) (result i32))
  )
  (memory 1)
  ;; アドレスが 0 の箇所から "Hello, World!\n" を格納
  (data (i32.const 0) "Hello, World!\n")

  (func $hello_world (result i32)
    (local $iovs i32)

    ;; stdout に書き出すデータのメモリ上の先頭アドレスの値を登録
    ;; 先頭から書き出したいので 0 を指定
    (i32.store (i32.const 16) (i32.const 0))
    ;; stdout に書き出すデータのバイト数の値を登録
    ;; "Hello, World!\n" を書き出すので 14 を指定
    (i32.store (i32.const 20) (i32.const 14))

    ;; fd_write が読み出すメモリの読み取り開始位置を登録
    (local.set $iovs (i32.const 16))

    (call $fd_write
      (i32.const 1)
      (local.get $iovs)
      (i32.const 1)
      (i32.const 24)
    )
  )

  (export "_start" (func $hello_world))
)
