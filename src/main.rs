use std::io;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let code = watchbill::run(&args, &mut io::stdout().lock(), &mut io::stderr().lock());
    ExitCode::from(code as u8)
}
