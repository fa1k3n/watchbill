//! An interrupt (Ctrl-C) while `wb init` asks: it says `Nothing written.` and ends with 1. While a session
//! that `wb <role>` started runs, an interrupt and a quit go to the session, and `wb` waits for it (`hold`).
//!
//! A Rust program is ended by SIGINT unless it handles the signal. The handler uses only what a
//! handler may: `write(2)` and `_exit(2)`. They are declared here, for the C library that the
//! standard library is linked with already, so no crate is needed. Standard output of `wb` is not
//! buffered, so what the handler writes comes after all that was printed.

use std::os::raw::{c_int, c_void};
use std::sync::atomic::{AtomicBool, Ordering};

extern "C" {
    fn signal(signal: c_int, handler: usize) -> usize;
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

/// `SIG_IGN` as the C library has it.
const IGNORED: usize = 1;

/// From now on an interrupt ends the program with `Nothing written.` and 1, unless the parent had set
/// SIGINT to be ignored (`nohup`, `trap '' INT`): Python keeps it ignored then, and so does this.
pub fn install() {
    unsafe {
        if signal(SIGINT, on_interrupt as extern "C" fn(c_int) as usize) == IGNORED {
            signal(SIGINT, IGNORED);
        }
    }
}

/// The file is about to be written: an interrupt after this is not acted on.
pub fn writing() {
    WRITING.store(true, Ordering::SeqCst);
}

const SIGQUIT: c_int = 3;

/// A handler that does nothing. A signal that is caught, not ignored, is set back to its default in a
/// program that is started, so the session gets an interrupt as it would without `wb`.
extern "C" fn on_nothing(_: c_int) {}

/// How many sessions hold the signals now, and what each signal was before the first: tests start sessions
/// from several threads of one process, and only the last to end puts the signals back.
static HELD: std::sync::Mutex<(usize, [usize; 2])> = std::sync::Mutex::new((0, [0, 0]));

/// While it is kept, an interrupt (SIGINT) or a quit (SIGQUIT) does not end `wb` (#84): at a terminal both go
/// to every program of the foreground, the session too, and `wb` waits for the session to end and gives its
/// status. A signal that was ignored stays ignored, as `install` keeps it.
pub struct Held;

pub fn hold() -> Held {
    let mut held = HELD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if held.0 == 0 {
        for (index, number) in [SIGINT, SIGQUIT].into_iter().enumerate() {
            unsafe {
                let before = signal(number, on_nothing as extern "C" fn(c_int) as usize);
                if before == IGNORED {
                    signal(number, IGNORED);
                }
                held.1[index] = before;
            }
        }
    }
    held.0 += 1;
    Held
}

impl Drop for Held {
    fn drop(&mut self) {
        let mut held = HELD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        held.0 -= 1;
        if held.0 == 0 {
            for (index, number) in [SIGINT, SIGQUIT].into_iter().enumerate() {
                unsafe {
                    signal(number, held.1[index]);
                }
            }
        }
    }
}
