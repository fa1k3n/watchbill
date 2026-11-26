//! The one loader of YAML in `wb`. Nothing else in `wb` uses the library.
//!
//! It reads the events the library gives and builds only plain values: texts, whole numbers,
//! lists and tables. It decides itself what a value written plain is, so the answer does not
//! rest on which version of YAML the library follows:
//!
//! - a key written twice is refused, at any depth;
//! - a tag, an anchor, an alias and a merge key (`<<`) are refused, with the line;
//! - a value is held to the type its field asks for, and where a text is expected and the value
//!   is written plain and could be read as something else (`no`, `1.10`, a date), the message
//!   says to put it in quotes.
//!
//! A fault is a `YamlError`. Its message names the file and the place, and is never a panic.

use std::collections::HashMap;
use std::fmt;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use yaml_rust2::parser::{Event, Parser, Tag};
use yaml_rust2::scanner::{Marker, ScanError, TScalarStyle};

/// The deepest a list or a table may be nested. The tables of a revision need four.
pub const DEEPEST: usize = 100;

/// How many faults of one file are named at most.
const FAULTS_NAMED: usize = 10;

/// A file that is refused. Its `Display` is the whole message for the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YamlError {
    message: String,
}

impl YamlError {
    fn new(message: String) -> YamlError {
        YamlError { message }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for YamlError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for YamlError {}

/// One step of a place in a file: a key of a table, or a position in a list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Part {
    Key(String),
    Index(usize),
}

impl From<&str> for Part {
    fn from(key: &str) -> Part {
        Part::Key(key.to_string())
    }
}

impl From<String> for Part {
    fn from(key: String) -> Part {
        Part::Key(key)
    }
}

impl From<usize> for Part {
    fn from(index: usize) -> Part {
        Part::Index(index)
    }
}

impl fmt::Display for Part {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Part::Key(key) => formatter.write_str(key),
            Part::Index(index) => write!(formatter, "{}", index),
        }
    }
}

/// What a value written plain may be read as, by YAML 1.1 or by YAML 1.2.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reading {
    /// Nothing that any version reads as another type: a text.
    Text,
    /// A whole number that every version reads alike.
    Whole(i64),
    /// A whole number too large to be held.
    Large,
    /// Nothing (null).
    Nothing,
    /// A truth value.
    Boolean,
    /// A number that is not a whole number written plainly: a fraction, an exponent, a base,
    /// a leading zero, an underscore, a number in base 60, infinity.
    Number,
    /// A date, or a date and a time.
    Date,
    /// The value tag of YAML 1.1 (`=`).
    Other,
}

impl Reading {
    /// What is said of a value written plain: what YAML may read it as, when a version of YAML
    /// writes it so, and else that it is too close to that (`yES`, `nULL`, `.nAn`).
    fn said(&self, text: &str) -> String {
        let spelt = |forms: &[&str]| forms.contains(&text);
        let as_yaml_writes_it = match self {
            Reading::Boolean => spelt(&[
                "yes", "Yes", "YES", "no", "No", "NO", "true", "True", "TRUE", "false", "False", "FALSE", "on", "On", "ON", "off",
                "Off", "OFF",
            ]),
            Reading::Nothing => spelt(&["", "~", "null", "Null", "NULL"]),
            Reading::Number => {
                let rest = unsigned(text);
                ![".inf", ".nan"].contains(&rest.to_ascii_lowercase().as_str())
                    || [".inf", ".Inf", ".INF"].contains(&rest)
                    || [".nan", ".NaN", ".NAN"].contains(&text)
            }
            _ => true,
        };
        if as_yaml_writes_it {
            format!("YAML may read it as {}", self.words())
        } else {
            format!("written plain, it is too close to {}", self.words())
        }
    }

    fn words(&self) -> &'static str {
        match self {
            Reading::Text => "a text",
            Reading::Whole(_) => "a whole number",
            Reading::Large => "a whole number too large to be read",
            Reading::Nothing => "nothing (null)",
            Reading::Boolean => "a boolean",
            Reading::Number => "a number",
            Reading::Date => "a date",
            Reading::Other => "a value of another type",
        }
    }
}

