use crate::print;
use crate::println;
use conquer_once::spin::OnceCell;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::stream::StreamExt;
use futures_util::{task::AtomicWaker, Stream};
use pc_keyboard::HandleControl;
use pc_keyboard::ScancodeSet1;
use pc_keyboard::{layouts, DecodedKey, Keyboard};

static WAKER: AtomicWaker = AtomicWaker::new();
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

/// キーボード割り込みハンドラから呼び出される
///
/// 処理をブロックしたり、アロケートをしてはいけない
pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            println!("WARNING: scancode queue full; dropping keyboard input");
            //       "警告：スキャンコードキューがいっぱいです。キーボード入力を取り零しています"
        } else {
            // wake_task が呼ばれ、queue にタスクが追加される
            WAKER.wake(); // Wake する
        }
    } else {
        println!("WARNING: scancode queue uninitialized");
        //       "警告：スキャンコードキューが初期化されていません"
    }
}

pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        //      "ScancodeStream::new は一度しか呼び出されてはなりません"
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE.try_get().expect("not initialized");

        if let Ok(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(&cx.waker());
        match queue.pop() {
            Ok(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            Err(crossbeam_queue::PopError) => Poll::Pending,
        }
    }
}

pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard: Keyboard<_, _> =
        Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore);

    while let Some(scancode) = scancodes.next().await {
        // print!("hello {:?}", scancode);
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => print!("{}", character),
                    DecodedKey::RawKey(key) => print!("{:?}", key),
                }
            }
        }
    }
}
