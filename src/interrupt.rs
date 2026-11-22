//! An interrupt (Ctrl-C) while `wb init` asks: it says `Nothing written.` and ends with 1. While a session
//! that `wb start <role>` started runs, an interrupt, a quit and a hangup go to the session, a termination is sent on to it, and `wb` waits for it (`hold`).
//!
//! A Rust program is ended by SIGINT unless it handles the signal. The handler uses only what a
//! handler may: `write(2)` and `_exit(2)`. They are declared here, for the C library that the
//! standard library is linked with already, so no crate is needed. Standard output of `wb` is not
//! buffered, so what the handler writes comes after all that was printed.

use std::os::raw::{c_int, c_void};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

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
const SIGHUP: c_int = 1;
const SIGTERM: c_int = 15;

/// The signals held while a session runs.
const HELD_SIGNALS: [c_int; 4] = [SIGINT, SIGQUIT, SIGHUP, SIGTERM];

/// A handler that does nothing. A signal that is caught, not ignored, is set back to its default in a
/// program that is started, so the session gets an interrupt as it would without `wb`.
extern "C" fn on_nothing(_: c_int) {}

extern "C" {
    fn kill(pid: c_int, signal: c_int) -> c_int;
}

/// The session that a termination is sent on to, or 0 when there is none yet.
static SESSION: AtomicI32 = AtomicI32::new(0);
/// Set when a termination came: a session that is registered after it is sent it then.
static TERMINATED: AtomicBool = AtomicBool::new(false);

/// A termination (#221): `wb` goes on waiting, and the session is sent the termination, as a person's `kill` of
/// `wb` meant it. Only `kill(2)` and atomics are used, as a handler may.
extern "C" fn on_termination(_: c_int) {
    TERMINATED.store(true, Ordering::SeqCst);
    let session = SESSION.load(Ordering::SeqCst);
    if session > 0 {
        unsafe {
            kill(session, SIGTERM);
        }
    }
}

/// How many sessions hold the signals now, and what each signal was before the first: tests start sessions
/// from several threads of one process, and only the last to end puts the signals back.
static HELD: std::sync::Mutex<(usize, [usize; 4])> = std::sync::Mutex::new((0, [0; 4]));

/// While it is kept, an interrupt (SIGINT), a quit (SIGQUIT), a hangup (SIGHUP) or a termination (SIGTERM) does
/// not end `wb` (#84, #221): at a terminal an interrupt, a quit and a hangup go to every program of the
/// foreground, the session too, and `wb` waits for the session to end and gives its status. A termination goes
/// to `wb` alone, and `wb` sends it on to the session (`session`). A signal that was ignored stays ignored, as
/// `install` keeps it.
pub struct Held;

pub fn hold() -> Held {
    let mut held = HELD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if held.0 == 0 {
        TERMINATED.store(false, Ordering::SeqCst);
        SESSION.store(0, Ordering::SeqCst);
        for (index, number) in HELD_SIGNALS.into_iter().enumerate() {
            let handler = if number == SIGTERM { on_termination as extern "C" fn(c_int) as usize } else { on_nothing as extern "C" fn(c_int) as usize };
            unsafe {
                let before = signal(number, handler);
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

impl Held {
    /// The session was started, with process number `pid`: a termination of `wb` is sent on to it, also one that
    /// came before this.
    pub fn session(&self, pid: u32) {
        SESSION.store(pid as c_int, Ordering::SeqCst);
        if TERMINATED.load(Ordering::SeqCst) {
            unsafe {
                kill(pid as c_int, SIGTERM);
            }
        }
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        let mut held = HELD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        held.0 -= 1;
        if held.0 == 0 {
            SESSION.store(0, Ordering::SeqCst);
            for (index, number) in HELD_SIGNALS.into_iter().enumerate() {
                unsafe {
                    signal(number, held.1[index]);
                }
            }
        }
    }
}
