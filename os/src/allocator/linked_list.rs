use core::{
    alloc::{GlobalAlloc, Layout},
    mem, ptr,
};

use super::{align_up, Locked};

struct ListNode {
    size: usize,
    next: Option<&'static mut ListNode>, // Box と同じ
}

impl ListNode {
    const fn new(size: usize) -> Self {
        ListNode { size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

pub struct LinkedListAllocator {
    head: ListNode,
}

impl LinkedListAllocator {
    /// 空のLinkedListAllocatorを作る。
    pub const fn new() -> Self {
        Self {
            head: ListNode::new(0),
        }
    }

    /// 与えられたヒープ境界でアロケータを初期化する。
    ///
    /// この関数はunsafeである。なぜなら、呼び出し元は渡すヒープ境界が
    /// 有効でヒープが未使用であることを保証しなければならないからである。
    /// このメソッドは一度しか呼ばれてはならない。
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_free_region(heap_start, heap_size);
    }

    /// 与えられたメモリ領域をリストの先頭に追加する。
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // 解放された領域がListNodeを格納できることを確かめる
        assert_eq!(align_up(addr, mem::align_of::<ListNode>()), addr);
        assert!(size >= mem::size_of::<ListNode>());

        // 新しいリストノードを作り、それをリストの先頭に追加する
        let mut node = ListNode::new(size);
        node.next = self.head.next.take();
        let node_ptr = addr as *mut ListNode;
        node_ptr.write(node);
        self.head.next = Some(&mut *node_ptr)
    }

    /// 与えられたサイズの解放された領域を探し、リストからそれを取り除く。
    ///
    /// リストノードと割り当ての開始アドレスからなるタプルを返す。
    fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut ListNode, usize)> {
        // 現在のリストノードへの参照。繰り返しごとに更新していく
        let mut current = &mut self.head;
        // 連結リストから十分大きな領域を探す
        while let Some(ref mut region) = current.next {
            if let Ok(alloc_start) = Self::alloc_from_region(&region, size, align) {
                // 領域が割り当てに適している -> リストから除く
                let next = region.next.take();
                let ret = Some((current.next.take().unwrap(), alloc_start));
                current.next = next;
                return ret;
            } else {
                // 割り当てに適していない -> 次の領域で繰り返す
                current = current.next.as_mut().unwrap();
            }
        }

        // 適した領域が見つからなかった
        None
    }

    /// 与えられた領域で与えられたサイズとアラインメントの割り当てを行おうとする。
    ///
    /// 成功した場合、割り当ての開始アドレスを返す。
    fn alloc_from_region(region: &ListNode, size: usize, align: usize) -> Result<usize, ()> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            // 領域が小さすぎる
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<ListNode>() {
            // 領域の残りが小さすぎてListNodeを格納できない
            // (割り当ては領域を使用部と解放部に分けるので、この条件が必要)
            return Err(());
        }

        // 領域は割り当てに適している
        Ok(alloc_start)
    }

    /// 与えられたレイアウトを調整し、割り当てられるメモリ領域が`ListNode`を格納することもできるようにする。
    ///
    /// 調整されたサイズとアラインメントをタプルとして返す。
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<ListNode>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<ListNode>());
        (size, layout.align())
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // レイアウト調整を行う
        let (size, align) = LinkedListAllocator::size_align(layout);
        let mut allocator = self.lock();

        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            let alloc_end = alloc_start.checked_add(size).expect("overflow");
            let excess_size = region.end_addr() - alloc_end;
            if excess_size > 0 {
                // 空いている領域からメモリを確保したあとの残りを再度リストに追加する
                allocator.add_free_region(alloc_end, excess_size);
            }
            alloc_start as *mut u8
        } else {
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // レイアウト調整を行う
        let (size, _) = LinkedListAllocator::size_align(layout);

        self.lock().add_free_region(ptr as usize, size)
    }
}
