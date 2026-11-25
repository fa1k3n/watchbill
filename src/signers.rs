//! The project's keys and the signed tags of the acceptances (#235; `docs/revision.md`, point 15).
//!
//! `.watchbill/signers` holds one line for each key a person signs with, in the form of git's allowed signers file with
//! one option of `wb`'s own: `<identity> roles="<role>,..." <key type> <base64 key> [comment]`. It is read only as the
//! tip of the default branch has it, so a branch cannot add a key for itself; `wb check` reads the one of the working
//! tree, to say what does not hold before it is merged.
//!
//! Where the list is there, a person's acceptance is a tag signed with an SSH key, on one exact object: the blob of the
//! issue's file for `wb plan accept`, the commit that `.finished` names for `wb accept`. `wb` checks the signature
//! itself, with `ssh-keygen -Y find-principals` and `ssh-keygen -Y verify`, against files of git's form made from the
//! list in a folder of its own under `$TMPDIR`, removed after: no setting of git decides what is trusted. A signature
//! that does not check counts for nothing.

use std::io::Write;
use std::process::{Command, Stdio};

/// The list, from the root of the repository.
pub const PATH: &str = ".watchbill/signers";

/// The roles a key may sign as: an acceptance, a stamp (#236), a handover, and the answers (#238).
pub const ROLES: [&str; 4] = ["accept", "start", "handover", "answer"];

/// The most the list may be.
pub const CAP: u64 = 64 * 1024;

/// Where the tags of the acceptances of the issues' files are.
pub const ACCEPT_TAGS: &str = "refs/tags/watchbill/accept/";

/// Where the tags of the acceptances of the work are.
pub const FINISHED_TAGS: &str = "refs/tags/watchbill/finished/";

/// The key types of OpenSSH that a line may name.
const KINDS: [&str; 7] = [
    "ssh-ed25519",
    "ecdsa-sha2-nistp256",
    "ecdsa-sha2-nistp384",
    "ecdsa-sha2-nistp521",
    "sk-ssh-ed25519@openssh.com",
    "sk-ecdsa-sha2-nistp256@openssh.com",
    "ssh-rsa",
];

/// One line of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signer {
    pub identity: String,
    pub roles: Vec<String>,
    /// The key type.
    pub kind: String,
    /// The key, in base64.
    pub key: String,
    /// The line it is on, from 1.
    pub line: usize,
}