/// A value as the file holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// A value written plain, as it is written, and what it may be read as.
    Plain(String, Reading),
    /// A value in quotes or written as a block (`|`, `>`): always a text.
    Quoted(String),
    List(Vec<Node>),
    /// The keys as they are written, in the order of the file.
    Table(Vec<(Key, Node)>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub value: Value,
    /// The line it starts on, from 1.
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Key {
    pub text: String,
    pub line: usize,
}

// What a value written plain is. Each test is a pattern of YAML 1.1 (its specification, and the
// resolver of PyYAML where that is wider) or of the core schema of YAML 1.2.

fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

fn all(text: &str, allowed: impl Fn(char) -> bool) -> bool {
    !text.is_empty() && text.chars().all(allowed)
}

fn unsigned(text: &str) -> &str {
    text.strip_prefix(['+', '-']).unwrap_or(text)
}

/// `0` or a digit 1 to 9 and digits, with a sign or not: what every version reads as the same whole number.
fn whole(text: &str) -> Option<Reading> {
    let digits = unsigned(text);
    if !all(digits, is_digit) || (digits.len() > 1 && digits.starts_with('0')) {
        return None;
    }
    Some(match text.parse::<i64>() {
        Ok(number) => Reading::Whole(number),
        Err(_) => Reading::Large,
    })
}

/// A whole number in another form: base 2, 8 or 16, a leading zero, underscores (1.1 and 1.2).
fn other_integer(text: &str) -> bool {
    let rest = unsigned(text);
    let base = |prefix: &str, digit: fn(char) -> bool| {
        rest.strip_prefix(prefix).is_some_and(|digits| all(digits, |c| digit(c) || c == '_'))
    };
    base("0b", |c| c == '0' || c == '1')
        || base("0o", |c| ('0'..='7').contains(&c))
        || base("0x", |c| c.is_ascii_hexdigit())
        || (rest.starts_with(|c: char| c.is_ascii_digit()) && all(rest, |c| is_digit(c) || c == '_'))
}

/// A fraction or an exponent: `1.5`, `1.`, `.5`, `1e3`, `1.5e-3`, `+.inf`, `.NaN`.
fn float(text: &str) -> bool {
    let rest = unsigned(text);
    if [".inf", ".nan"].contains(&rest.to_ascii_lowercase().as_str()) {
        return true;
    }
    let (mantissa, exponent) = match rest.find(['e', 'E']) {
        Some(at) => (&rest[..at], Some(&rest[at + 1..])),
        None => (rest, None),
    };
    let mantissa_ok = match mantissa.split_once('.') {
        Some((whole, fraction)) => {
            !(whole.is_empty() && fraction.is_empty())
                && whole.chars().chain(fraction.chars()).all(|c| is_digit(c) || c == '_')
                && whole.chars().chain(fraction.chars()).any(is_digit)
        }
        None => exponent.is_some() && all(mantissa, is_digit),
    };
    let exponent_ok = match exponent {
        None => mantissa.contains('.'),
        Some(digits) => all(unsigned(digits), is_digit),
    };
    mantissa_ok && exponent_ok
}

/// A number in base 60 (1.1): `12:30`, `1:20:30`, `190:20:30.15`.
fn base_sixty(text: &str) -> bool {
    let mut parts = unsigned(text).split(':');
    let first = parts.next().unwrap_or("");
    if !first.starts_with(|c: char| c.is_ascii_digit()) || !all(first, |c| is_digit(c) || c == '_') {
        return false;
    }
    let rest: Vec<&str> = parts.collect();
    if rest.is_empty() {
        return false;
    }
    let sixty = |part: &str| match part.len() {
        1 => all(part, is_digit),
        2 => part.starts_with(|c: char| ('0'..='5').contains(&c)) && all(part, is_digit),
        _ => false,
    };
    let (last, middle) = rest.split_last().unwrap_or((&"", &[]));
    let last = match last.split_once('.') {
        Some((part, fraction)) => {
            if !fraction.chars().all(|c| is_digit(c) || c == '_') {
                return false;
            }
            part
        }
        None => last,
    };
    middle.iter().all(|part| sixty(part)) && sixty(last)
}

/// Takes `count` digits, or `low` to `high` of them, from the front of `text`.
fn take_digits(text: &str, low: usize, high: usize) -> Option<(u32, &str)> {
    let count = text.chars().take(high).take_while(|c| c.is_ascii_digit()).count();
    if count < low {
        return None;
    }
    Some((text[..count].parse().ok()?, &text[count..]))
}

/// A date of 1.1, alone or with a time: the year, the month, the day, and whether the time is one.
fn timestamp(text: &str) -> Option<(u32, u32, u32, bool)> {
    let (year, rest) = take_digits(text, 4, 4)?;
    let (month, rest) = take_digits(rest.strip_prefix('-')?, 1, 2)?;
    let (day, rest) = take_digits(rest.strip_prefix('-')?, 1, 2)?;
    if rest.is_empty() {
        return Some((year, month, day, true));
    }
    let rest = match rest.strip_prefix(['T', 't']) {
        Some(after) => after,
        None => {
            let after = rest.trim_start_matches([' ', '\t']);
            if after.len() == rest.len() {
                return None;
            }
            after
        }
    };
    let (hour, rest) = take_digits(rest, 1, 2)?;
    let (minute, rest) = take_digits(rest.strip_prefix(':')?, 2, 2)?;
    let (second, rest) = take_digits(rest.strip_prefix(':')?, 2, 2)?;
    let rest = match rest.strip_prefix('.') {
        Some(after) => after.trim_start_matches(|c: char| c.is_ascii_digit()),
        None => rest,
    };
    let zone = rest.trim_start_matches([' ', '\t']);
    let zone_ok = if zone.is_empty() {
        rest.is_empty()
    } else if zone == "Z" {
        true
    } else {
        match zone.strip_prefix(['+', '-']).and_then(|after| take_digits(after, 1, 2)) {
            Some((_, "")) => true,
            Some((_, minutes)) => minutes.strip_prefix(':').and_then(|after| take_digits(after, 2, 2)).is_some_and(|(_, end)| end.is_empty()),
            None => false,
        }
    };
    if !zone_ok {
        return None;
    }
    Some((year, month, day, hour <= 23 && minute <= 59 && second <= 59))
}

fn real_date(year: u32, month: u32, day: u32) -> bool {
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    year >= 1 && (1..=days).contains(&day)
}

/// What a value written plain may be read as.
pub fn reading(text: &str) -> Reading {
    // `y` and `n` are left out: the type list of YAML 1.1 has them, but PyYAML and YAML 1.2 read
    // them as texts, and so did every table so far.
    let words = ["yes", "no", "true", "false", "on", "off"];
    if text.is_empty() || text == "~" || text.eq_ignore_ascii_case("null") {
        Reading::Nothing
    } else if words.iter().any(|word| text.eq_ignore_ascii_case(word)) {
        Reading::Boolean
    } else if let Some(number) = whole(text) {
        number
    } else if other_integer(text) || float(text) || base_sixty(text) {
        Reading::Number
    } else if timestamp(text).is_some() {
        Reading::Date
    } else if text == "=" {
        Reading::Other
    } else {
        Reading::Text
    }
}

/// Whether a character may stand in a YAML file: tab, line breaks, and the printable characters.
fn printable(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r' | '\u{85}' | ' '..='~' | '\u{a0}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}')
        || c >= '\u{10000}'
}

