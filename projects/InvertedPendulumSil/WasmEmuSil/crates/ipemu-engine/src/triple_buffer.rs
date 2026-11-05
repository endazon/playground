//! スケジューラ → I/O のスナップショット受け渡し用トリプルバッファ。
//!
//! 書き手 (スケジューラスレッド) は決して待たず、読み手 (tokio) は常に最新の完成した値だけを見る。
//! 中間スロットの index と「未読」ビットを 1 つの原子変数に畳み込む古典的な実装。

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

const DIRTY: u8 = 0b100;
const INDEX: u8 = 0b011;

struct Shared<T> {
    slots: [UnsafeCell<T>; 3],
    /// 下位 2 bit: 中間スロットの index、bit2: 読み手がまだ取っていない
    middle: AtomicU8,
}

// 書き手と読み手がそれぞれ別のスロットしか触らないことを `middle` の交換で保証する
unsafe impl<T: Send> Send for Shared<T> {}
unsafe impl<T: Send> Sync for Shared<T> {}

pub struct Writer<T> {
    shared: Arc<Shared<T>>,
    back: u8,
}

pub struct Reader<T> {
    shared: Arc<Shared<T>>,
    front: u8,
}

/// 3 スロットを `init` の複製で初期化して (writer, reader) を返す。
pub fn triple_buffer<T: Clone + Send>(init: T) -> (Writer<T>, Reader<T>) {
    let shared = Arc::new(Shared {
        slots: [
            UnsafeCell::new(init.clone()),
            UnsafeCell::new(init.clone()),
            UnsafeCell::new(init),
        ],
        middle: AtomicU8::new(1),
    });
    (
        Writer {
            shared: shared.clone(),
            back: 0,
        },
        Reader { shared, front: 2 },
    )
}

impl<T> Writer<T> {
    /// 裏スロットへの可変参照。`publish` するまで読み手には見えない
    pub fn back_mut(&mut self) -> &mut T {
        // SAFETY: back は書き手だけが保持する index
        unsafe { &mut *self.shared.slots[self.back as usize].get() }
    }

    /// 裏スロットを中間に差し出し、古い中間を次の裏にする。
    pub fn publish(&mut self) {
        let old = self.shared.middle.swap(self.back | DIRTY, Ordering::AcqRel);
        self.back = old & INDEX;
    }

    pub fn write(&mut self, value: T) {
        *self.back_mut() = value;
        self.publish();
    }
}

impl<T> Reader<T> {
    /// 未読の値があれば表スロットを更新して true。
    pub fn update(&mut self) -> bool {
        if self.shared.middle.load(Ordering::Acquire) & DIRTY == 0 {
            return false;
        }
        let old = self.shared.middle.swap(self.front, Ordering::AcqRel);
        self.front = old & INDEX;
        true
    }

    /// 表スロット (直近に `update` で取り込んだ値)
    pub fn front(&self) -> &T {
        // SAFETY: front は読み手だけが保持する index
        unsafe { &*self.shared.slots[self.front as usize].get() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reader_sees_latest_only() {
        let (mut w, mut r) = triple_buffer(0u64);
        assert!(!r.update());
        w.write(1);
        w.write(2);
        w.write(3);
        assert!(r.update());
        assert_eq!(*r.front(), 3);
        assert!(!r.update());
        assert_eq!(*r.front(), 3);
    }

    #[test]
    fn concurrent_writes_never_tear() {
        let (mut w, mut r) = triple_buffer([0u64; 4]);
        let h = std::thread::spawn(move || {
            for i in 1..200_000u64 {
                w.write([i; 4]);
            }
        });
        let mut seen = 0;
        while !h.is_finished() {
            if r.update() {
                let f = r.front();
                assert!(f.iter().all(|&v| v == f[0]), "tearing: {f:?}");
                seen += 1;
            }
        }
        h.join().unwrap();
        r.update();
        assert_eq!(*r.front(), [199_999; 4]);
        assert!(seen > 0);
    }
}