/// The bytes of base64 text, or none where it is not base64 with its padding.
fn base64(text: &str) -> Option<Vec<u8>> {
    let value = |byte: u8| match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let bytes = text.as_bytes();
    if bytes.is_empty() || bytes.len() % 4 != 0 {
        return None;
    }
    let padding = bytes.iter().rev().take_while(|byte| **byte == b'=').count();
    if padding > 2 {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for (at, chunk) in bytes.chunks(4).enumerate() {
        let last = at == bytes.len() / 4 - 1;
        let mut word: u32 = 0;
        for (index, byte) in chunk.iter().enumerate() {
            let part = if *byte == b'=' && last && index >= 4 - padding { 0 } else { value(*byte)? };
            word = (word << 6) | u32::from(part);
        }
        let three = [(word >> 16) as u8, (word >> 8) as u8, word as u8];
        let keep = if last { 3 - padding } else { 3 };
        out.extend_from_slice(&three[..keep]);
    }
    Some(out)
}

/// Whether the base64 `key` holds a public key of the type `kind`: its first field, a length of four bytes and a
/// text, is the type.
fn holds_kind(key: &str, kind: &str) -> bool {
    let Some(bytes) = base64(key) else { return false };
    if bytes.len() < 4 {
        return false;
    }
    let length = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    bytes.len() > 4 + length && &bytes[4..4 + length] == kind.as_bytes()
}

/// What is wrong with an identity, if anything: it is one principal, with no pattern, comma, quote or space.
fn identity_fault(identity: &str) -> Option<String> {
    if identity.len() > 254 {
        return Some("the identity is longer than 254 bytes".to_string());
    }
    if let Some(c) = identity.chars().find(|c| c.is_control() || "*?!,\"'\\".contains(*c)) {
        return Some(format!(
            "the identity {} holds {}, and an identity is one principal, with no pattern, comma, quote or backslash",
            crate::issue::shown(identity),
            crate::issue::shown(&c.to_string())
        ));
    }
    None
}

/// The roles of the option `roles="..."`, or why it is not one.
fn roles_of(option: &str) -> Result<Vec<String>, String> {
    let known = || ROLES.join(", ");
    let Some(inner) = option.strip_prefix("roles=\"").and_then(|rest| rest.strip_suffix('"')) else {
        return Err(format!(
            "after the identity comes the option roles=\"<role>,...\", not {}; no other option of ssh-keygen is taken",
            crate::issue::shown(option)
        ));
    };
    let mut roles: Vec<String> = Vec::new();
    for role in inner.split(',') {
        if !ROLES.contains(&role) {
            return Err(format!("the role `{}` is not known; the roles are: {}", crate::issue::shown(role), known()));
        }
        if roles.iter().any(|before| before == role) {
            return Err(format!("the role `{}` stands twice", role));
        }
        roles.push(role.to_string());
    }
    Ok(roles)
}

/// One line of the list, or why it is not one.
fn line_of(text: &str, line: usize) -> Result<Signer, String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < 4 {
        return Err("a line is an identity, roles=\"<role>,...\", a key type and a key".to_string());
    }
    let identity = words[0];
    if let Some(why) = identity_fault(identity) {
        return Err(why);
    }
    let roles = roles_of(words[1])?;
    let kind = words[2];
    if !KINDS.contains(&kind) {
        return Err(format!("{} is not a key type of OpenSSH; the types are: {}", crate::issue::shown(kind), KINDS.join(", ")));
    }
    let key = words[3];
    if !holds_kind(key, kind) {
        return Err(format!("the key is not a public key of the type {} in base64", kind));
    }
    Ok(Signer { identity: identity.to_string(), roles, kind: kind.to_string(), key: key.to_string(), line })
}

/// Reads the list: every line that is not empty or a comment, each held to its form. The faults, one for each line
/// that does not hold, as `line <n>: <why>`.
pub fn parse(text: &str) -> Result<Vec<Signer>, Vec<String>> {
    let mut signers: Vec<Signer> = Vec::new();
    let mut faults = Vec::new();
    for (index, line) in text.split('\n').enumerate() {
        let number = index + 1;
        let line = line.strip_suffix('\r').unwrap_or(line);
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if line.chars().any(|c| c.is_control() && c != '\t') {
            faults.push(format!("line {}: the line holds a control character", number));
            continue;
        }
        match line_of(trimmed, number) {
            Ok(signer) => match signers.iter().find(|before| before.key == signer.key) {
                Some(before) => faults.push(format!("line {}: the key is on line {} already; a key stands on one line", number, before.line)),
                None => signers.push(signer),
            },
            Err(why) => faults.push(format!("line {}: {}", number, why)),
        }
    }
    if faults.is_empty() {
        Ok(signers)
    } else {
        Err(faults)
    }
}

/// The faults of the list of the working tree at `root`, each as a line that names the file, for `wb check`. None
/// where the file is not there.
pub fn check(root: &std::path::Path) -> Vec<String> {
    let path = root.join(PATH);
    match std::fs::symlink_metadata(&path) {
        Err(_) => Vec::new(),
        Ok(found) if found.file_type().is_symlink() => vec![format!("link: {} (a link is never followed)", PATH)],
        Ok(found) if !found.is_file() => vec![format!("{}: it is not a plain file", PATH)],
        Ok(_) => match crate::bounded::read_plain(&path, CAP) {
            Err(error) => vec![format!("{}: it cannot be read ({}); it is at most 64 KiB", PATH, error)],
            Ok(bytes) => match String::from_utf8(bytes) {
                Err(_) => vec![format!("{}: it is not UTF-8 text", PATH)],
                Ok(text) => parse(&text).err().unwrap_or_default().into_iter().map(|why| format!("{}: {}", PATH, why)).collect(),
            },
        },
    }
}

