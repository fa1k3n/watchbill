//! `wb init` finds the project's code style (#289): the facts of `HEAD` with no model, written into
//! `.watchbill/project/all.md`, and `wb init --survey`, a session on a runner the organisation marks `local: true`
//! that drafts `.watchbill/project/style.md`. The local model is stood in for by a script the test writes.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const ALL_MD: &str = ".watchbill/project/all.md";
const STYLE_MD: &str = ".watchbill/project/style.md";

/// The words of the warning that stay whatever the file: the problem, the file that sets it, what to put there.
const WARNED: [&str; 4] = [
    "no runner is marked local in ",
    "so the code style cannot be surveyed by a model on this machine",
    "local: true",
    "Then run:  wb init --survey",
];

/// The environment of a program a test runs: its own home, settings and git settings.
struct Home {
    home: Folder,
}

impl Home {
    fn new() -> Home {
        let home = folder_named("home");
        fs::create_dir_all(home.path.join(".config")).unwrap();
        fs::write(home.path.join(".gitconfig"), "[user]\n\tname = Ada Lovelace\n\temail = ada@example.org\n[commit]\n\tgpgsign = false\n").unwrap();
        Home { home }
    }

    fn vars(&self) -> Vec<(String, String)> {
        let mut vars = env();
        vars.retain(|(name, _)| !["HOME", "XDG_CONFIG_HOME", "GIT_CONFIG_GLOBAL"].contains(&name.as_str()));
        vars.push(("HOME".to_string(), self.home.path.display().to_string()));
        vars.push(("XDG_CONFIG_HOME".to_string(), self.home.path.join(".config").display().to_string()));
        vars.push(("GIT_CONFIG_GLOBAL".to_string(), self.home.path.join(".gitconfig").display().to_string()));
        vars
    }

    fn git(&self, folder: &Path, args: &[&str]) {
        let status = Command::new("git").arg("-C").arg(folder).args(args).env_clear().envs(self.vars()).stdout(Stdio::null()).status().unwrap();
        assert!(status.success(), "git {:?} failed", args);
    }

    fn git_out(&self, folder: &Path, args: &[&str]) -> String {
        let run = Command::new("git").arg("-C").arg(folder).args(args).env_clear().envs(self.vars()).output().unwrap();
        text(&run.stdout)
    }

    /// `wb` as a person runs it, with this home.
    fn wb(&self, folder: &Path, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_wb")).args(args).current_dir(folder).env_clear().envs(self.vars()).stdin(Stdio::null()).output().unwrap()
    }

    /// Commits everything of the working tree.
    fn commit_all(&self, folder: &Path) {
        self.git(folder, &["add", "-A"]);
        self.git(folder, &["commit", "-q", "--allow-empty", "-m", "All of it"]);
    }
}

/// A repository with code committed: `src/brake.c` and `src/brake.h`, indented with four spaces.
fn code_repository(home: &Home) -> Folder {
    let folder = repository(None);
    put(&folder.path, "src/brake.c", "#include \"brake.h\"\n\nint brake_apply(int force)\n{\n    if (force > 0) {\n        return force;\n    }\n    return 0;\n}\n");
    put(&folder.path, "src/brake.h", "#ifndef BRAKE_H\n#define BRAKE_H\n\nint brake_apply(int force);\n\n#endif\n");
    home.commit_all(&folder.path);
    folder
}

/// A repository with code, set up by `wb init --yes` (with no all.md, as the files of the test give none to it when
/// `bare`) and all of it committed.
fn set_up(home: &Home) -> Folder {
    let folder = code_repository(home);
    let run = home.wb(&folder.path, &["init", "--yes"]);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    home.commit_all(&folder.path);
    folder
}

/// What a call of the library gives, as the program would.
fn output(status: i32, out: Vec<u8>, err: Vec<u8>) -> Output {
    Output { status: std::process::ExitStatus::from_raw(status << 8), stdout: out, stderr: err }
}

/// The machine of a test: the organisation's file, a state folder, bubblewrap or its stand-in, and a home.
struct Bench {
    home: Home,
    _organisation: Folder,
    org: PathBuf,
    state: Folder,
    tools: Folder,
    log: Folder,
    sandbox: PathBuf,
    real: bool,
}

const SANDBOX_STAND_IN: &str = r#"#!/bin/sh
# A stand-in for bubblewrap, where it cannot make a sandbox here: it makes none, and runs the session in the view,
# as bubblewrap holds the view at the checkout.
pairs=''
chdir=''
while [ $# -gt 0 ] && [ "$1" != -- ]; do
  case "$1" in
    --bind|--ro-bind) pairs="$pairs$2
$3
"; shift 3 ;;
    --bind-try|--ro-bind-try|--symlink|--setenv) shift 3 ;;
    --chdir) chdir="$2"; shift 2 ;;
    --unsetenv|--tmpfs|--proc|--dev|--info-fd) shift 2 ;;
    *) shift ;;
  esac