/// Whether a character is a line break in YAML 1.1 and not in YAML 1.2, where it is part of a text.
fn break_of_one_version(c: char) -> bool {
    matches!(c, '\u{85}' | '\u{2028}' | '\u{2029}')
}

/// Each character of a text with its line, and whether it is written content: not white
/// space, not a line break, and not on a line that is a comment. Lines are counted as the
/// library counts them: CR LF, CR and LF are each one break.
fn characters(text: &str) -> Vec<(char, usize, bool)> {
    let mut all = Vec::new();
    let (mut line, mut after_cr, mut comment, mut line_start) = (1, false, false, true);
    for c in text.chars() {
        let is_break = c == '\n' || c == '\r';
        if line_start && c == '#' {
            comment = true;
        }
        if !matches!(c, ' ' | '\t') {
            line_start = false;
        }
        all.push((c, line, !is_break && !matches!(c, ' ' | '\t') && !comment));
        if c == '\n' && after_cr {
            after_cr = false;
            continue;
        }
        after_cr = c == '\r';
        if is_break {
            line += 1;
            comment = false;
            line_start = true;
        }
    }
    all
}

/// The lines of a text, as the library counts them: CR LF, CR and LF each end one.
fn lines(text: &str) -> Vec<&str> {
    let mut all = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find(['\r', '\n']) {
        all.push(&rest[..at]);
        // not a refusal: the width of a line break, no status.
        let width = if rest[at..].starts_with("\r\n") { 2 } else { 1 };
        rest = &rest[at + width..];
    }
    all.push(rest);
    all
}