/// The list as the default branch has it.
pub struct List {
    /// The default branch, as a person writes it (`main`, `origin/main`).
    pub on: String,
    pub signers: Vec<Signer>,
}

/// What `wb` checks a signature against.
pub enum Keys {
    /// No list, so no signature is checked; the words say why.
    Unchecked(String),
    List(List),
    /// A list that cannot be read: no signature counts.
    Fault(String),
}

/// Runs git in `root` with no variable `GIT_*` of this process, as `plan` does: what it printed, or what it said.
fn git(root: &str, args: &[&str], input: Option<&[u8]>) -> Result<Vec<u8>, String> {
    let mut command = Command::new("git");
    command.arg("-C").arg(root).args(args);
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    command.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() }).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| format!("git could not be run: {}", error))?;
    if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
        let _ = stdin.write_all(input);
    }
    let output = child.wait_with_output().map_err(|error| format!("git could not be waited for: {}", error))?;
    if output.status.success() {
        return Ok(output.stdout);
    }
    let said = String::from_utf8_lossy(&output.stderr);
    let said: String = said.trim().chars().filter(|c| !c.is_control() || *c == '\n').collect();
    Err(if said.is_empty() { format!("git {} failed", args.first().copied().unwrap_or("")) } else { said })
}

fn git_line(root: &str, args: &[&str]) -> Result<String, String> {
    git(root, args, None).map(|printed| String::from_utf8_lossy(&printed).trim().to_string())
}

/// The default branch as a person writes it.
fn short(full: &str) -> &str {
    full.strip_prefix("refs/heads/").or_else(|| full.strip_prefix("refs/remotes/")).unwrap_or(full)
}

/// The list of the tip of the default branch of the repository at `root`.
pub fn load(root: &str) -> Keys {
    let Some(default) = crate::status::default_branch(root) else {
        return Keys::Unchecked("no default branch was found, so no .watchbill/signers is read: no signature is checked".to_string());
    };
    let on = short(&default).to_string();
    let fault = |why: String| Keys::Fault(format!("{} of {} cannot be read: {}, so no acceptance counts", PATH, on, why));
    let entry = match git(root, &["ls-tree", "-z", &default, "--", PATH], None) {
        Ok(entry) => entry,
        Err(why) => return fault(why),
    };
    if entry.is_empty() {
        return Keys::Unchecked(format!("no {} on {}: no signature is checked", PATH, on));
    }
    // `<mode> <type> <object>\t<path>`
    let entry = String::from_utf8_lossy(&entry).to_string();
    let head: Vec<&str> = entry.split('\t').next().unwrap_or("").split(' ').collect();
    let [mode, kind, object] = head.as_slice() else {
        return fault("git ls-tree printed what is not an entry".to_string());
    };
    if !matches!(*mode, "100644" | "100755") || *kind != "blob" {
        return fault("it is not a plain file".to_string());
    }
    match git_line(root, &["cat-file", "-s", object]).map(|size| size.parse::<u64>()) {
        Ok(Ok(size)) if size <= CAP => {}
        Ok(Ok(_)) => return fault("it is larger than 64 KiB".to_string()),
        Ok(Err(_)) | Err(_) => return fault("git could not say its size".to_string()),
    }
    let text = match git(root, &["cat-file", "blob", object], None).map(String::from_utf8) {
        Ok(Ok(text)) => text,
        Ok(Err(_)) => return fault("it is not UTF-8 text".to_string()),
        Err(why) => return fault(why),
    };
    match parse(&text) {
        Ok(signers) => Keys::List(List { on, signers }),
        Err(faults) => fault(faults.join("; ")),
    }
}

