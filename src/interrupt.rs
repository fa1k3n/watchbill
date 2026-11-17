//! An interrupt (Ctrl-C) while `wb init` asks: it says `Nothing written.` and ends with 1.
//!
//! A Rust program is ended by SIGINT unless it handles the signal. The handler uses only what a
//! handler may: `write(2)` and `_exit(2)`. They are declared here, for the C library that the
//! standard library is linked with already, so no crate is needed. Standard output of `wb` is not
//! buffered, so what the handler writes comes after all that was printed.

use std::os::raw::{c_int, c_void};
use std::sync::atomic::{AtomicBool, Ordering};

extern "C" {
    fn signal(signal: c_int, handler: extern "C" fn(c_int)) -> usize;
    fn write(fd: c_int, buffer: *const c_void, count: usize) -> isize;
    fn _exit(status: c_int) -> !;
}

const SIGINT: c_int = 2;
const MESSAGE: &[u8] = b"\nNothing written.\n";

/// Set while the file is written: from then on it is no longer true that nothing is written.
static WRITING: AtomicBool = AtomicBool::new(false);

extern "C" fn on_interrupt(_: c_int) {
    if !WRITING.load(Ordering::SeqCst) {
        unsafe {
            write(1, MESSAGE.as_ptr() as *const c_void, MESSAGE.len());
            _exit(1);
        }
    }
}

/// From now on an interrupt ends the program with `Nothing written.` and 1.
pub fn install() {
    unsafe {
        signal(SIGINT, on_interrupt);
    }
}

/// The file is about to be written: an interrupt after this is not acted on.
pub fn writing() {
    WRITING.store(true, Ordering::SeqCst);
}