done
shift
view=''
if [ -n "$chdir" ]; then
  view=$(printf '%s' "$pairs" | awk -v want="$chdir" 'NR % 2 == 1 { from = $0 } NR % 2 == 0 && $0 == want && found == 0 { print from; found = 1 }')
fi
if [ -n "$view" ]; then cd "$view" || exit 99; fi
exec "$@"
"#;

/// The stand-in of a local model: it looks at the tree, tries to write outside `.watchbill/project/`, writes a file
/// there beside the draft, and writes the draft. `$1` is the prompt, or the model where `start` gives one first.
const SESSION: &str = r#"#!/bin/sh
if [ -f src/brake.c ]; then seen=yes; else seen=no; fi
if ( echo x > src/new.c ) 2>/dev/null; then wrote=yes; else wrote=no; fi
echo other > .watchbill/project/other.md
case "$*" in *style.md*) asked=yes ;; *) asked=no ;; esac
printf '# The code style\n\nseen: %s\nwrote elsewhere: %s\nasked for style.md: %s\nfirst word: %s\n' "$seen" "$wrote" "$asked" "$(printf '%s' "$1" | head -n 1)" > .watchbill/project/style.md
"#;

impl Bench {
    fn new() -> Bench {
        let tools = folder_named("tools");
        let stand_in = tools.path.join("bwrap");
        let (sandbox, real) = match watchbill::view::works(Path::new(watchbill::view::BWRAP)) {
            Ok(()) => (PathBuf::from(watchbill::view::BWRAP), true),
            Err(why) if std::env::var_os("WB_TEST_REQUIRE_SANDBOX").is_some_and(|value| value == "1") => {
                panic!("WB_TEST_REQUIRE_SANDBOX=1, and bubblewrap cannot make a sandbox here: {}", why)
            }
            Err(why) => {
                eprintln!("the session runs by a stand-in for bubblewrap, which cannot make a sandbox here: {}", why);
                program(&stand_in, SANDBOX_STAND_IN);
                (stand_in, false)
            }
        };
        let organisation = folder_named("organisation");
        let org = organisation.path.join("organisation.yml");
        let bench = Bench { home: Home::new(), _organisation: organisation, org, state: folder_named("state"), tools, log: folder_named("log"), sandbox, real };
        bench.session("session.sh", SESSION);
        bench
    }

    fn org_file(&self) -> PathBuf {
        self.org.clone()
    }

    /// Writes the organisation's file; `{tools}` is the folder of the stand-ins, `{log}` a folder they may write.
    fn organisation(&self, text: &str) {
        let text = text.replace("{tools}", &self.tools.path.display().to_string()).replace("{log}", &self.log.path.display().to_string());
        fs::write(self.org_file(), text).unwrap();
    }

    fn session(&self, name: &str, text: &str) -> PathBuf {
        let path = self.tools.path.join(name);
        program(&path, text);
        path
    }

    fn machine_with<'a>(&'a self, sandbox: &'a Path) -> watchbill::Machine<'a> {
        watchbill::Machine { organisation: &self.org, terminal: false, state: Some(&self.state.path), sandbox, home: Some(&self.home.home.path) }
    }

    fn init(&self, folder: &Path, args: &[&str]) -> Output {
        self.init_on(folder, &self.sandbox, args)
    }

    fn init_on(&self, folder: &Path, sandbox: &Path, args: &[&str]) -> Output {
        let args: Vec<OsString> = args.iter().map(OsString::from).collect();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let machine = self.machine_with(sandbox);
        let status = watchbill::init_on(&args, folder, &machine, &mut std::io::empty(), &mut out, &mut err);
        output(status, out, err)
    }

    fn check(&self, folder: &Path) -> Output {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let status = watchbill::check_on(&[], folder, &self.org_file(), &mut out, &mut err);
        output(status, out, err)
    }

    /// The views and the folders of what sessions wrote, that are left in the state folder.
    fn left(&self) -> Vec<String> {
        let mut found = Vec::new();
        for below in ["views", "out"] {
            if let Ok(entries) = fs::read_dir(self.state.path.join("watchbill").join(below)) {
                found.extend(entries.flatten().map(|entry| entry.path().display().to_string()));
            }
        }
        found
    }
}

