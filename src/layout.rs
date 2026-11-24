//! The command layout (#171): the groups, the commands, and the commands not built yet.
//! `wb help`, the usage and the dispatch are made from this one list.

/// A command of the layout.
pub struct Command {
    pub name: &'static str,
    pub group: &'static str,
    /// What it does, in one line for `wb help`.
    pub summary: &'static str,
}

/// The groups, in the order `wb help` prints them.
pub const GROUPS: [&str; 5] = ["set up", "work", "see", "take back", "plumbing"];

/// The commands of the layout, in the order of `wb help` within each group.
pub const COMMANDS: [Command; 17] = [
    Command { name: "init", group: "set up", summary: "set Watchbill up in this repository" },
    Command { name: "check", group: "set up", summary: "say whether the repository is at its revision" },
    Command { name: "doctor", group: "set up", summary: "say what on this machine Watchbill needs" },
    Command { name: "upgrade", group: "set up", summary: "pin the repository to the installed revision" },
    Command { name: "plan", group: "work", summary: "take a plan in, and list, show, edit, accept or remove it" },
    Command { name: "accept", group: "work", summary: "accept the file of an issue" },
    Command { name: "start", group: "work", summary: "start a session of one role" },
    Command { name: "run", group: "work", summary: "run the whole flow of an issue" },
    Command { name: "status", group: "see", summary: "list the issues in the flow and where each stands" },
    Command { name: "view", group: "see", summary: "show what a session of a role sees, and whether a file is in it" },
    Command { name: "log", group: "see", summary: "show what was done to an issue" },
    Command { name: "answer", group: "see", summary: "answer the open questions of an issue" },
    Command { name: "revert", group: "take back", summary: "take an issue back to before a step" },
    Command { name: "clean", group: "take back", summary: "remove what a finished session left" },
    Command { name: "prompt", group: "plumbing", summary: "print the prompt of a role" },
    Command { name: "revision", group: "plumbing", summary: "print the version, the revision and its manifest" },
    Command { name: "compare", group: "plumbing", summary: "compare two revisions or two runs" },
];

/// The commands not built yet: the command, the release it is planned for and the issue that builds it.
/// The piece that builds one removes its row, and adds its arm to the dispatch in `src/lib.rs`. After 0.0.9 the
/// releases are 0.10.0, 0.11.0, 0.12.0 and 1.0.0, as the owner named them.
pub const PLANNED: [(&str, &str, u32); 3] = [
    ("doctor", "0.12.0", 178),
    ("log", "0.12.0", 177),
    ("run", "0.12.0", 183),
];

/// Whether the layout has this command.
pub fn is_command(name: &str) -> bool {
    COMMANDS.iter().any(|command| command.name == name)
}

/// What a planned command says: `wb <name> is planned for <release> (#<issue>)`, or none when it is built.
pub fn planned(name: &str) -> Option<String> {
    PLANNED
        .iter()
        .find(|row| row.0 == name)
        .map(|(name, release, issue)| format!("wb {} is planned for {} (#{})", name, release, issue))
}

/// What `wb help` prints: the groups, each with its commands, one line each.
pub fn help() -> String {
    let mut text = String::from("usage: wb <command> [<args>]\n");
    for group in GROUPS {
        text.push_str(&format!("\n{}\n", group));
        for command in COMMANDS.iter().filter(|command| command.group == group) {
            let line = match PLANNED.iter().find(|row| row.0 == command.name) {
                Some((_, release, issue)) => format!("{} (planned for {}, #{})", command.summary, release, issue),
                None => command.summary.to_string(),
            };
            text.push_str(&format!("  {:<9} {}\n", command.name, line));
        }
    }
    text.push_str("\nwb help <command> says more of one command; wb --version prints the version.\n");
    text
}

/// What `wb help <command>` prints, or none when the command is not in the layout. A built command
/// has its usage lines and its paragraph from `usage`; a planned one says what it is planned for.
pub fn help_of(name: &str, usage: &str) -> Option<String> {
    if name == "help" {
        return Some("usage:\n  wb help [<command>]\n\n  help      print the commands by group, or what one command does\n".to_string());
    }
    let command = COMMANDS.iter().find(|command| command.name == name)?;
    if let Some(line) = planned(name) {
        return Some(format!("{}\n\n  {}\n", line, command.summary));
    }
    let lines: Vec<&str> = usage.lines().collect();
    let opens = format!("  wb {}", name);
    let mut text = String::from("usage:\n");
    let mut at = 0;
    while at < lines.len() && !lines[at].is_empty() {
        let line = lines[at];
        if line == opens || line.starts_with(&format!("{} ", opens)) {
            text.push_str(line);
            text.push('\n');
            // A usage line goes on over the lines indented further.
            while at + 1 < lines.len() && lines[at + 1].starts_with("            ") {
                at += 1;
                text.push_str(lines[at]);
                text.push('\n');
            }
        }
        at += 1;
    }
    let paragraph = format!("  {} ", name);
    if let Some(start) = lines.iter().position(|line| line.starts_with(&paragraph)) {
        text.push('\n');
        text.push_str(lines[start]);
        text.push('\n');
        for line in lines[start + 1..].iter().take_while(|line| line.starts_with("    ")) {
            text.push_str(line);
            text.push('\n');
        }
    }
    Some(text)
}
