//! An interrupt (Ctrl-C) while `wb init` asks: it says `Nothing written.` and ends with 1. While a session
//! that `wb start <role>` started runs, an interrupt, a quit and a hangup go to the session, a termination is sent on to it, and `wb` waits for it (`hold`).
//! In a view the session is inside bubblewrap: bubblewrap ignores the interrupt, the quit, the hangup and the
//! termination, and `wb` sends them on to the session itself, held by a pidfd (#257, #271).
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
            // not a refusal: the person's own interrupt, before anything is written.
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
/// The held signals with their names, as `env --default-signal` takes them.
pub(crate) const NAMED: [(c_int, &str); 4] = [(SIGINT, "INT"), (SIGQUIT, "QUIT"), (SIGHUP, "HUP"), (SIGTERM, "TERM")];

/// The signals held while a session runs.
const HELD_SIGNALS: [c_int; 4] = [SIGINT, SIGQUIT, SIGHUP, SIGTERM];

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
/// Set when an interrupt came.
static INTERRUPTED: AtomicBool = AtomicBool::new(false);
/// Set when a quit came.
static QUIT: AtomicBool = AtomicBool::new(false);
/// Whether the signals of the terminal (a hangup, an interrupt, a quit) are sent on to the session too: only
/// where it has no controlling terminal (#257, #271).
static FROM_TERMINAL: AtomicBool = AtomicBool::new(false);
/// Set where the session was not found in bubblewrap, and `SESSION` is bubblewrap's own number: where the session
/// has no controlling terminal, an interrupt or a quit then kills bubblewrap, and the session with it, as nothing
/// else would reach it. Where it has one, it gets them from the terminal, and bubblewrap is not killed under it.
static LOST: AtomicBool = AtomicBool::new(false);

const SIGKILL: c_int = 9;

/// Whether a session is registered, by a pidfd or by its number.
fn registered() -> bool {
    SESSION_FD.load(Ordering::SeqCst) >= 0 || SESSION.load(Ordering::SeqCst) > 0
}

/// Whether `number` is to reach the session (or bubblewrap, where the session was not found): a termination
/// always, the others only where the session has no controlling terminal.
fn wanted(number: c_int) -> bool {
    number == SIGTERM || FROM_TERMINAL.load(Ordering::SeqCst)
}

/// Sends on `number`, which came and was taken: an interrupt or a quit where the session was not found kills
/// bubblewrap; anything else goes to the session.
fn deliver(number: c_int) {
    if (number == SIGINT || number == SIGQUIT) && LOST.load(Ordering::SeqCst) {
        let bubblewrap = SESSION.load(Ordering::SeqCst);
        if bubblewrap > 0 {
            unsafe {
                kill(bubblewrap, SIGKILL);
            }
        }
    } else {
        send(number);
    }
}

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

/// The watchdog ends the session past `max_minutes` (#267) as a termination of `wb` does (#221): the termination is
/// sent on to the session, by its pidfd or its number, or to bubblewrap where the session was not found, which
/// ignores it. Nothing is sent where no session is registered.
pub fn terminate_session() {
    if registered() {
        send(SIGTERM);
    }
}

/// After the grace (#267): the session is killed, or bubblewrap where the session was not found, and the session
/// with it (`--die-with-parent`).
pub fn kill_session() {
    if registered() {
        send(SIGKILL);
    }
}

