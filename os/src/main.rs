#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(blog_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

mod serial;
mod vga_buffer;
use core::panic::PanicInfo;

// この関数はパニック時に呼ばれる
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    blog_os::test_panic_handler(info)
}

// リンカはデフォルトで `_start` という名前の関数を探すので、
// この関数がエントリポイントとなる
#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello World{}", "!");
    blog_os::init();

    fn stack_overflow() {
        stack_overflow(); // 再帰呼び出しのために、リターンアドレスがプッシュされる
    }

    // スタックオーバーフローを起こす
    stack_overflow();

    #[cfg(test)]
    test_main();

    loop {}
}