/// The version a `%YAML` directive on this line names, when it names one that is not 1.
fn other_version(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("%YAML")?;
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    let version = rest.split_whitespace().next().unwrap_or("");
    let one = version.strip_prefix("1.").is_some_and(|minor| all(minor, is_digit));
    if one {
        None
    } else {
        Some(version)
    }
}

/// A tag as it is written: `!!str` for the tags of YAML, `!local` for others.
fn tag_words(tag: &Tag) -> String {
    if tag.handle == "tag:yaml.org,2002:" {
        format!("!!{}", tag.suffix)
    } else {
        format!("{}{}", tag.handle, tag.suffix)
    }
}

/// The message of a library error without its place, which the loader writes itself.
fn scan_words(error: &ScanError) -> String {
    match error.info() {
        "recursion limit exceeded" => format!("it is nested too deep to be read (more than {} levels)", DEEPEST),
        info => info.split_whitespace().collect::<Vec<_>>().join(" "),
    }
}

/// A list or a table that is being read.
enum Open {
    List(Vec<Node>, usize),
    Table(Vec<(Key, Node)>, usize, Option<Key>, HashMap<String, usize>),
}

struct Builder<'a> {
    text: &'a str,
    /// Made when an empty value in a list first needs its line.
    content: Option<Vec<(char, usize, bool)>>,
    open: Vec<Open>,
    root: Option<Node>,
    documents: usize,
    faults: Vec<(usize, String)>,
}

