//! Each refusal of each command is a limit or a guard (#180), and says which.
//!
//! A limit is what no flag of `wb` passes: the view, classed code, the acceptance, the rule table, the checks
//! that a file is as `HEAD` has it, the bounded reads, the sandbox, the checks of a take-back, and a fault `wb`
//! cannot go past. A guard protects a person's time, and names the flag that passes it, and that flag passes it
//! alone. There is no general `--force`.
//!
//! A refusal is written on standard error, after the command's name as every line of `wb` is:
//! `<command>: limit: <why>` or `<command>: guard: <why>`. A message of several lines has the label on its first.
//! Standard output does not change: the plumbing's output is fixed. Every place that ends a command with 1 for a
//! refusal writes it here; a test holds that no other does.

use std::fmt::Display;
use std::io::Write;

/// The flags that pass a guard, each with its command: `--ignore-local` a local change under `reads` (#172),
/// `--ignore-kept` a view kept with work not taken back (#222), `--no-commit` the checks of the commit of
/// `wb accept`, which then commits nothing.
pub const GUARDS: [(&str, &str); 3] = [("start", "--ignore-local"), ("start", "--ignore-kept"), ("accept", "--no-commit")];

/// The message of a limit: `limit: ` and why.
pub fn limit_text(why: impl Display) -> String {
    format!("limit: {}", why)
}

/// The message of a guard: `guard: ` and why, which names `flag`.
pub fn guard_text(flag: &str, why: impl Display) -> String {
    let why = why.to_string();
    debug_assert!(why.contains(flag), "a guard names its flag {}: {}", flag, why);
    format!("guard: {}", why)
}

/// Writes a message that `limit_text` or `guard_text` made, after `name`, and gives 1.
pub fn say(err: &mut dyn Write, name: &str, message: &str) -> i32 {
    debug_assert!(message.starts_with("limit: ") || message.starts_with("guard: "), "{}", message);
    let _ = writeln!(err, "{}: {}", name, message);
    1
}

/// A message without its label: the lines of a start keep why it was refused, and `check:` says at which check
/// (#124), as before #180.
pub fn unlabelled(message: &str) -> &str {
    message.strip_prefix("limit: ").or_else(|| message.strip_prefix("guard: ")).unwrap_or(message)
}

/// Writes a limit after `name`, and gives 1.
pub fn limit(err: &mut dyn Write, name: &str, why: impl Display) -> i32 {
    say(err, name, &limit_text(why))
}

/// Writes a guard after `name`, naming `flag`, and gives 1.
pub fn guard(err: &mut dyn Write, name: &str, flag: &str, why: impl Display) -> i32 {
    say(err, name, &guard_text(flag, why))
}