/// What a signature says.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Signed by a key of the list that holds the role, of this identity.
    Good(String),
    Unsigned,
    /// Signed by a key that is not in the list, or not with SSH.
    NotListed,
    /// Signed by a key of the list, of this identity, that does not hold the role.
    WithoutRole(String),
    /// Signed by a key of the list, and the signature does not check: what was signed is not what is there.
    Broken,
}

/// Where the signature of a signed object of git starts, as git finds it: the last line that starts a signature.
fn signature_start(raw: &[u8]) -> Option<usize> {
    const STARTS: [&[u8]; 4] = [b"-----BEGIN PGP SIGNATURE-----", b"-----BEGIN PGP MESSAGE-----", b"-----BEGIN SIGNED MESSAGE-----", b"-----BEGIN SSH SIGNATURE-----"];
    let mut found = None;
    let mut at = 0;
    while at < raw.len() {
        if STARTS.iter().any(|start| raw[at..].starts_with(start)) {
            found = Some(at);
        }
        at = match raw[at..].iter().position(|byte| *byte == b'\n') {
            Some(end) => at + end + 1,
            None => raw.len(),
        };
    }
    found
}

/// A folder of its own under `$TMPDIR`, readable by this user alone, removed when it is dropped.
struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new() -> Result<Scratch, String> {
        use std::os::unix::fs::DirBuilderExt;
        let base = std::env::temp_dir();
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|since| since.subsec_nanos()).unwrap_or(0);
        for attempt in 0..100u32 {
            let path = base.join(format!("wb-signers-{}-{}-{}", std::process::id(), nanos, attempt));
            match std::fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Ok(Scratch(path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(format!("a folder for the check could not be made in {} ({})", base.display(), error)),
            }
        }
        Err(format!("a folder for the check could not be made in {}", base.display()))
    }

    fn write(&self, name: &str, bytes: &[u8]) -> Result<std::path::PathBuf, String> {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).map_err(|error| format!("{} could not be written ({})", path.display(), error))?;
        Ok(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Runs `ssh-keygen` with `args`, and `input` on standard input: whether it ended with 0, and what it printed.
fn ssh_keygen(args: &[&std::ffi::OsStr], input: Option<&[u8]>) -> Result<(bool, String), String> {
    let mut command = Command::new("ssh-keygen");
    command.args(args).stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() }).stdout(Stdio::piped()).stderr(Stdio::null());
    let mut child = command.spawn().map_err(|error| format!("ssh-keygen could not be run ({})", error))?;
    if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
        let _ = stdin.write_all(input);
    }
    let output = child.wait_with_output().map_err(|error| format!("ssh-keygen could not be waited for ({})", error))?;
    Ok((output.status.success(), String::from_utf8_lossy(&output.stdout).to_string()))
}

/// The lines of git's allowed signers form for the keys of `signers` that `keep` keeps.
fn allowed(signers: &[Signer], keep: &dyn Fn(&Signer) -> bool) -> String {
    signers.iter().filter(|signer| keep(signer)).map(|signer| format!("{} {} {}\n", signer.identity, signer.kind, signer.key)).collect()
}

