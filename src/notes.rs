//! The notes that name the commits of a session (#237; `docs/revision.md`, point 15).
//!
//! Each commit `wb` takes back from a view carries a note in `refs/notes/watchbill`, of two lines: `stamp: <the commit of
//! the session's stamp on the job's ref>` and `role: <role>`. A session's commits are then found by git alone, on any
//! machine that fetched the notes, with no tracker: `git notes --ref=refs/notes/watchbill show <commit>`. A note, not a
//! trailer, as a squash merge erases a trailer.
//!
//! **A note is never read back.** Git writes a notes commit with no signature, and anyone who can push to `origin` can
//! write any note, so `wb` counts nothing from them: no round, stamp, plan, acceptance, handover or session. What counts
//! of a session is its stamp, signed, on the job's ref (`stamps`). The notes are an index for a person or a script.
//!
//! A stamp of a change with no issue is a comment, not a commit, so a session of it has nothing to name and gets no
//! note, which is said. Each note is written with `git notes add` and no `-f`: a note that is there is not overwritten.
//! The notes ref is pushed to `origin` after the commits are taken back, with no force, as the person.

use std::io::Write;

use crate::signers;
use crate::stamps::{self, Pushed};

/// The notes ref.
pub const REF: &str = "refs/notes/watchbill";

/// The ref the advice of a refused push keeps the notes in: not matched by the pattern of the fetch line, which a fetch
/// would otherwise force.
const KEPT: &str = "refs/notes/wb-kept";

/// The refspec that pushes it: no `+`, so no force.
fn spec() -> String {
    format!("{0}:{0}", REF)
}

/// The text of a note.
pub fn text(stamp: &str, role: &str) -> String {
    format!("stamp: {}\nrole: {}\n", stamp, role)
}

/// What to say, and what to name, for the commits `taken` back by a session of `role`.
pub struct Session<'a> {
    /// The command, which every line is said after.
    pub name: &'a str,
    pub root: &'a str,
    pub role: &'a str,
    /// The commit of the session's stamp, or why there is none to name.
    pub stamp: Result<&'a str, &'a str>,
    /// The commits taken back, as they are on the branch.
    pub taken: &'a [String],
    /// Why the notes are not pushed, where they are not: `--no-push`, or a take-back that is not the end of a session.
    pub held: Option<&'a str>,
}

/// Writes a note on each of the commits taken back, then pushes the notes ref, as the `Session` says. The notes are
/// said before they are written. 0, or 1 where a note could not be written or the push did not go through: the commits
/// taken back and the notes that were written stay in any case.
pub fn after(session: &Session, err: &mut dyn Write) -> i32 {
    if session.taken.is_empty() {
        return 0;
    }
    let name = session.name;
    let stamp = match session.stamp {
        Ok(stamp) => stamp,
        Err(why) => {
            crate::refusal::inform(err, format_args!("{}: no note is written on the commits taken back: {}", name, why));
            return 0;
        }
    };
    let _ = writeln!(
        err,
        "{}: will write a note on each of the {} commit{} taken back, in {}: `{}` and `role: {}`. A note is not signed, and wb never reads one",
        name,
        session.taken.len(),
        if session.taken.len() == 1 { "" } else { "s" },
        REF,
        crate::issue::shown(&format!("stamp: {}", stamp)),
        crate::issue::shown(session.role)
    );
    let body = text(stamp, session.role);
    let ref_arg = format!("--ref={}", REF);
    let mut code = 0;
    let mut written = 0;
    for commit in session.taken {
        match signers::git(session.root, &["notes", &ref_arg, "add", "-F", "-", commit], Some(body.as_bytes())) {
            Ok(_) => written += 1,
            Err(why) => {
                let held = if written == 0 { String::new() } else { format!(" The {} written before it stay.", written) };
                code = crate::refusal::limit(
                    err,
                    name,
                    format_args!("the note on the commit {} could not be written, and no more are: {}.{} The commits taken back stay", crate::view::short(commit), crate::issue::shown(&why), held),
                );
                break;
            }
        }
    }
    if written == 0 {
        return code;
    }
    if let Some(why) = session.held {
        let _ = writeln!(
            err,
            "{}: the notes are not pushed: {}; a `git fetch` of origin before they are pushed replaces them, as the fetch line of the notes forces the ref.\n{}: to push them, run:\n  git push origin {}",
            name,
            why,
            name,
            spec()
        );
        return code;
    }
    let _ = writeln!(err, "{}: will push the notes to origin, as the person, with no force:\n  git push origin {}", name, spec());
    match stamps::push_refs(session.root, &[spec()]) {
        Pushed::Remote => {
            crate::refusal::inform(err, format_args!("{}: pushed {} to origin", name, REF));
        }
        Pushed::Local => {
            let _ = writeln!(err, "{}: there is no remote origin, so the notes are on the local {} alone", name, REF);
        }
        Pushed::Lost(said) => {
            code = crate::refusal::limit(
                err,
                name,
                format_args!(
                    "origin refused {0} as not a fast-forward of its own ({1}): another session's notes went first. The commits taken back stay, and so do their notes, in {0} here. \
                     The fetch line of the notes forces {0}, so any `git fetch` of origin would replace the notes that are not at origin. Keep them first, then fetch, merge, push and drop what you kept: \
                     `git update-ref {3} {0}`, `git fetch origin +{0}:{0}`, `git notes --ref={0} merge -s union {3}`, `git push origin {2}`, `git update-ref -d {3}`",
                    REF,
                    said,
                    spec(),
                    KEPT
                ),
            );
        }
        Pushed::Failed(said) => {
            code = crate::refusal::limit(
                err,
                name,
                format_args!("the notes were not pushed to origin: {}. The commits taken back stay, and so do their notes, in {} here. Push them with `git push origin {}`", said, REF, spec()),
            );
        }
    }
    code
}