fn program(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// An organisation's file whose default runner is local, starts the stand-in session and binds the folder of the
/// stand-ins.
const LOCAL: &str = "levels: [public, internal, confidential]
default: local-a
runners:
  local-a:
    tools: this machine
    model: a model on this machine
    agreement: nothing leaves the machine
    up_to: confidential
    local: true
    start: ['{tools}/session.sh', '{prompt}']
    binds: ['{tools}']
";

/// An organisation's file with no runner marked local.
const VENDOR: &str = "levels: [public, internal, confidential]
default: vendor
runners:
  vendor:
    tools: this machine
    model: a vendor's model
    agreement: EU, no retention, no training
    up_to: internal
    start: ['{tools}/session.sh', '{prompt}']
    binds: ['{tools}']
";

/// Both: the vendor's runner is the default.
const BOTH: &str = "levels: [public, internal, confidential]
default: vendor
runners:
  vendor:
    tools: this machine
    model: a vendor's model
    agreement: EU, no retention, no training
    up_to: internal
    start: ['{tools}/session.sh', '{prompt}']
    binds: ['{tools}']
  local-a:
    tools: this machine
    model: a model on this machine
    agreement: nothing leaves the machine
    up_to: confidential
    local: true
    start: ['{tools}/session.sh', '{prompt}']
    binds: ['{tools}']
  local-b:
    tools: this machine
    model: a model on this machine
    agreement: nothing leaves the machine
    up_to: confidential
    local: true
";

fn has_warning(said: &str) -> bool {
    WARNED.iter().all(|words| said.contains(words))
}

// ---- WB-INIT-61 to 65: the facts of HEAD, with no model ----

const EDITORCONFIG: &str = "root = true\n\n[*.{c,h}]\nindent_style = space\nindent_size = 4\n";
const CLANG_FORMAT: &str = "BasedOnStyle: LLVM\nIndentWidth: 4\nColumnLimit: 100\n";

#[test]
fn test_WB_INIT_61_the_style_files_of_head_are_written_into_all_md_with_their_paths_and_shown_first() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    let folder = code_repository(&bench.home);
    put(&folder.path, ".editorconfig", EDITORCONFIG);
    put(&folder.path, ".clang-format", CLANG_FORMAT);
    bench.home.commit_all(&folder.path);

    let run = bench.init(&folder.path, &["--yes"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let all = read(&folder.path.join(ALL_MD));
    assert!(all.contains("`.editorconfig`") && all.contains("indent_size = 4") && all.contains("[*.{c,h}]"), "{}", all);
    assert!(all.contains("`.clang-format`") && all.contains("ColumnLimit: 100"), "{}", all);
    let out = text(&run.stdout);
    let shown = out.find(&format!("{}\n", ALL_MD)).unwrap_or_else(|| panic!("all.md is not shown: {}", out));
    let wrote = out.find("Wrote ").unwrap_or(out.len());
    assert!(shown < wrote, "{}", out);
    assert!(out[..wrote].contains("ColumnLimit: 100"), "what all.md holds is shown before it is written: {}", out);
}

#[test]
fn test_WB_INIT_61_a_second_run_writes_over_nothing() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    let folder = code_repository(&bench.home);
    put(&folder.path, ".editorconfig", EDITORCONFIG);
    bench.home.commit_all(&folder.path);
    assert_eq!(code(&bench.init(&folder.path, &["--yes"])), Some(0));
    fs::write(folder.path.join(ALL_MD), "Corrected by a person.\n").unwrap();

    let again = bench.init(&folder.path, &["--yes"]);

    assert_eq!(code(&again), Some(1), "{}", text(&again.stderr));
    assert!(text(&again.stderr).contains("is already there"), "{}", text(&again.stderr));
    assert_eq!(read(&folder.path.join(ALL_MD)), "Corrected by a person.\n");
}

#[test]
fn test_WB_INIT_62_of_clang_tidy_only_the_naming_rules_are_copied() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    let folder = code_repository(&bench.home);
    put(
        &folder.path,
        ".clang-tidy",
        "Checks: '-*,modernize-use-nullptr'\nCheckOptions:\n  - key: readability-identifier-naming.FunctionCase\n    value: lower_case\n  - key: modernize-use-nullptr.NullMacros\n    value: NULL\n  - key: readability-identifier-naming.MacroDefinitionCase\n    value: UPPER_CASE\n",
    );
    bench.home.commit_all(&folder.path);

    let run = bench.init(&folder.path, &["--yes"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let all = read(&folder.path.join(ALL_MD));
    assert!(all.contains("readability-identifier-naming.FunctionCase: lower_case"), "{}", all);
    assert!(all.contains("readability-identifier-naming.MacroDefinitionCase: UPPER_CASE"), "{}", all);
    assert!(!all.contains("NullMacros") && !all.contains("modernize"), "{}", all);
}

#[test]
fn test_WB_INIT_62_a_clang_tidy_whose_naming_rules_are_all_left_out_is_not_said_to_hold_none() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    let folder = code_repository(&bench.home);
    put(&folder.path, ".clang-tidy", "CheckOptions:\n  - key: readability-identifier-naming.FunctionCase\n    value: '{lower}_case'\n");
    bench.home.commit_all(&folder.path);

    let run = bench.init(&folder.path, &["--yes"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let all = read(&folder.path.join(ALL_MD));
    assert!(all.contains("`.clang-tidy`") && all.contains("1 line left out"), "{}", all);
    assert!(!all.contains("holds no naming rule"), "{}", all);
    assert!(!all.contains("{lower}"), "{}", all);
}

#[test]
fn test_WB_INIT_63_the_facts_of_the_code_are_written_with_the_files_they_were_seen_in() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    let folder = repository(None);
    put(&folder.path, "src/a.c", "int a(void)\n{\n    return 1;\n}\n");
    put(&folder.path, "src/b.c", "int b(void)\n{\n    if (1) {\n        return 2;\n    }\n    return 0;\n}\n");
    put(&folder.path, "src/c.c", "int c(void)\r\n{\r\n\treturn 3;\r\n}\r\n");
    let longest = format!("    return 4; /* {} */", "x".repeat(84));
    put(&folder.path, "src/long.h", &format!("int d(void)\n{{\n{}\n}}\n", longest));
    put(&folder.path, "tools/run.py", "def main():\n  return 0\n");
    bench.home.commit_all(&folder.path);

    let run = bench.init(&folder.path, &["--yes"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let all = read(&folder.path.join(ALL_MD));
    let c = all.lines().find(|line| line.contains("`*.c`")).unwrap_or_else(|| panic!("{}", all));
    assert!(c.contains("seen in 3 files"), "{}", c);
    assert!(c.contains("indented with spaces in 2, with tabs in 1"), "{}", c);
    assert!(c.contains("the indent is 4 spaces in 2"), "{}", c);
    assert!(c.contains("line endings LF in 2, CRLF in 1"), "{}", c);
    let h = all.lines().find(|line| line.contains("`*.h`")).unwrap_or_else(|| panic!("{}", all));
    assert!(h.contains("seen in 1 file") && h.contains(&format!("the longest line has {} characters, in src/long.h", longest.len())), "{}", h);
    let py = all.lines().find(|line| line.contains("`*.py`")).unwrap_or_else(|| panic!("{}", all));
    assert!(py.contains("the indent is 2 spaces in 1"), "{}", py);
}

#[test]
fn test_WB_INIT_64_a_line_a_prompt_would_read_as_a_placeholder_is_left_out_and_counted() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    let folder = code_repository(&bench.home);
    put(&folder.path, "CONTRIBUTING.md", "# Contributing\n\nUse snake_case for functions.\nName a file {module}_{part}.c.\nA bell\x07 rings.\n");
    bench.home.commit_all(&folder.path);

    let run = bench.init(&folder.path, &["--yes"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let all = read(&folder.path.join(ALL_MD));
    assert!(all.contains("Use snake_case for functions."), "{}", all);
    assert!(!all.contains("{module}") && !all.contains("bell"), "{}", all);
    assert!(all.contains("2 lines left out"), "{}", all);
}

#[test]
fn test_WB_INIT_64_a_style_file_over_the_bound_or_not_utf_8_is_named_and_not_copied() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    let folder = code_repository(&bench.home);
    put(&folder.path, "CODING_STYLE.md", &"Indent with four spaces.\n".repeat(3000));
    fs::write(folder.path.join(".pylintrc"), b"[FORMAT]\nmax-line-length=\xff\n").unwrap();
    let many: String = (0..250).map(|n| format!("rule {}\n", n)).collect();
    put(&folder.path, "rustfmt.toml", &many);
    bench.home.commit_all(&folder.path);

    let run = bench.init(&folder.path, &["--yes"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let all = read(&folder.path.join(ALL_MD));
    assert!(all.contains("`CODING_STYLE.md`") && all.contains("over 64 KiB") && !all.contains("Indent with four spaces."), "{}", all);
    assert!(all.contains("`.pylintrc`") && all.contains("not UTF-8") && !all.contains("max-line-length"), "{}", all);
    assert!(all.contains("rule 199\n") && !all.contains("rule 200\n"), "{}", all);
}

#[test]
fn test_WB_INIT_65_a_file_not_in_head_is_not_read() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    let folder = code_repository(&bench.home);
    put(&folder.path, ".editorconfig", EDITORCONFIG);
    put(&folder.path, "src/new.c", "int n(void)\n{\n\treturn 0;\n}\n");

    let run = bench.init(&folder.path, &["--yes"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let all = read(&folder.path.join(ALL_MD));
    assert!(!all.contains(".editorconfig") && !all.contains("indent_size"), "{}", all);
    assert!(all.contains("seen in 1 file") && !all.contains("tabs in 1"), "{}", all);
}

#[test]
fn test_WB_INIT_65_where_head_gives_nothing_no_all_md_is_written() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    let folder = repository(None);

    let run = bench.init(&folder.path, &["--yes"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(config(&folder.path).exists());
    assert!(!folder.path.join(ALL_MD).exists());
    assert!(!text(&run.stdout).contains("all.md"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_INIT_65_an_all_md_that_is_there_is_in_the_way_and_nothing_is_written() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    let folder = code_repository(&bench.home);
    put(&folder.path, ALL_MD, "Written by a person.\n");

    let run = bench.init(&folder.path, &["--yes"]);

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("all.md is already there"), "{}", text(&run.stderr));
    assert!(!config(&folder.path).exists());
    assert_eq!(read(&folder.path.join(ALL_MD)), "Written by a person.\n");
}

// ---- WB-INIT-66, 67, WB-CONF-27: the warning where no runner is local ----

#[test]
fn test_WB_INIT_66_with_no_local_runner_init_warns_and_writes_the_rest() {
    let bench = Bench::new();
    bench.organisation(VENDOR);
    let folder = code_repository(&bench.home);
    put(&folder.path, ".editorconfig", EDITORCONFIG);
    bench.home.commit_all(&folder.path);

    let run = bench.init(&folder.path, &["--yes"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let err = text(&run.stderr);
    assert!(has_warning(&err), "{}", err);
    assert!(err.starts_with("wb init: no runner is marked local in "), "{}", err);
    assert!(err.contains(&bench.org_file().display().to_string()), "{}", err);
    assert!(config(&folder.path).exists() && folder.path.join(ALL_MD).exists());
}

#[test]
fn test_WB_INIT_66_with_no_organisations_file_init_warns_and_writes_the_rest() {
    let bench = Bench::new();
    let folder = code_repository(&bench.home);

    let run = bench.init(&folder.path, &["--yes"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let err = text(&run.stderr);
    assert!(has_warning(&err), "{}", err);
    assert!(err.contains(&bench.org_file().display().to_string()) && err.contains("there is no such file"), "{}", err);
    assert!(config(&folder.path).exists());
}

#[test]
fn test_WB_INIT_66_with_a_local_runner_init_does_not_warn() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    let folder = code_repository(&bench.home);

    let run = bench.init(&folder.path, &["--yes"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(!text(&run.stderr).contains("marked local"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_INIT_67_survey_with_no_local_runner_ends_with_1_in_the_same_words_and_runs_nothing() {
    let bench = Bench::new();
    bench.organisation(VENDOR);
    let folder = code_repository(&bench.home);
    let init = bench.init(&folder.path, &["--yes"]);
    assert_eq!(code(&init), Some(0), "{}", text(&init.stderr));
    bench.home.commit_all(&folder.path);
    let warned = text(&init.stderr).strip_prefix("wb init: ").unwrap_or_else(|| panic!("{}", text(&init.stderr))).to_string();

    let run = bench.init(&folder.path, &["--survey"]);

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let err = text(&run.stderr);
    assert_eq!(err, format!("wb init: limit: {}", warned));
    assert!(has_warning(&err), "{}", err);
    assert!(!folder.path.join(STYLE_MD).exists());
    assert!(bench.left().is_empty(), "{:?}", bench.left());
}

#[test]
fn test_WB_INIT_67_survey_with_no_organisations_file_ends_with_1_and_runs_nothing() {
    let bench = Bench::new();
    let folder = set_up(&bench.home);

    let run = bench.init(&folder.path, &["--survey"]);

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let err = text(&run.stderr);
    assert!(has_warning(&err) && err.contains("there is no such file"), "{}", err);
    assert!(!folder.path.join(STYLE_MD).exists());
    assert!(bench.left().is_empty(), "{:?}", bench.left());
}

#[test]
fn test_WB_CONF_27_a_runner_is_local_only_by_local_true_never_by_its_name_or_words() {
    let bench = Bench::new();
    bench.organisation(
        "levels: [public]
default: local
runners:
  local:
    tools: this machine, local
    model: a local model on this machine
    agreement: local, nothing leaves the machine
    up_to: public
    local: false
    start: ['{tools}/session.sh', '{prompt}']
    binds: ['{tools}']
",
    );
    let folder = code_repository(&bench.home);

    let init = bench.init(&folder.path, &["--yes"]);
    bench.home.commit_all(&folder.path);
    let survey = bench.init(&folder.path, &["--survey"]);

    assert_eq!(code(&init), Some(0), "{}", text(&init.stderr));
    assert!(has_warning(&text(&init.stderr)), "{}", text(&init.stderr));
    assert_eq!(code(&survey), Some(1));
    assert!(has_warning(&text(&survey.stderr)), "{}", text(&survey.stderr));
    assert!(!folder.path.join(STYLE_MD).exists());
}

// ---- WB-INIT-68 to 75: the survey ----

#[test]
fn test_WB_INIT_68_a_runner_that_is_not_local_is_refused_by_name() {
    let bench = Bench::new();
    bench.organisation(BOTH);
    let folder = set_up(&bench.home);

    let by_default = bench.init(&folder.path, &["--survey"]);
    let named = bench.init(&folder.path, &["--survey", "--runner", "vendor"]);
    let unknown = bench.init(&folder.path, &["--survey", "--runner", "nobody"]);
    let no_start = bench.init(&folder.path, &["--survey", "--runner", "local-b"]);

    for (run, name) in [(&by_default, "'vendor'"), (&named, "'vendor'"), (&unknown, "'nobody'"), (&no_start, "'local-b'")] {
        assert_eq!(code(run), Some(1), "{}", text(&run.stderr));
        let err = text(&run.stderr);
        assert!(err.starts_with("wb init: limit: ") && err.contains(name), "{}", err);
    }
    assert!(text(&by_default.stderr).contains("local: true") && text(&by_default.stderr).contains("local-a"), "{}", text(&by_default.stderr));
    assert!(text(&no_start.stderr).contains("start"), "{}", text(&no_start.stderr));
    assert!(!folder.path.join(STYLE_MD).exists());
    assert!(bench.left().is_empty(), "{:?}", bench.left());
}

#[test]
fn test_WB_INIT_69_a_local_runner_that_reaches_off_this_machine_is_refused() {
    let bench = Bench::new();
    bench.organisation(&format!("{}    reach: ['http://localhost:11434', 'models.example.com:443']\n", LOCAL));
    let folder = set_up(&bench.home);

    let run = bench.init(&folder.path, &["--survey"]);

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let err = text(&run.stderr);
    assert!(err.starts_with("wb init: limit: ") && err.contains("models.example.com:443") && !err.contains("localhost:11434"), "{}", err);
    assert!(!folder.path.join(STYLE_MD).exists());
}

#[test]
fn test_WB_INIT_70_with_no_network_of_its_own_the_survey_ends_with_1_saying_why() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    let folder = set_up(&bench.home);
    let no_net = bench.tools.path.join("no-net-bwrap");
    program(
        &no_net,
        "#!/bin/sh\nfor word in \"$@\"; do\n  if [ \"$word\" = --unshare-net ]; then echo 'bwrap: loopback: Failed RTM_NEWADDR: Operation not permitted' >&2; exit 1; fi\ndone\nexec /usr/bin/true\n",
    );

    let run = bench.init_on(&folder.path, &no_net, &["--survey"]);

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let err = text(&run.stderr);
    assert!(err.starts_with("wb init: limit: ") && err.contains("network of its own") && err.contains("RTM_NEWADDR"), "{}", err);
    assert!(!folder.path.join(STYLE_MD).exists());
    assert!(bench.left().is_empty(), "{:?}", bench.left());
}

#[test]
fn test_WB_INIT_71_the_session_runs_in_a_view_of_the_whole_tree_and_only_style_md_comes_back_uncommitted() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    let folder = set_up(&bench.home);
    let head = bench.home.git_out(&folder.path, &["rev-parse", "HEAD"]);

    let run = bench.init(&folder.path, &["--survey"]);

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    let style = read(&folder.path.join(STYLE_MD));
    assert!(style.contains("seen: yes"), "the session did not see the whole tree: {}", style);
    assert!(style.contains("asked for style.md: yes"), "{}", style);
    if bench.real {
        assert!(style.contains("wrote elsewhere: no"), "the tree was writable: {}", style);
    }
    assert!(!folder.path.join("src/new.c").exists());
    assert!(!folder.path.join(".watchbill/project/other.md").exists());
    assert_eq!(bench.home.git_out(&folder.path, &["rev-parse", "HEAD"]), head, "something was committed");
    assert_eq!(bench.home.git_out(&folder.path, &["status", "--porcelain"]), "?? .watchbill/project/style.md\n");
    assert!(bench.left().is_empty(), "{:?}", bench.left());
    let said = text(&run.stdout) + &text(&run.stderr);
    assert!(said.contains("style.md") && said.contains("not committed"), "{}", said);
}

#[test]
fn test_WB_INIT_71_a_draft_line_with_a_control_character_or_a_placeholder_is_left_out_and_counted() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    // An escape, a right-to-left override (U+202E) and a known placeholder, each on a line of its own.
    bench.session(
        "session.sh",
        "#!/bin/sh\nprintf '# The code style\\n\\nkept\\na \\033[31m red\\nb \\342\\200\\256 turned\\nc {issue} filled\\nalso kept\\n' > .watchbill/project/style.md\n",
    );
    let folder = set_up(&bench.home);

    let run = bench.init(&folder.path, &["--survey"]);

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    assert_eq!(read(&folder.path.join(STYLE_MD)), "# The code style\n\nkept\nalso kept\n");
    let said = text(&run.stdout) + &text(&run.stderr);
    assert!(said.contains("3 lines left out"), "{}", said);
}

#[test]
fn test_WB_INIT_71_a_project_folder_of_head_that_is_a_link_is_refused_before_the_session() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    let ran = bench.log.path.join("ran");
    bench.session("session.sh", &format!("#!/bin/sh\ntouch '{}'\necho drafted > .watchbill/project/style.md\n", ran.display()));
    let folder = set_up(&bench.home);
    let outside = folder_named("outside");
    let project = folder.path.join(".watchbill/project");
    fs::remove_dir_all(&project).unwrap();
    std::os::unix::fs::symlink(&outside.path, &project).unwrap();
    bench.home.commit_all(&folder.path);
    // The working tree has a real folder again, not committed.
    fs::remove_file(&project).unwrap();
    fs::create_dir(&project).unwrap();

    let run = bench.init(&folder.path, &["--survey"]);

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let err = text(&run.stderr);
    assert!(err.contains(".watchbill/project of HEAD is a link"), "{}", err);
    assert!(!ran.exists(), "the session ran");
    assert_eq!(fs::read_dir(&outside.path).unwrap().count(), 0);
    assert!(fs::symlink_metadata(folder.path.join(STYLE_MD)).is_err());
    assert!(bench.left().is_empty(), "{:?}", bench.left());
}

#[test]
fn test_WB_INIT_72_a_style_md_that_is_there_is_never_written_over_and_nothing_runs() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    bench.session("session.sh", "#!/bin/sh\necho drafted > .watchbill/project/style.md\n");
    let folder = set_up(&bench.home);
    put(&folder.path, STYLE_MD, "Written by a person.\n");

    let in_tree = bench.init(&folder.path, &["--survey"]);

    assert_eq!(code(&in_tree), Some(1), "{}", text(&in_tree.stderr));
    assert!(text(&in_tree.stderr).contains("style.md is already there"), "{}", text(&in_tree.stderr));
    assert_eq!(read(&folder.path.join(STYLE_MD)), "Written by a person.\n");
    bench.home.commit_all(&folder.path);
    fs::remove_file(folder.path.join(STYLE_MD)).unwrap();
    let in_head = bench.init(&folder.path, &["--survey"]);
    assert_eq!(code(&in_head), Some(1), "{}", text(&in_head.stderr));
    assert!(text(&in_head.stderr).contains("style.md"), "{}", text(&in_head.stderr));
    assert!(!folder.path.join(STYLE_MD).exists(), "the session ran");
    assert!(bench.left().is_empty(), "{:?}", bench.left());
}

#[test]
fn test_WB_INIT_73_a_session_that_fails_or_leaves_no_plain_draft_writes_nothing() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    let folder = set_up(&bench.home);
    let cases = [
        ("fails", "#!/bin/sh\necho drafted > .watchbill/project/style.md\nexit 3\n"),
        ("none", "#!/bin/sh\nexit 0\n"),
        ("link", "#!/bin/sh\nln -s /etc/hostname .watchbill/project/style.md\n"),
        ("binary", "#!/bin/sh\nprintf 'a\\377b\\n' > .watchbill/project/style.md\n"),
    ];
    for (name, session) in cases {
        bench.session("session.sh", session);

        let run = bench.init(&folder.path, &["--survey"]);

        assert_eq!(code(&run), Some(1), "{}: {}", name, text(&run.stderr));
        assert!(fs::symlink_metadata(folder.path.join(STYLE_MD)).is_err(), "{}: a style.md was written", name);
        assert!(bench.left().is_empty(), "{}: {:?}", name, bench.left());
    }
}

#[test]
fn test_WB_INIT_73_a_session_that_fails_leaves_no_folder_made_in_the_working_tree() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    bench.session("session.sh", "#!/bin/sh\necho drafted > .watchbill/project/style.md\nexit 3\n");
    let folder = set_up(&bench.home);
    let project = folder.path.join(".watchbill/project");
    fs::remove_dir_all(&project).unwrap();

    let failed = bench.init(&folder.path, &["--survey"]);

    assert_eq!(code(&failed), Some(1), "{}", text(&failed.stderr));
    assert!(fs::symlink_metadata(&project).is_err(), "a folder was made in the working tree");
    assert!(bench.left().is_empty(), "{:?}", bench.left());
    // Where the session ends with 0, the folder is made with the draft.
    bench.session("session.sh", "#!/bin/sh\necho drafted > .watchbill/project/style.md\n");
    let taken = bench.init(&folder.path, &["--survey"]);
    assert_eq!(code(&taken), Some(0), "{}", text(&taken.stderr));
    assert_eq!(read(&folder.path.join(STYLE_MD)), "drafted\n");
}

#[test]
fn test_WB_INIT_74_survey_is_not_taken_with_yes_nor_runner_without_it() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    let folder = set_up(&bench.home);

    let with_yes = bench.init(&folder.path, &["--survey", "--yes"]);
    let runner_alone = bench.init(&folder.path, &["--runner", "local-a"]);

    for run in [&with_yes, &runner_alone] {
        assert_eq!(code(run), Some(2), "{}", text(&run.stderr));
        assert!(text(&run.stderr).contains("usage:"), "{}", text(&run.stderr));
    }
    assert!(!folder.path.join(STYLE_MD).exists());
}

#[test]
fn test_WB_INIT_74_survey_in_a_repository_not_set_up_ends_with_2() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    let folder = code_repository(&bench.home);

    let run = bench.init(&folder.path, &["--survey"]);

    assert_eq!(code(&run), Some(2), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("not set up"), "{}", text(&run.stderr));
    assert!(!folder.path.join(".watchbill").exists());
}

#[test]
fn test_WB_INIT_75_model_and_effort_come_from_the_runners_planner() {
    let bench = Bench::new();
    let with_planner = "levels: [public, internal, confidential]
default: local-a
runners:
  local-a:
    tools: this machine
    model: a model on this machine
    agreement: nothing leaves the machine
    up_to: confidential
    local: true
    planner:
      up_to: confidential
      model: qwen-coder-3-30
      effort: high
    start: ['{tools}/session.sh', '{model}', '{effort}', '{prompt}']
    binds: ['{tools}']
";
    bench.organisation(with_planner);
    let folder = set_up(&bench.home);

    let run = bench.init(&folder.path, &["--survey"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(read(&folder.path.join(STYLE_MD)).contains("first word: qwen-coder-3-30"), "{}", read(&folder.path.join(STYLE_MD)));

    fs::remove_file(folder.path.join(STYLE_MD)).unwrap();
    let start = with_planner.find("    planner:").unwrap();
    let end = with_planner.find("    start:").unwrap();
    bench.organisation(&format!("{}{}", &with_planner[..start], &with_planner[end..]));
    let refused = bench.init(&folder.path, &["--survey"]);
    assert_eq!(code(&refused), Some(1), "{}", text(&refused.stderr));
    assert!(text(&refused.stderr).contains("{model}") && text(&refused.stderr).contains("planner"), "{}", text(&refused.stderr));
    assert!(!folder.path.join(STYLE_MD).exists());
}

// ---- WB-CONF-26: the field `local` ----

#[test]
fn test_WB_CONF_26_local_is_true_or_false_alone() {
    let bench = Bench::new();
    let folder = set_up(&bench.home);
    for value in ["yes", "'true'", "True", "1", "[true]"] {
        bench.organisation(&LOCAL.replace("local: true", &format!("local: {}", value)));

        let run = bench.init(&folder.path, &["--survey"]);

        assert_eq!(code(&run), Some(1), "{}: {}", value, text(&run.stderr));
        let err = text(&run.stderr);
        assert!(err.contains(&bench.org_file().display().to_string()) && err.contains("runners.local-a.local"), "{}: {}", value, err);
        assert!(!folder.path.join(STYLE_MD).exists());
    }
    bench.organisation(&LOCAL.replace("    local: true\n", "    locally: true\n"));
    let unknown = bench.init(&folder.path, &["--survey"]);
    assert_eq!(code(&unknown), Some(1));
    assert!(text(&unknown.stderr).contains("`local`"), "the known keys name local: {}", text(&unknown.stderr));
}

// ---- WB-CONF-28, 29: `wb check` says it again ----

#[test]
fn test_WB_CONF_28_check_warns_where_there_is_no_style_md_and_no_local_runner() {
    let bench = Bench::new();
    bench.organisation(VENDOR);
    let folder = set_up(&bench.home);
    put_signers(&folder.path);

    let run = bench.check(&folder.path);

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    let err = text(&run.stderr);
    assert!(err.starts_with("wb check: no runner is marked local in "), "{}", err);
    assert!(has_warning(&err), "{}", err);
}

#[test]
fn test_WB_CONF_29_check_does_not_warn_with_a_style_md_or_a_local_runner() {
    let bench = Bench::new();
    bench.organisation(LOCAL);
    let folder = set_up(&bench.home);
    put_signers(&folder.path);

    let local = bench.check(&folder.path);

    assert_eq!(code(&local), Some(0), "{}{}", text(&local.stdout), text(&local.stderr));
    assert_eq!(text(&local.stderr), "");
    bench.organisation(VENDOR);
    put(&folder.path, STYLE_MD, "# The code style\n");
    let drafted = bench.check(&folder.path);
    assert_eq!(code(&drafted), Some(0), "{}{}", text(&drafted.stdout), text(&drafted.stderr));
    assert_eq!(text(&drafted.stderr), "");
}