/// Checks the signature of the signed object `raw` (a tag as `git cat-file tag` prints it), as git signs one with SSH
/// (the namespace `git`), against the keys of `list` that hold `role`. `Err` where it could not be checked.
pub fn verify(raw: &[u8], role: &str, list: &List) -> Result<Verdict, String> {
    let Some(start) = signature_start(raw) else { return Ok(Verdict::Unsigned) };
    if !raw[start..].starts_with(b"-----BEGIN SSH SIGNATURE-----") {
        return Ok(Verdict::NotListed);
    }
    let (payload, signature) = raw.split_at(start);
    let scratch = Scratch::new()?;
    let sig = scratch.write("signature", signature)?;
    let all = scratch.write("all", allowed(&list.signers, &|_| true).as_bytes())?;
    let with_role = scratch.write("role", allowed(&list.signers, &|signer| signer.roles.iter().any(|held| held == role)).as_bytes())?;
    let os = |text: &'static str| std::ffi::OsStr::new(text);
    let (found, printed) = ssh_keygen(&[os("-Y"), os("find-principals"), os("-s"), sig.as_os_str(), os("-f"), all.as_os_str()], None)?;
    let Some(principal) = printed.lines().map(str::trim).find(|line| !line.is_empty()).filter(|_| found).map(str::to_string) else {
        return Ok(Verdict::NotListed);
    };
    let check = |file: &std::path::Path| {
        ssh_keygen(&[os("-Y"), os("verify"), os("-f"), file.as_os_str(), os("-I"), std::ffi::OsStr::new(&principal), os("-n"), os("git"), os("-s"), sig.as_os_str()], Some(payload))
            .map(|(good, _)| good)
    };
    if check(&with_role)? {
        return Ok(Verdict::Good(principal));
    }
    if check(&all)? {
        return Ok(Verdict::WithoutRole(principal));
    }
    Ok(Verdict::Broken)
}

/// What the signature of an acceptance comes to.
#[derive(Debug, PartialEq, Eq)]
pub enum Signed {
    /// No signature is checked; the words say why.
    Unchecked(String),
    /// It checks; the words say by whom.
    Good(String),
    /// It counts for nothing; the words say why.
    Void(String),
}

/// The tag of an acceptance: under `prefix`, the name, and the first 12 hex digits of the SHA-256 of `body`, the
/// `.accepted` or `.finished` it is made with.
pub fn tag_of(prefix: &str, name: &str, body: &[u8]) -> String {
    format!("{}{}/{}", prefix, name, &crate::manifest::hash_bytes(body)[..12])
}

/// The value of the line `<key>: ` of a text.
fn value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|line| line.strip_prefix(key).and_then(|rest| rest.strip_prefix(": ")))
}

/// The message of the tag of the acceptance of an issue's file: its SHA-256, a line `job:` for each hash of the job
/// (`none` where there is no plan), and the identity and the time of the acceptance `body`.
pub fn file_message(name: &str, body: &str, jobs: &[String]) -> String {
    let mut message = format!("Accept the issue's file ({})\n\nsha256: {}\n", name, value(body, "sha256").unwrap_or(""));
    if jobs.is_empty() {
        message.push_str("job: none\n");
    }
    for job in jobs {
        message.push_str(&format!("job: {}\n", job));
    }
    message.push_str(&format!("by: {}\ntime: {}\n", value(body, "by").unwrap_or(""), value(body, "time").unwrap_or("")));
    message
}

/// The message of the tag of the acceptance of the work: the lines of the record `body`.
pub fn work_message(name: &str, body: &str) -> String {
    format!("Accept the work ({})\n\n{}", name, body)
}

/// The hashes of a job that a message of the tag of an issue's file names.
fn jobs_in(message: &str) -> Vec<&str> {
    message.lines().filter_map(|line| line.strip_prefix("job: ")).collect()
}

/// Whether git's config of `root` signs a tag with an SSH key: `user.signingkey` is set, and `gpg.format` is `ssh`.
/// The setting that is not, named, with how to set it.
pub fn signing_ready(root: &str) -> Result<(), String> {
    let key = git_line(root, &["config", "user.signingkey"]).unwrap_or_default();
    if key.is_empty() {
        return Err(format!(
            "git has no signing key set (user.signingkey), and where {} is there an acceptance is a signed tag. Set it with `git config user.signingkey <the path of your SSH key>`.",
            PATH
        ));
    }
    let format = git_line(root, &["config", "gpg.format"]).unwrap_or_default();
    if format != "ssh" {
        let now = if format.is_empty() { "not set, so git signs with OpenPGP".to_string() } else { format!("{}", crate::issue::shown(&format)) };
        return Err(format!("git signs with an SSH key only where gpg.format is ssh, and it is {}. Set it with `git config gpg.format ssh`.", now));
    }
    Ok(())
}

