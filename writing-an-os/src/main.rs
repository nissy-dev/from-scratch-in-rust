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

// static HELLO: &[u8] = b"Hello World!";

// リンカはデフォルトで `_start` という名前の関数を探すので、
// この関数がエントリポイントとなる
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // let vga_buffer = 0xb8000 as *mut u8;
    // for (i, &byte) in HELLO.iter().enumerate() {
    //     unsafe {
    //         let line_offset: isize = 160 * 2;
    //         let char_offset_within_line: isize = i as isize * 2;
    //         let color_offset_within_line: isize = i as isize * 2 + 1;
    //         let char_offset = char_offset_within_line + line_offset;
    //         let color_offset = color_offset_within_line + line_offset;
    //         *vga_buffer.offset(char_offset) = byte;
    //         *vga_buffer.offset(color_offset) = 0xb;
    //     }
    // }

    // use core::fmt::Write;
    // vga_buffer::WRITER.lock().write_str("Hello again").unwrap();
    // write!(
    //     vga_buffer::WRITER.lock(),
    //     ", some numbers: {} {}",
    //     42,
    //     1.337
    // )
    // .unwrap();

    println!("Hello World{}", "!");
    // panic!("Some panic message");

    #[cfg(test)]
    test_main();

    loop {}
}
