//! An interrupt (Ctrl-C) while `wb init` asks: it says `Nothing written.` and ends with 1. While a session
//! that `wb start <role>` started runs, an interrupt, a quit and a hangup go to the session, a termination is sent on to it, and `wb` waits for it (`hold`).
//! In a view the session is inside bubblewrap: bubblewrap ignores the hangup and the termination, and `wb` sends them on
//! to the session itself, held by a pidfd (#257).
//!
//! A Rust program is ended by SIGINT unless it handles the signal. The handler uses only what a
//! handler may: `write(2)` and `_exit(2)`. They are declared here, for the C library that the
//! standard library is linked with already, so no crate is needed. Standard output of `wb` is not
//! buffered, so what the handler writes comes after all that was printed.

use std::os::raw::{c_int, c_long, c_void};
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
pub(crate) const SIGHUP: c_int = 1;
pub(crate) const SIGTERM: c_int = 15;

/// The signals held while a session runs.
const HELD_SIGNALS: [c_int; 4] = [SIGINT, SIGQUIT, SIGHUP, SIGTERM];

/// A handler that does nothing. A signal that is caught, not ignored, is set back to its default in a
/// program that is started, so the session gets an interrupt as it would without `wb`.
extern "C" fn on_nothing(_: c_int) {}

extern "C" {
    fn kill(pid: c_int, signal: c_int) -> c_int;
    /// The C library of Linux: the place of `errno` of this thread. The standard library can read it, not set it.
    fn __errno_location() -> *mut c_int;
    /// A call of the kernel by its number: `pidfd_open` and `pidfd_send_signal`, which the C library of Enterprise
    /// Linux 9 has no function for.
    fn syscall(number: c_long, ...) -> c_long;
    fn fcntl(fd: c_int, command: c_int, ...) -> c_int;
}

/// The numbers of the two calls, the same on every architecture Linux has added them to since 5.1.
const PIDFD_SEND_SIGNAL: c_long = 424;
const PIDFD_OPEN: c_long = 434;

/// The session that a signal is sent on to, by its number, or 0 when there is none yet.
static SESSION: AtomicI32 = AtomicI32::new(0);
/// The session that a signal is sent on to, by a pidfd, which no other process can come to have; -1 when there
/// is none. Where there is one, it is used and not `SESSION`.
static SESSION_FD: AtomicI32 = AtomicI32::new(-1);
/// Set when a termination came: a session that is registered after it is sent it then.
static TERMINATED: AtomicBool = AtomicBool::new(false);
/// Set when a hangup came.
static HUNG_UP: AtomicBool = AtomicBool::new(false);
/// Whether a hangup is sent on to the session too: only where it has no controlling terminal (#257).
static HANGUPS: AtomicBool = AtomicBool::new(false);

/// Sends `signal` to the session, when there is one. Only atomics, `kill(2)` and the call of the kernel.
fn send(signal: c_int) {
    let fd = SESSION_FD.load(Ordering::SeqCst);
    if fd >= 0 {
        unsafe {
            syscall(PIDFD_SEND_SIGNAL, fd as c_long, signal as c_long, 0 as c_long, 0 as c_long);
        }
        return;
    }
    let session = SESSION.load(Ordering::SeqCst);
    if session > 0 {
        unsafe {
            kill(session, signal);
        }
    }
}