/// The id of the blob of `bytes`, as git would write it with no filter.
pub fn blob_of(root: &str, bytes: &[u8], write: bool) -> Result<String, String> {
    let args: &[&str] = if write { &["hash-object", "-w", "--no-filters", "--stdin"] } else { &["hash-object", "--no-filters", "--stdin"] };
    git(root, args, Some(bytes)).map(|printed| String::from_utf8_lossy(&printed).trim().to_string())
}

/// Makes the signed tag `tag` on `target` with `message`, by `git tag -s`, as the person's git signs. Never over a tag
/// that is there.
pub fn sign(root: &str, tag: &str, target: &str, message: &str) -> Result<(), String> {
    let short = tag.strip_prefix("refs/tags/").unwrap_or(tag);
    git(root, &["tag", "-s", "--cleanup=verbatim", "-F", "-", short, target], Some(message.as_bytes())).map(|_| ())
}

/// Removes the tag `tag` that `wb` made at `object`, where it is still there.
pub fn unmake(root: &str, tag: &str, object: &str) {
    let _ = git(root, &["update-ref", "-d", tag, object], None);
}

/// The object of the tag `tag`, where it is an annotated tag.
pub fn object_of(root: &str, tag: &str) -> Option<String> {
    git_line(root, &["rev-parse", "--verify", "-q", tag]).ok()
}

/// Checks the tag `tag` of an acceptance against `keys`: it is there, is a tag signed by a key of the list with the
/// role `accept`, on the object `target` of the type `kind`, named `tag`, and its message is `message`. Where the
/// message differs in its `job:` lines alone, the words say how the job differs.
fn checked(root: &str, keys: &Keys, tag: &str, kind: &str, target: &str, message: &str) -> Signed {
    let list = match keys {
        Keys::Unchecked(why) => return Signed::Unchecked(why.clone()),
        Keys::Fault(why) => return Signed::Void(why.clone()),
        Keys::List(list) => list,
    };
    let void = |why: String| Signed::Void(format!("{}: {}, so the acceptance counts for nothing", tag, why));
    match git_line(root, &["cat-file", "-t", tag]) {
        Err(_) => return Signed::Void(format!("{} is not there, so the acceptance counts for nothing", tag)),
        Ok(found) if found != "tag" => return void("unsigned".to_string()),
        Ok(_) => {}
    }
    let raw = match git(root, &["cat-file", "tag", tag], None) {
        Ok(raw) => raw,
        Err(why) => return void(format!("it cannot be read ({})", why)),
    };
    match verify(&raw, "accept", list) {
        Err(why) => return void(format!("its signature could not be checked: {}", why)),
        Ok(Verdict::Unsigned) => return void("unsigned".to_string()),
        Ok(Verdict::NotListed) => return void(format!("signed by a key that is not in {} of {}", PATH, list.on)),
        Ok(Verdict::WithoutRole(identity)) => {
            return void(format!("signed by the key of {}, which {} of {} holds without the role accept", crate::issue::shown(&identity), PATH, list.on))
        }
        Ok(Verdict::Broken) => return void("its signature does not check: what was signed is not what the tag holds".to_string()),
        Ok(Verdict::Good(identity)) => {
            let text = String::from_utf8_lossy(&raw).to_string();
            let Some((head, rest)) = text.split_once("\n\n") else { return void("it is not a tag of an acceptance".to_string()) };
            let header = |key: &str| head.lines().find_map(|line| line.strip_prefix(key).and_then(|rest| rest.strip_prefix(' ')));
            let short = tag.strip_prefix("refs/tags/").unwrap_or(tag);
            if header("object") != Some(target) || header("type") != Some(kind) || header("tag") != Some(short) {
                return void(format!("it is not on the {} {} of the acceptance", kind, crate::view::short(target)));
            }
            let found = rest.split("-----BEGIN SSH SIGNATURE-----").next().unwrap_or("");
            if found != message {
                let (then, now) = (jobs_in(found), jobs_in(message));
                let without = |text: &str| text.lines().filter(|line| !line.starts_with("job: ")).collect::<Vec<_>>().join("\n");
                if then != now && without(found) == without(message) {
                    let shown = |jobs: &[&str]| jobs.iter().map(|job| crate::issue::shown(job)).collect::<Vec<_>>().join(", ");
                    return void(format!("names the job {}, and the job is now {}: text was taken in after the acceptance", shown(&then), shown(&now)));
                }
                return void("its message is not that of the acceptance".to_string());
            }
            Signed::Good(format!("{}: signed by {}, with the role accept in {} of {}", tag, crate::issue::shown(&identity), PATH, list.on))
        }
    }
}