impl Builder<'_> {
    fn fault(&mut self, line: usize, words: String) {
        self.faults.push((line, words));
    }

    /// The line of a scalar. An empty value written plain, which is no text at all, is on the line
    /// of its key in a table, and elsewhere on the line where something was last written: the
    /// library gives the place of what comes after it.
    fn scalar_line(&mut self, value: &str, style: TScalarStyle, mark: &Marker) -> usize {
        if !value.is_empty() || style != TScalarStyle::Plain {
            return mark.line();
        }
        match self.open.last() {
            Some(Open::Table(_, _, Some(key), _)) => return key.line,
            // An empty key: the library gives the place where it stands.
            Some(Open::Table(_, _, None, _)) => return mark.line(),
            _ => {}
        }
        // The place given is that of what follows: the next item, past its `-`, or what ends the list.
        let text = self.text;
        let all = self.content.get_or_insert_with(|| characters(text));
        let mut at = mark.index().min(all.len());
        let skip_blank = |at: &mut usize| {
            while *at > 0 && !all[*at - 1].2 {
                *at -= 1;
            }
        };
        skip_blank(&mut at);
        if at > 0 && all[at - 1].0 == '-' && all[at - 1].1 == mark.line() && (at == 1 || !all[at - 2].2) {
            at -= 1;
            skip_blank(&mut at);
        }
        if at > 0 {
            all[at - 1].1
        } else {
            mark.line()
        }
    }

    fn anchor_and_tag(&mut self, line: usize, anchor: usize, tag: &Option<Tag>) {
        if anchor > 0 {
            self.fault(line, "an anchor (`&`) is not read: write the value out where it is used".to_string());
        }
        if let Some(tag) = tag {
            self.fault(line, format!("a tag (`{}`) is not read: only plain values are, and a value in quotes is a text", tag_words(tag)));
        }
    }

    /// Puts a value that is read in its place: the top, a list, a key, or the value of a key.
    fn place(&mut self, node: Node) {
        let line = node.line;
        match self.open.last_mut() {
            None => self.root = Some(node),
            Some(Open::List(items, _)) => items.push(node),
            Some(Open::Table(entries, _, pending, seen)) => match pending.take() {
                Some(key) => entries.push((key, node)),
                None => {
                    let text = match node.value {
                        Value::Plain(text, _) if text.is_empty() => {
                            *pending = Some(Key { text: String::new(), line });
                            self.faults.push((line, "a key is empty here; a key is a text".to_string()));
                            return;
                        }
                        Value::Plain(text, _) | Value::Quoted(text) => text,
                        Value::List(_) | Value::Table(_) => {
                            let what = if matches!(node.value, Value::List(_)) { "a list" } else { "a mapping" };
                            *pending = Some(Key { text: String::new(), line });
                            self.faults.push((line, format!("a key is {} here; a key is a text", what)));
                            return;
                        }
                    };
                    let first = seen.get(&text).copied();
                    seen.entry(text.clone()).or_insert(line);
                    *pending = Some(Key { text: text.clone(), line });
                    if let Some(first) = first {
                        self.faults.push((line, format!("the key `{}` is written twice (first on line {})", text, first)));
                    }
                }
            },
        }
    }

    /// Opens a list or a table, unless it is nested too deep.
    fn start(&mut self, mark: Marker, anchor: usize, tag: &Option<Tag>, open: Open) -> Result<(), (usize, String)> {
        if self.open.len() >= DEEPEST {
            return Err((mark.line(), format!("it is nested too deep to be read (more than {} levels)", DEEPEST)));
        }
        self.anchor_and_tag(mark.line(), anchor, tag);
        self.open.push(open);
        Ok(())
    }

    fn event(&mut self, event: Event, mark: Marker) -> Result<bool, (usize, String)> {
        match event {
            Event::StreamEnd => return Ok(false),
            Event::DocumentStart => {
                self.documents += 1;
                if self.documents > 1 {
                    return Err((mark.line(), "it holds more than one document; one is read".to_string()));
                }
            }
            Event::Scalar(value, style, anchor, tag) => {
                let line = self.scalar_line(&value, style, &mark);
                self.anchor_and_tag(line, anchor, &tag);
                let key_expected = matches!(self.open.last(), Some(Open::Table(_, _, None, _)));
                let value = if style == TScalarStyle::Plain {
                    if value == "<<" {
                        if key_expected {
                            self.fault(line, "a merge key (`<<`) is not read: write the keys out".to_string());
                        } else {
                            self.fault(line, "`<<` written plain is a merge key in YAML 1.1: put the value in quotes".to_string());
                        }
                    }
                    let read = reading(&value);
                    if read == Reading::Date && !key_expected {
                        if let Some((year, month, day, time_ok)) = timestamp(&value) {
                            if !real_date(year, month, day) || !time_ok {
                                self.fault(
                                    line,
                                    format!("`{}` written plain is a date in YAML 1.1, and no such date is. Put the value in quotes.", value),
                                );
                            }
                        }
                    }
                    Value::Plain(value, read)
                } else {
                    Value::Quoted(value)
                };
                self.place(Node { value, line });
            }
            Event::Alias(_) => {
                let line = mark.line();
                self.fault(line, "an alias (`*`) is not read: write the value out where it is used".to_string());
                self.place(Node { value: Value::Quoted(String::new()), line });
            }
            Event::SequenceStart(anchor, tag) => self.start(mark, anchor, &tag, Open::List(Vec::new(), mark.line()))?,
            Event::MappingStart(anchor, tag) => {
                self.start(mark, anchor, &tag, Open::Table(Vec::new(), mark.line(), None, HashMap::new()))?
            }
            Event::SequenceEnd | Event::MappingEnd => {
                let node = match self.open.pop() {
                    Some(Open::List(items, line)) => Node { value: Value::List(items), line },
                    Some(Open::Table(entries, line, _, _)) => Node { value: Value::Table(entries), line },
                    None => return Err((mark.line(), "the library gave an end with no start".to_string())),
                };
                self.place(node);
            }
            Event::StreamStart | Event::DocumentEnd | Event::Nothing => {}
        }
        Ok(true)
    }
}

/// Reads the events of a text into one value, or the faults found, each with its line.
fn build(text: &str) -> Result<Option<Node>, Vec<(usize, String)>> {
    let mut builder = Builder { text, content: None, open: Vec::new(), root: None, documents: 0, faults: Vec::new() };
    // Events are taken one by one: the library's own `load` calls itself for each level of nesting.
    let mut parser = Parser::new_from_str(text);
    loop {
        match parser.next_token() {
            Ok((event, mark)) => match builder.event(event, mark) {
                Ok(true) => {}
                Ok(false) => break,
                Err(fault) => {
                    builder.faults.push(fault);
                    return Err(builder.faults);
                }
            },
            Err(error) => {
                builder.faults.push((error.marker().line(), scan_words(&error)));
                return Err(builder.faults);
            }
        }
    }
    if builder.faults.is_empty() {
        Ok(builder.root)
    } else {
        Err(builder.faults)
    }
}

