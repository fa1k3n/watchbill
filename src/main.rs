use std::io;
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| watchbill::word(arg.as_bytes()))
        .collect();
    let code = watchbill::run(&args, &mut io::stdout().lock(), &mut io::stderr().lock());
    ExitCode::from(code as u8)
}