/// The signature of the acceptance `body` (the text of `.accepted`) of the issue's file of `name`, whose bytes are
/// `bytes`, against `keys`.
pub fn file_signed(root: &str, keys: &Keys, name: &str, bytes: &[u8], body: &str) -> Signed {
    if let Keys::Unchecked(why) = keys {
        return Signed::Unchecked(why.clone());
    }
    let blob = match blob_of(root, bytes, false) {
        Ok(blob) => blob,
        Err(why) => return Signed::Void(format!("git could not give the blob of the issue's file ({}), so the acceptance counts for nothing", why)),
    };
    let jobs = crate::plan::source_hashes(root, name);
    let tag = tag_of(ACCEPT_TAGS, name, body.as_bytes());
    checked(root, keys, &tag, "blob", &blob, &file_message(name, body, &jobs))
}

/// The signature of the acceptance of the work of `name`, whose record is `body` and names `commit`, against `keys`.
pub fn work_signed(root: &str, keys: &Keys, name: &str, commit: &str, body: &str) -> Signed {
    let tag = tag_of(FINISHED_TAGS, name, body.as_bytes());
    checked(root, keys, &tag, "commit", commit, &work_message(name, body))
}

/// Makes and checks the signed tag of an acceptance, after it is said on `out` under `command`: `Ok` with the tag where
/// it checks, else why, with the tag it made removed again.
pub fn make(root: &str, keys: &Keys, command: &str, tag: &str, kind: &str, target: &str, message: &str, out: &mut dyn Write) -> Result<(), String> {
    let _ = writeln!(out, "{}: will make the signed tag {} on the {} {}, with the message:\n{}", command, tag, kind, target, message);
    let _ = out.flush();
    sign(root, tag, target, message).map_err(|why| format!("the signed tag {} could not be made: {}", tag, why))?;
    let made = object_of(root, tag).unwrap_or_default();
    let raw = git(root, &["cat-file", "tag", tag], None).unwrap_or_default();
    let list = match keys {
        Keys::List(list) => list,
        _ => {
            unmake(root, tag, &made);
            return Err(format!("{} could not be read again", PATH));
        }
    };
    let verdict = verify(&raw, "accept", list);
    let why = match verdict {
        Ok(Verdict::Good(_)) => return Ok(()),
        Ok(Verdict::Unsigned) => "git made it unsigned".to_string(),
        Ok(Verdict::NotListed) => format!("it is signed by a key that is not in {} of {}, which a person adds by a change that a person merges", PATH, list.on),
        Ok(Verdict::WithoutRole(identity)) => format!("it is signed by the key of {}, which {} of {} holds without the role accept", crate::issue::shown(&identity), PATH, list.on),
        Ok(Verdict::Broken) => "its signature does not check".to_string(),
        Err(why) => format!("its signature could not be checked: {}", why),
    };
    unmake(root, tag, &made);
    Err(format!("the tag {} does not check: {}. It is removed again", tag, why))
}