/// A termination (#221), a hangup, an interrupt or a quit: `wb` goes on waiting. The session is sent the
/// termination, as a person's `kill` of `wb` meant it, and the hangup, the interrupt and the quit where the
/// terminal cannot reach it (#257, #271). Only `kill(2)`, the call of the kernel and atomics are used, as a
/// handler may. A signal is marked as come and taken back with one swap, here or in `pending`, so that exactly
/// one of the two sends it.
///
/// A signal that is caught, not ignored, is set back to its default in a program that is started, so a session
/// with no view gets an interrupt as it would without `wb`.
extern "C" fn on_signal(number: c_int) {
    // `kill` sets `errno` when it fails, and the program that was interrupted may be reading it: put it back.
    let errno = unsafe { __errno_location() };
    let saved = unsafe { *errno };
    let came = match number {
        SIGTERM => Some(&TERMINATED),
        SIGHUP => Some(&HUNG_UP),
        SIGINT => Some(&INTERRUPTED),
        SIGQUIT => Some(&QUIT),
        _ => None,
    };
    if let Some(came) = came {
        came.store(true, Ordering::SeqCst);
        if wanted(number) && registered() && came.swap(false, Ordering::SeqCst) {
            deliver(number);
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
/// to `wb` alone, and `wb` sends it on to the session (`session`). A session with no controlling terminal is sent
/// the others too (`terminal_sent_on`). A signal that was ignored stays ignored, as `install` keeps it.
pub struct Held;

pub fn hold() -> Held {
    let mut held = HELD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if held.0 == 0 {
        TERMINATED.store(false, Ordering::SeqCst);
        HUNG_UP.store(false, Ordering::SeqCst);
        INTERRUPTED.store(false, Ordering::SeqCst);
        QUIT.store(false, Ordering::SeqCst);
        FROM_TERMINAL.store(false, Ordering::SeqCst);
        LOST.store(false, Ordering::SeqCst);
        SESSION.store(0, Ordering::SeqCst);
        SESSION_FD.store(-1, Ordering::SeqCst);
        for (index, number) in HELD_SIGNALS.into_iter().enumerate() {
            unsafe {
                let before = signal(number, on_signal as extern "C" fn(c_int) as usize);
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
        FROM_TERMINAL.store(false, Ordering::SeqCst);
        LOST.store(false, Ordering::SeqCst);
    }

    /// The session was not found in bubblewrap, whose number is `pid` and which `wb` has not waited for (#271): a
    /// termination and a hangup go to bubblewrap, which ignores them, and, where the session has no controlling
    /// terminal, an interrupt or a quit kills it, and the session with it, also one that came before this.
    pub fn session_lost(&self, pid: u32) {
        // Set before the number, so that a handler that sees the number sees this too.
        LOST.store(true, Ordering::SeqCst);
        SESSION.store(pid as c_int, Ordering::SeqCst);
        self.pending();
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

    /// From now on a hangup, an interrupt and a quit of `wb` are sent on to the session too, also one that came
    /// before: the session has no controlling terminal, so the terminal's own cannot reach it (#257, #271).
    pub fn terminal_sent_on(&self) {
        FROM_TERMINAL.store(true, Ordering::SeqCst);
    }

    /// Sends what came before the session was registered.
    fn pending(&self) {
        for (came, number) in [(&TERMINATED, SIGTERM), (&HUNG_UP, SIGHUP), (&INTERRUPTED, SIGINT), (&QUIT, SIGQUIT)] {
            if wanted(number) && came.swap(false, Ordering::SeqCst) {
                deliver(number);
            }
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
            FROM_TERMINAL.store(false, Ordering::SeqCst);
            LOST.store(false, Ordering::SeqCst);
            for (index, number) in HELD_SIGNALS.into_iter().enumerate() {
                unsafe {
                    signal(number, held.1[index]);
                }
            }
        }
    }
}

/// Whether `number` (one of the held signals) was ignored when the first hold began, as by `nohup`, or by a shell
/// that runs `wb` in the background: the session is then started with it ignored too.
pub fn was_ignored(number: c_int) -> bool {
    let held = HELD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    match HELD_SIGNALS.iter().position(|one| *one == number) {
        Some(index) => held.0 > 0 && held.1[index] == IGNORED,
        None => false,
    }
}

/// What the child does between fork and exec, before it becomes bubblewrap (#257, #271): SIGINT, SIGQUIT, SIGHUP
/// and SIGTERM are ignored, where `ignore` says so, so that bubblewrap is not ended by them, and `fd` is kept
/// open across the exec. Only `signal(2)` and `fcntl(2)`, which may be called there.
pub fn before_bubblewrap(fd: c_int, ignore: bool) -> std::io::Result<()> {
    const F_SETFD: c_int = 2;
    if ignore {
        for number in HELD_SIGNALS {
            unsafe {
                signal(number, IGNORED);
            }
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