/// The text of an error of the system, without the number Rust adds to it.
pub(crate) fn system_words(error: &std::io::Error) -> String {
    let words = error.to_string();
    match words.find(" (os error") {
        Some(at) => words[..at].to_string(),
        None => words,
    }
}

/// Reads a YAML file into a `Document`, or refuses it with the file named.
pub fn load(path: impl AsRef<Path>) -> Result<Document, YamlError> {
    let path = path.as_ref();
    let bytes = crate::bounded::read_plain(path, crate::bounded::TEXT_CAP)
        .map_err(|fault| YamlError::new(format!("{}: cannot be read ({})", path.display(), fault)))?;
    load_bytes(path, bytes)
}

/// As `load`, for a file outside the working tree that a session cannot write (the organisation's file, the
/// installed tables): it is read where it is kept as a link, through the cap.
pub fn load_following(path: impl AsRef<Path>) -> Result<Document, YamlError> {
    let path = path.as_ref();
    let bytes = crate::bounded::read_following(path, crate::bounded::TEXT_CAP)
        .map_err(|fault| YamlError::new(format!("{}: cannot be read ({})", path.display(), fault)))?;
    load_bytes(path, bytes)
}

/// As `load`, for bytes that were read already (named by `path` in the faults): the file is read once,
/// and what is held to its form is what was hashed.
pub fn load_bytes(path: impl AsRef<Path>, bytes: Vec<u8>) -> Result<Document, YamlError> {
    let path = path.as_ref();
    let shown = path.display();
    let text = String::from_utf8(bytes).map_err(|_| YamlError::new(format!("{}: is not text in UTF-8", shown)))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let mut before_document = true;
    for (index, line) in lines(text).into_iter().enumerate() {
        let line_number = index + 1;
        if before_document {
            let first = line.trim_start_matches([' ', '\t']);
            if first == "..." || first.starts_with("... ") || first.starts_with("...\t") {
                return Err(YamlError::new(format!(
                    "{}, line {}: a document end (`...`) stands before the document; one document is read",
                    shown, line_number
                )));
            }
            before_document = first.is_empty() || first.starts_with('#') || first.starts_with('%');
        }
        if let Some(c) = line.chars().find(|c| !printable(*c)) {
            return Err(YamlError::new(format!(
                "{}, line {}: holds the character U+{:04X}, which YAML does not allow",
                shown, line_number, c as u32
            )));
        }
        if line.contains('\t') {
            // PyYAML stops at a tab wherever a token could start, and YAML 1.2 takes one as a space.
            return Err(YamlError::new(format!(
                "{}, line {}: holds a tab, which is not read: indent and separate with spaces, and write `\\t` in double quotes for a tab in a text",
                shown, line_number
            )));
        }
        if let Some(c) = line.chars().find(|c| break_of_one_version(*c)) {
            return Err(YamlError::new(format!(
                "{}, line {}: holds the character U+{:04X}, which YAML 1.1 reads as a line break and YAML 1.2 does not: write a line break or a space",
                shown, line_number, c as u32
            )));
        }
        if let Some(version) = other_version(line) {
            return Err(YamlError::new(format!("{}, line {}: YAML version `{}` is not read; version 1 is", shown, line_number, version)));
        }
    }
    // The library is not to panic on any text; should it, the file is refused all the same.
    let built = panic::catch_unwind(AssertUnwindSafe(|| build(text)))
        .unwrap_or_else(|_| Err(vec![(0, "the YAML library failed on it".to_string())]));
    let root = match built {
        Ok(root) => root,
        Err(faults) => {
            let mut lines: Vec<String> = faults
                .iter()
                .take(FAULTS_NAMED)
                .map(|(line, words)| match line {
                    0 => format!("{}: {}", shown, words),
                    line => format!("{}, line {}: {}", shown, line, words),
                })
                .collect();
            if faults.len() > FAULTS_NAMED {
                lines.push(format!("{}: and {} more", shown, faults.len() - FAULTS_NAMED));
            }
            return Err(YamlError::new(lines.join("\n")));
        }
    };
    let root = match root {
        Some(root) => root,
        None => return Err(YamlError::new(format!("{}: holds no YAML document; a mapping of keys and values is expected", shown))),
    };
    let what = match &root.value {
        Value::Table(_) => None,
        Value::List(_) => Some("a list".to_string()),
        Value::Quoted(_) => Some("a text".to_string()),
        Value::Plain(_, read) => Some(read.words().to_string()),
    };
    if let Some(what) = what {
        return Err(YamlError::new(format!(
            "{}, line {} (the top of the file): a mapping of keys and values is expected, but {} is here",
            shown, root.line, what
        )));
    }
    Ok(Document { path: path.to_path_buf(), root })
}