/// A termination (#221), or a hangup: `wb` goes on waiting. The session is sent the termination, as a person's
/// `kill` of `wb` meant it, and the hangup where the terminal cannot reach it (#257). Only `kill(2)`, the call
/// of the kernel and atomics are used, as a handler may.
extern "C" fn on_signal(number: c_int) {
    // `kill` sets `errno` when it fails, and the program that was interrupted may be reading it: put it back.
    let errno = unsafe { __errno_location() };
    let saved = unsafe { *errno };
    if number == SIGTERM {
        TERMINATED.store(true, Ordering::SeqCst);
        send(SIGTERM);
    } else if number == SIGHUP {
        HUNG_UP.store(true, Ordering::SeqCst);
        if HANGUPS.load(Ordering::SeqCst) {
            send(SIGHUP);
        }
    }
    unsafe { *errno = saved };
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
        HUNG_UP.store(false, Ordering::SeqCst);
        HANGUPS.store(false, Ordering::SeqCst);
        SESSION.store(0, Ordering::SeqCst);
        SESSION_FD.store(-1, Ordering::SeqCst);
        for (index, number) in HELD_SIGNALS.into_iter().enumerate() {
            let handler = match number {
                SIGTERM | SIGHUP => on_signal as extern "C" fn(c_int) as usize,
                _ => on_nothing as extern "C" fn(c_int) as usize,
            };
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
    /// The session has been waited for: a signal is no longer sent to its number, which may be another's now.
    pub fn ended(&self) {
        SESSION_FD.store(-1, Ordering::SeqCst);
        SESSION.store(0, Ordering::SeqCst);
        HANGUPS.store(false, Ordering::SeqCst);
    }

    /// The session was started, with process number `pid`: a termination of `wb` is sent on to it, also one that
    /// came before this.
    pub fn session(&self, pid: u32) {
        SESSION.store(pid as c_int, Ordering::SeqCst);
        self.pending();
    }

    /// The session in a view was found, and is held by the pidfd `fd` (#257): a signal of `wb` is sent on to it
    /// through `fd`, also one that came before this. `fd` stays open until after `ended`.
    pub fn session_held(&self, fd: &std::os::fd::OwnedFd) {
        use std::os::fd::AsRawFd;
        SESSION_FD.store(fd.as_raw_fd(), Ordering::SeqCst);
        self.pending();
    }

    /// From now on a hangup of `wb` is sent on to the session too, also one that came before: the session has no
    /// controlling terminal, so the terminal's own hangup cannot reach it (#257).
    pub fn hangups_sent_on(&self) {
        HANGUPS.store(true, Ordering::SeqCst);
    }

    /// Sends what came before the session was registered.
    fn pending(&self) {
        if TERMINATED.load(Ordering::SeqCst) {
            send(SIGTERM);
        }
        if HANGUPS.load(Ordering::SeqCst) && HUNG_UP.load(Ordering::SeqCst) {
            send(SIGHUP);
        }
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        let mut held = HELD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        held.0 -= 1;
        if held.0 == 0 {
            SESSION_FD.store(-1, Ordering::SeqCst);
            SESSION.store(0, Ordering::SeqCst);
            HANGUPS.store(false, Ordering::SeqCst);
            for (index, number) in HELD_SIGNALS.into_iter().enumerate() {
                unsafe {
                    signal(number, held.1[index]);
                }
            }
        }
    }
}

/// Whether `number` (SIGHUP or SIGTERM) was ignored when the first hold began, as by `nohup`: the session is
/// then started with it ignored too.
pub fn was_ignored(number: c_int) -> bool {
    let held = HELD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    match HELD_SIGNALS.iter().position(|one| *one == number) {
        Some(index) => held.0 > 0 && held.1[index] == IGNORED,
        None => false,
    }
}

/// What the child does between fork and exec, before it becomes bubblewrap (#257): SIGHUP and SIGTERM are
/// ignored, where `ignore` says so, so that bubblewrap is not ended by them, and `fd` is kept open across the
/// exec. Only `signal(2)` and `fcntl(2)`, which may be called there.
pub fn before_bubblewrap(fd: c_int, ignore: bool) -> std::io::Result<()> {
    const F_SETFD: c_int = 2;
    if ignore {
        unsafe {
            signal(SIGHUP, IGNORED);
            signal(SIGTERM, IGNORED);
        }
    }
    if unsafe { fcntl(fd, F_SETFD, 0 as c_int) } == -1 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// A pidfd of process `pid`, or none where the kernel gives none. The pidfd is closed when it is dropped.
pub fn pidfd(pid: i32) -> Option<std::os::fd::OwnedFd> {
    use std::os::fd::FromRawFd;
    if pid <= 1 {
        return None;
    }
    let fd = unsafe { syscall(PIDFD_OPEN, pid as c_long, 0 as c_long) };
    if fd < 0 || fd > c_int::MAX as c_long {
        return None;
    }
    // The kernel gave a new descriptor that nothing else holds.
    Some(unsafe { std::os::fd::OwnedFd::from_raw_fd(fd as c_int) })
}

/// Whether the process of the pidfd `fd` is alive: signal 0, which sends nothing, is let through to it.
pub fn alive(fd: &std::os::fd::OwnedFd) -> bool {
    use std::os::fd::AsRawFd;
    unsafe { syscall(PIDFD_SEND_SIGNAL, fd.as_raw_fd() as c_long, 0 as c_long, 0 as c_long, 0 as c_long) == 0 }
}
