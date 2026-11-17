use std::fs::File;
use std::io;
use std::mem::ManuallyDrop;
use std::os::fd::FromRawFd;
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| watchbill::word(arg.as_bytes()))
        .collect();
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