/// What a file holds, with the means to hold it to its fields and to name a place in it.
#[derive(Clone, Debug)]
pub struct Document {
    path: PathBuf,
    root: Node,
}

impl Document {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn root(&self) -> &Node {
        &self.root
    }

    /// A place as words: `roles.build.step`, or the top of the file.
    pub fn dotted(place: &[Part]) -> String {
        if place.is_empty() {
            "the top of the file".to_string()
        } else {
            place.iter().map(Part::to_string).collect::<Vec<_>>().join(".")
        }
    }

    /// The value at a place and the line of its key, if it is there.
    fn find(&self, place: &[Part]) -> Option<(&Node, usize)> {
        let mut node = &self.root;
        let mut key_line = node.line;
        for part in place {
            match (&node.value, part) {
                (Value::Table(entries), Part::Key(name)) => {
                    let (key, value) = entries.iter().find(|(key, _)| &key.text == name)?;
                    key_line = key.line;
                    node = value;
                }
                (Value::List(items), Part::Index(index)) => {
                    node = items.get(*index)?;
                    key_line = node.line;
                }
                _ => return None,
            }
        }
        Some((node, key_line))
    }

    pub fn has(&self, place: &[Part]) -> bool {
        self.find(place).is_some()
    }

    /// The line a value at a place starts on, if it is there.
    pub fn line(&self, place: &[Part]) -> Option<usize> {
        self.find(place).map(|(node, _)| node.line)
    }

    /// The refusal of the file at a place, with its line: the line of the key when `key` is true.
    pub fn fail(&self, place: &[Part], message: &str, key: bool) -> YamlError {
        let line = match self.find(place) {
            Some((node, key_line)) => format!(", line {}", if key { key_line } else { node.line }),
            None => String::new(),
        };
        YamlError::new(format!("{}{} ({}): {}", self.path.display(), line, Self::dotted(place), message))
    }

    fn value(&self, place: &[Part]) -> Result<&Node, YamlError> {
        match self.find(place) {
            Some((node, _)) => Ok(node),
            None => Err(self.fail(place, "nothing is written here", false)),
        }
    }

    /// A text. A value written plain that YAML may read as something else is refused with a word on quotes.
    pub fn text(&self, place: &[Part]) -> Result<String, YamlError> {
        let text = match &self.value(place)?.value {
            Value::Quoted(text) => text.clone(),
            Value::Plain(text, Reading::Text) => text.clone(),
            Value::Plain(text, read) => {
                let written = if text.is_empty() { String::new() } else { format!(" (written as `{}`)", text) };
                return Err(self.fail(
                    place,
                    &format!(
                        "a text is expected, but {}{}. Put the value in quotes.",
                        read.said(text),
                        written
                    ),
                    false,
                ));
            }
            Value::List(_) => return Err(self.fail(place, "a text is expected, but a list is here", false)),
            Value::Table(_) => return Err(self.fail(place, "a text is expected, but a mapping is here", false)),
        };
        if crate::detect::strip(&text).is_empty() {
            return Err(self.fail(place, "a text is expected, and there is nothing in it", false));
        }
        Ok(text)
    }

