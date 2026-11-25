use std::fs::File;
use std::io;
use std::mem::ManuallyDrop;
use std::os::fd::FromRawFd;
use std::process::ExitCode;

fn main() -> ExitCode {
    // In a view, `wb` is the way out of the sandbox by this name alone (#195): it is no command of `wb`.
    let mut all = std::env::args_os();
    if all.next().is_some_and(|name| name == watchbill::reach::FORWARDER) {
        let rest: Vec<std::ffi::OsString> = all.collect();
        return ExitCode::from(watchbill::reach::forward(&rest) as u8);
    }
    // The words go to `lexopt` as they were given, bytes that are not UTF-8 too.
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    // Standard output and standard error are written as they come, not held in a buffer: what an
    // interrupt prints must come after all that was printed before it. The files are not closed here.
    let mut out = ManuallyDrop::new(unsafe { File::from_raw_fd(1) });
    let mut err = ManuallyDrop::new(unsafe { File::from_raw_fd(2) });
    let mut fault = ManuallyDrop::new(unsafe { File::from_raw_fd(2) });
    let code = watchbill::guarded(
        || watchbill::run(&args, &mut io::stdin().lock(), &mut *out, &mut *err),
        &mut *fault,
    );
    ExitCode::from(code as u8)
}