    /// A truth value, written plain as `true` or `false` and nothing else: `yes`, `no`, `on`, `off` and every other
    /// spelling are refused, since YAML 1.1 and 1.2 read them differently.
    pub fn boolean(&self, place: &[Part]) -> Result<bool, YamlError> {
        match &self.value(place)?.value {
            Value::Plain(text, Reading::Boolean) if text == "true" => Ok(true),
            Value::Plain(text, Reading::Boolean) if text == "false" => Ok(false),
            Value::Plain(text, read) => Err(self.fail(place, &format!("`true` or `false` is expected, but {} is here (written as `{}`)", read.words(), text), false)),
            Value::Quoted(_) => Err(self.fail(place, "`true` or `false` is expected, but a text in quotes is here", false)),
            Value::List(_) => Err(self.fail(place, "`true` or `false` is expected, but a list is here", false)),
            Value::Table(_) => Err(self.fail(place, "`true` or `false` is expected, but a mapping is here", false)),
        }
    }

    /// A whole number, and at least `minimum` when it is given.
    pub fn whole(&self, place: &[Part], minimum: Option<i64>) -> Result<i64, YamlError> {
        let need = match minimum {
            Some(minimum) => format!("a whole number of at least {} is expected", minimum),
            None => "a whole number is expected".to_string(),
        };
        let what = match &self.value(place)?.value {
            Value::Plain(_, Reading::Whole(number)) => {
                return match minimum {
                    Some(minimum) if *number < minimum => {
                        Err(self.fail(place, &format!("{}, and {} is less", need, number), false))
                    }
                    _ => Ok(*number),
                }
            }
            Value::Plain(text, read) => format!("{} here", read.said(text)),
            Value::Quoted(_) => "a text in quotes is here".to_string(),
            Value::List(_) => "a list is here".to_string(),
            Value::Table(_) => "a mapping is here".to_string(),
        };
        Err(self.fail(place, &format!("{}, but {}", need, what), false))
    }

    /// A list of texts, each held as `text` is. An empty list only when `empty`.
    pub fn texts(&self, place: &[Part], empty: bool) -> Result<Vec<String>, YamlError> {
        let count = match &self.value(place)?.value {
            Value::List(items) => items.len(),
            Value::Plain(_, read) => {
                return Err(self.fail(place, &format!("a list of texts is expected, but {} is here", read.words()), false))
            }
            Value::Quoted(_) => return Err(self.fail(place, "a list of texts is expected, but a text is here", false)),
            Value::Table(_) => return Err(self.fail(place, "a list of texts is expected, but a mapping is here", false)),
        };
        if count == 0 && !empty {
            return Err(self.fail(place, "a list with something in it is expected", false));
        }
        (0..count)
            .map(|index| {
                let mut at = place.to_vec();
                at.push(Part::Index(index));
                self.text(&at)
            })
            .collect()
    }

    /// The keys of a mapping, as they are written, in the order of the file.
    pub fn table(&self, place: &[Part]) -> Result<Vec<String>, YamlError> {
        let what = match &self.value(place)?.value {
            Value::Table(entries) => return Ok(entries.iter().map(|(key, _)| key.text.clone()).collect()),
            Value::Plain(_, read) => read.words(),
            Value::Quoted(_) => "a text",
            Value::List(_) => "a list",
        };
        Err(self.fail(place, &format!("a mapping of keys and values is expected, but {} is here", what), false))
    }

    /// Refuses a key that is not in `allowed`, naming where it stands.
    pub fn only(&self, place: &[Part], allowed: &[&str]) -> Result<(), YamlError> {
        for key in self.table(place)? {
            if !allowed.contains(&key.as_str()) {
                let mut at = place.to_vec();
                at.push(Part::Key(key.clone()));
                let known = allowed.iter().map(|name| format!("`{}`", name)).collect::<Vec<_>>().join(", ");
                return Err(self.fail(
                    &at,
                    &format!("unknown key `{}` in {}; the keys known here are: {}", key, Self::dotted(place), known),
                    true,
                ));
            }
        }
        Ok(())
    }

    /// Refuses a mapping that lacks one of `fields`, naming the field.
    pub fn need(&self, place: &[Part], fields: &[&str]) -> Result<(), YamlError> {
        let present = self.table(place)?;
        for field in fields {
            if !present.iter().any(|key| key == field) {
                return Err(self.fail(place, &format!("lacks the field `{}`", field), false));
            }
        }
        Ok(())
    }
}
