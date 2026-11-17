//! The one loader of YAML in `wb`: plain values only, a key written twice refused, no panic.
//! Every test of tests/test_yamlfile.py is here under the name it has, and then the tests of #23.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use watchbill::yamlfile::{self, Document, Part};

fn src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// A place in a file: keys, then list positions where they are given as numbers.
fn at(keys: &[&str]) -> Vec<Part> {
    keys.iter().map(|key| Part::from(*key)).collect()
}

struct Folder {
    folder: common::Folder,
}

impl Folder {
    fn new() -> Folder {
        Folder { folder: common::folder_named("tables") }
    }

    fn write_bytes(&self, bytes: &[u8], name: &str) -> PathBuf {
        let path = self.folder.path.join(name);
        fs::write(&path, bytes).expect("the file could not be written");
        path
    }

    fn write(&self, text: &str, name: &str) -> PathBuf {
        self.write_bytes(text.as_bytes(), name)
    }

    fn load(&self, text: &str) -> Document {
        let path = self.write(text, "table.yml");
        match yamlfile::load(&path) {
            Ok(doc) => doc,
            Err(error) => panic!("the file was refused: {}", error),
        }
    }

    /// The file is refused when it is read, and the message names it and is no panic.
    fn refused(&self, text: &str, name: &str) -> String {
        let path = self.write(text, name);
        match yamlfile::load(&path) {
            Ok(_) => panic!("the file was read: {:?}", text),
            Err(error) => {
                let message = error.to_string();
                assert!(message.contains(name), "{}", message);
                assert!(!message.contains("panicked"), "{}", message);
                message
            }
        }
    }
}

/// What the loader makes of `v: <written>` when a text is asked for at `v`.
fn text_of(written: &str) -> Result<String, String> {
    let folder = Folder::new();
    let path = folder.write(&format!("ok: 1\nv: {}\n", written), "table.yml");
    let doc = yamlfile::load(&path).map_err(|error| error.to_string())?;
    doc.text(&at(&["v"])).map_err(|error| error.to_string())
}

/// Every file below `folder` whose name ends in `ending`.
fn files(folder: &Path, ending: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut todo = vec![folder.to_path_buf()];
    while let Some(next) = todo.pop() {
        for entry in fs::read_dir(&next).expect("the folder could not be read") {
            let path = entry.expect("the folder could not be read").path();
            if path.is_dir() {
                todo.push(path);
            } else if path.to_string_lossy().ends_with(ending) {
                found.push(path);
            }
        }
    }
    found
}

// The tests of tests/test_yamlfile.py, TestLoader.

#[test]
fn test_WB_RULE_18_a_plain_file_is_read_into_plain_values() {
    let folder = Folder::new();
    let doc = folder.load("a: 1\nb: [x, y]\nc:\n  d: text\n");
    assert_eq!(doc.table(&[]).unwrap(), vec!["a", "b", "c"]);
    assert_eq!(doc.whole(&at(&["a"]), None).unwrap(), 1);
    assert_eq!(doc.texts(&at(&["b"]), false).unwrap(), vec!["x", "y"]);
    assert_eq!(doc.table(&at(&["c"])).unwrap(), vec!["d"]);
    assert_eq!(doc.text(&at(&["c", "d"])).unwrap(), "text");
}

#[test]
fn test_WB_RULE_18_a_tag_that_builds_a_python_object_is_refused() {
    Folder::new().refused("a: !!python/object/apply:os.system [echo hello]\n", "table.yml");
}

#[test]
fn test_WB_RULE_18_a_tag_that_builds_a_set_or_bytes_is_refused() {
    let folder = Folder::new();
    folder.refused("a: !!set {x, y}\n", "set.yml");
    folder.refused("a: !!binary aGVsbG8=\n", "binary.yml");
}

#[test]
fn test_WB_RULE_18_nothing_but_the_loader_imports_the_library() {
    let sources = files(&src(), ".rs");
    assert!(sources.iter().any(|path| path.ends_with("yamlfile.rs")));
    for path in sources {
        if path.file_name().unwrap() != "yamlfile.rs" {
            let text = fs::read_to_string(&path).unwrap();
            assert!(!text.contains("yaml_rust2"), "{}", path.display());
        }
    }
}

#[test]
fn test_WB_RULE_15_a_file_that_is_not_yaml_is_refused_with_the_file_named() {
    let message = Folder::new().refused("a: [unclosed\nb: 1\n", "broken.yml");
    assert!(message.contains("line"), "{}", message);
}

#[test]
fn test_WB_RULE_15_a_file_that_is_not_text_is_refused_with_the_file_named() {
    let folder = Folder::new();
    let path = folder.write_bytes(b"a: \xff\xfe\n", "bytes.yml");
    let error = yamlfile::load(&path).err().expect("the file was read");
    assert!(error.to_string().contains("bytes.yml"), "{}", error);
}

#[test]
fn test_WB_RULE_15_a_value_that_yaml_cannot_build_is_refused_with_the_file_and_the_line() {
    for text in ["step: 2026-02-30\n", "a: !!int abc\n", "a: !!float abc\n", "a: !!bool maybe\n", "a: !!timestamp xyz\n"] {
        let message = Folder::new().refused(&format!("ok: 1\n{}", text), "table.yml");
        assert!(message.contains("line 2"), "{}: {}", text, message);
    }
}

#[test]
fn test_WB_RULE_15_a_file_nested_too_deep_is_refused_with_the_file_named() {
    let folder = Folder::new();
    folder.refused(&format!("a: {}\n", "[".repeat(5000)), "table.yml");
    folder.refused(&format!("a: {}{}\n", "[".repeat(5000), "]".repeat(5000)), "deep.yml");
}

#[test]
fn test_WB_RULE_26_a_merge_key_is_refused_with_the_line() {
    let message = Folder::new().refused("base: &b {x: 1}\nuse:\n  <<: *b\n  y: 2\n", "table.yml");
    assert!(message.contains("line 3"), "{}", message);
    assert!(message.contains("merge"), "{}", message);
}

#[test]
fn test_WB_RULE_26_a_key_in_a_merge_and_again_in_the_mapping_is_not_taken_in_silence() {
    Folder::new().refused("base: &b {x: 1}\nuse: {<<: *b, x: 2}\n", "table.yml");
}

#[test]
fn test_WB_RULE_26_a_quoted_double_angle_is_a_plain_key() {
    let doc = Folder::new().load("\"<<\": 1\n");
    assert_eq!(doc.table(&[]).unwrap(), vec!["<<"]);
    assert_eq!(doc.whole(&at(&["<<"]), None).unwrap(), 1);
}

#[test]
fn test_WB_RULE_18_a_tag_that_builds_ordered_pairs_is_refused() {
    let folder = Folder::new();
    folder.refused("a: !!omap [x: 1]\n", "omap.yml");
    folder.refused("a: !!pairs [x: 1]\n", "pairs.yml");
}

#[test]
fn test_WB_RULE_21_a_file_that_is_not_there_is_refused_with_the_file_named() {
    let folder = Folder::new();
    let path = folder.folder.path.join("missing.yml");
    let error = yamlfile::load(&path).err().expect("a file that is not there was read");
    assert!(error.to_string().contains("missing.yml"), "{}", error);
    assert!(!error.to_string().contains("panicked"), "{}", error);
}

#[test]
fn test_WB_RULE_16_a_key_written_twice_at_the_top_is_refused() {
    let message = Folder::new().refused("a: 1\nb: 2\na: 3\n", "table.yml");
    assert!(message.contains("`a`"), "{}", message);
    assert!(message.contains("line 3"), "{}", message);
}

#[test]
fn test_WB_RULE_16_a_key_written_twice_in_a_nested_mapping_is_refused() {
    let message = Folder::new().refused("outer:\n  inner:\n    k: 1\n    j: 2\n    k: 3\n", "table.yml");
    assert!(message.contains("line 5"), "{}", message);
}

#[test]
fn test_WB_RULE_16_a_key_written_twice_in_a_mapping_inside_a_list_is_refused() {
    let message = Folder::new().refused("items:\n  - x: 1\n  - y: 1\n    y: 2\n", "table.yml");
    assert!(message.contains("line 4"), "{}", message);
}

#[test]
fn test_WB_RULE_16_the_same_key_in_two_mappings_is_not_a_duplicate() {
    let doc = Folder::new().load("a:\n  k: 1\nb:\n  k: 2\n");
    assert_eq!(doc.whole(&at(&["a", "k"]), None).unwrap(), 1);
    assert_eq!(doc.whole(&at(&["b", "k"]), None).unwrap(), 2);
}

// The tests of tests/test_yamlfile.py, TestTypes.

#[test]
fn test_WB_RULE_17_a_text_read_as_a_boolean_is_refused_and_quotes_are_asked_for() {
    let doc = Folder::new().load("prompt: no\n");
    let message = doc.text(&at(&["prompt"])).err().expect("`no` was taken as a text").to_string();
    for part in ["quotes", "prompt", "table.yml", "line 1"] {
        assert!(message.contains(part), "{}: {}", part, message);
    }
}

#[test]
fn test_WB_RULE_17_a_text_read_as_a_number_is_refused_and_quotes_are_asked_for() {
    let doc = Folder::new().load("version: 1.10\n");
    let message = doc.text(&at(&["version"])).err().expect("`1.10` was taken as a text").to_string();
    assert!(message.contains("quotes"), "{}", message);
}

#[test]
fn test_WB_RULE_17_a_quoted_value_is_a_text() {
    let doc = Folder::new().load("prompt: \"no\"\nversion: \"1.10\"\n");
    assert_eq!(doc.text(&at(&["prompt"])).unwrap(), "no");
    assert_eq!(doc.text(&at(&["version"])).unwrap(), "1.10");
}

#[test]
fn test_WB_RULE_17_a_whole_number_is_held_to_a_whole_number() {
    let doc = Folder::new().load("a: 2\nb: true\nc: 1.5\nd: '2'\n");
    assert_eq!(doc.whole(&at(&["a"]), None).unwrap(), 2);
    for key in ["b", "c", "d"] {
        assert!(doc.whole(&at(&[key]), None).is_err(), "{}", key);
    }
}

#[test]
fn test_WB_RULE_17_a_list_of_texts_is_held_to_texts() {
    let doc = Folder::new().load("a: [x, y]\nb: [x, 1]\nc: x\n");
    assert_eq!(doc.texts(&at(&["a"]), false).unwrap(), vec!["x", "y"]);
    let message = doc.texts(&at(&["b"]), false).err().expect("a number was taken as a text").to_string();
    assert!(message.contains("quotes"), "{}", message);
    assert!(doc.texts(&at(&["c"]), false).is_err());
}

// The tests of #23.

#[test]
fn test_WB_RULE_30_an_alias_is_refused_with_the_line() {
    let message = Folder::new().refused("base: x\nother: 1\ncopy: *b\n", "table.yml");
    assert!(message.contains("line 3"), "{}", message);
    let message = Folder::new().refused("base: &b x\nother: 1\ncopy: *b\n", "table.yml");
    assert!(message.contains("line 3"), "{}", message);
    assert!(message.contains("alias"), "{}", message);
}

#[test]
fn test_WB_RULE_30_an_anchor_is_refused_with_the_line_also_when_no_alias_names_it() {
    for text in ["ok: 1\na: &x text\n", "ok: 1\na: &x [1, 2]\n", "ok: 1\na: &x {b: 1}\n", "ok: 1\n&x a: 1\n", "ok: 1\na: &x\n"] {
        let message = Folder::new().refused(text, "table.yml");
        assert!(message.contains("line 2"), "{:?}: {}", text, message);
        assert!(message.contains("anchor"), "{:?}: {}", text, message);
    }
}

#[test]
fn test_WB_RULE_17_plain_values_that_two_versions_of_yaml_read_differently_are_refused_with_quotes_asked_for() {
    for written in ["no", "yes", "on", "off", "~", "1.10", "0x1f", "017", "1_000", "2026-02-30", "12:30"] {
        let message = text_of(written).err().unwrap_or_else(|| panic!("`{}` written plain was taken as a text", written));
        assert!(message.contains("quotes"), "{}: {}", written, message);
        assert!(message.contains("table.yml"), "{}: {}", written, message);
        assert!(message.contains("line 2"), "{}: {}", written, message);
    }
}

#[test]
fn test_WB_RULE_17_the_same_values_in_quotes_are_taken_as_the_text_they_are() {
    for written in ["no", "yes", "on", "off", "~", "1.10", "0x1f", "017", "1_000", "2026-02-30", "12:30"] {
        assert_eq!(text_of(&format!("\"{}\"", written)), Ok(written.to_string()), "double quotes");
        assert_eq!(text_of(&format!("'{}'", written)), Ok(written.to_string()), "single quotes");
    }
}

#[test]
fn test_WB_RULE_17_other_plain_values_that_yaml_may_read_as_another_type_are_refused() {
    for written in [
        "Yes", "NO", "True", "FALSE", "Off", "yES", "oN", "null", "Null", "NULL", "", "1.5", "1.", ".5", "1e3", "1.5e-3",
        "+1.0", ".inf", "-.Inf", ".NaN", "0o17", "0b101", "0x_1F", "00", "-017", "1:20:30", "190:20:30.15", "2026-11-17",
        "2026-11-17 10:00:00", "2026-11-17T10:00:00Z", "2026-1-7t1:00:00.5 +01:00", "=",
    ] {
        let message = text_of(written).err().unwrap_or_else(|| panic!("`{}` written plain was taken as a text", written));
        assert!(message.contains("quotes"), "{}: {}", written, message);
    }
}

#[test]
fn test_WB_RULE_17_a_plain_value_that_no_version_reads_as_another_type_is_a_text() {
    for written in [
        "brakes", "0.0.1", "1.2.3-rc1", "claude-sonnet-5-5", "3 things", "x0", "a.b_c", "12:75", "2026-11", "1e", "0x", "yess",
        "nope", "roles/tester.md", "_1", "1-2", "2026-11-17 noon", "y", "n", "Y", "N",
    ] {
        assert_eq!(text_of(written), Ok(written.to_string()), "{}", written);
    }
}

#[test]
fn test_WB_RULE_17_a_whole_number_is_only_one_that_every_version_reads_alike() {
    let doc = Folder::new().load("a: 0\nb: -3\nc: +7\nd: 017\ne: 1_000\nf: 0x1f\ng: 99999999999999999999\n");
    assert_eq!(doc.whole(&at(&["a"]), None).unwrap(), 0);
    assert_eq!(doc.whole(&at(&["b"]), None).unwrap(), -3);
    assert_eq!(doc.whole(&at(&["c"]), None).unwrap(), 7);
    for key in ["d", "e", "f", "g"] {
        assert!(doc.whole(&at(&[key]), None).is_err(), "{}", key);
    }
    let message = doc.whole(&at(&["a"]), Some(1)).err().expect("0 was taken as at least 1").to_string();
    assert!(message.contains("line 1"), "{}", message);
}

#[test]
fn test_WB_RULE_17_a_block_text_is_a_text() {
    let doc = Folder::new().load("a: |\n  no\nb: >\n  1.10\nc: plain\n  over two lines\n");
    assert_eq!(doc.text(&at(&["a"])).unwrap(), "no\n");
    assert_eq!(doc.text(&at(&["b"])).unwrap(), "1.10\n");
    assert_eq!(doc.text(&at(&["c"])).unwrap(), "plain over two lines");
}

#[test]
fn test_WB_RULE_15_an_empty_document_is_refused_with_the_file_named() {
    let folder = Folder::new();
    folder.refused("", "empty.yml");
    folder.refused("# nothing but a comment\n", "comment.yml");
    folder.refused("---\n", "dashes.yml");
    folder.refused("~\n", "null.yml");
}

#[test]
fn test_WB_RULE_15_a_document_whose_top_is_a_list_is_refused_with_the_file_named() {
    let folder = Folder::new();
    folder.refused("- a\n- b\n", "list.yml");
    folder.refused("just text\n", "text.yml");
}

#[test]
fn test_WB_RULE_15_a_file_with_two_documents_is_refused_with_the_file_named() {
    let message = Folder::new().refused("a: 1\n---\nb: 2\n", "two.yml");
    assert!(message.contains("line 2"), "{}", message);
}

#[test]
fn test_WB_RULE_15_block_nesting_too_deep_is_refused_and_does_not_end_the_program() {
    let folder = Folder::new();
    folder.refused(&format!("{}x\n", "- ".repeat(100_000)), "list.yml");
    let mut text = String::new();
    for depth in 0..1000 {
        text.push_str(&" ".repeat(depth));
        text.push_str("k:\n");
    }
    folder.refused(&text, "mapping.yml");
}

#[test]
fn test_WB_RULE_15_nesting_that_is_not_too_deep_is_read() {
    let mut text = String::new();
    for depth in 0..20 {
        text.push_str(&" ".repeat(depth));
        text.push_str("k:\n");
    }
    text.push_str(&" ".repeat(20));
    text.push_str("end: 1\n");
    let doc = Folder::new().load(&text);
    let mut place = at(&["k"; 20]);
    place.push(Part::from("end"));
    assert_eq!(doc.whole(&place, None).unwrap(), 1);
}

#[test]
fn test_WB_RULE_15_a_character_that_yaml_does_not_allow_is_refused_with_the_line() {
    for text in ["ok: 1\na: x\u{7}y\n", "ok: 1\na: \"x\u{0}y\"\n", "ok: 1\na: x\u{1b}\n", "ok: 1\na: \u{fffe}\n"] {
        let message = Folder::new().refused(text, "table.yml");
        assert!(message.contains("line 2"), "{:?}: {}", text, message);
    }
}

#[test]
fn test_WB_RULE_15_a_byte_order_mark_and_crlf_line_ends_are_read() {
    let folder = Folder::new();
    let path = folder.write_bytes(b"\xef\xbb\xbfa: 1\r\nb: text\r\n", "table.yml");
    let doc = yamlfile::load(&path).unwrap();
    assert_eq!(doc.table(&[]).unwrap(), vec!["a", "b"]);
    assert_eq!(doc.text(&at(&["b"])).unwrap(), "text");
}

#[test]
fn test_WB_RULE_18_a_tag_on_a_key_a_mapping_or_a_list_is_refused_with_the_line() {
    for text in [
        "ok: 1\n!!str a: 1\n",
        "ok: 1\na: !!map {b: 1}\n",
        "ok: 1\na: !!seq [1]\n",
        "ok: 1\na: !!str text\n",
        "ok: 1\na: !local text\n",
        "ok: 1\na: ! text\n",
    ] {
        let message = Folder::new().refused(text, "table.yml");
        assert!(message.contains("line 2"), "{:?}: {}", text, message);
        assert!(message.contains("tag"), "{:?}: {}", text, message);
    }
}

#[test]
fn test_WB_RULE_16_a_key_written_twice_deep_down_is_refused_with_its_line() {
    let message = Folder::new().refused("a:\n  - b:\n      c: {d: 1, e: 2}\n      c: 3\n", "table.yml");
    assert!(message.contains("line 4"), "{}", message);
    assert!(message.contains("`c`"), "{}", message);
    let message = Folder::new().refused("a: {b: [{c: 1, c: 2}]}\n", "table.yml");
    assert!(message.contains("line 1"), "{}", message);
}

#[test]
fn test_WB_RULE_16_a_key_written_once_plain_and_once_quoted_is_written_twice() {
    Folder::new().refused("a: 1\n'a': 2\n", "table.yml");
}

#[test]
fn test_WB_RULE_26_a_merge_key_without_an_anchor_is_refused_with_the_line() {
    let message = Folder::new().refused("a:\n  b: 1\n  <<: {c: 2}\n", "table.yml");
    assert!(message.contains("line 3"), "{}", message);
    assert!(message.contains("merge"), "{}", message);
}

#[test]
fn test_WB_RULE_15_a_key_that_is_a_list_or_a_mapping_is_refused_with_the_line() {
    for text in ["ok: 1\n? [a, b]\n: 1\n", "ok: 1\n? {a: b}\n: 1\n", "ok: 1\n[a]: 1\n"] {
        let message = Folder::new().refused(text, "table.yml");
        assert!(message.contains("line 2"), "{:?}: {}", text, message);
    }
}

#[test]
fn test_WB_RULE_15_no_fault_of_a_file_ends_in_a_panic() {
    let folder = Folder::new();
    let broken = [
        "a: [", "a: {", "a: ]", "a: }", "a: \"", "a: '", "- a\nb: 1", "a:\n- b\n c", "? ", ": ", "&", "*", "*x", "!", "!!",
        "%YAML 1.1\n", "%YAML 2.0\n---\na: 1\n", "%TAG ! x\n", "---\n...\n---\n", "a: |\n x\n  y\n z", "a: >-\n", "a: |9\n x",
        "\t", "a:\tb", "a: b: c", "- - - - ", "[[[[]]]]: 1", "{a: [b, {c: [d", "a: 'x\n\ny'", "a: \"\\x\"", "a: \"\\u12\"",
        "a: @b", "a: `b", "a: %b", "{? a}", "[? a: b]", "a: !<tag:yaml.org,2002:str> x", "a: !e!x y", "\u{feff}\u{feff}a: 1",
        "a: 1\n\u{feff}b: 2",
    ];
    for (index, text) in broken.iter().enumerate() {
        let path = folder.write(text, &format!("case{}.yml", index));
        // A panic ends this test; an answer of either kind is what is asked.
        if let Err(error) = yamlfile::load(&path) {
            assert!(error.to_string().contains(&format!("case{}.yml", index)), "{:?}: {}", text, error);
        }
    }
}

// Found by the comparison with the Python: files the Python refused and the loader took.

#[test]
fn test_WB_RULE_15_a_yaml_version_that_is_not_1_is_refused_with_the_line() {
    let message = Folder::new().refused("%YAML 2.0\n---\na: 1\n", "table.yml");
    assert!(message.contains("line 1"), "{}", message);
    for text in ["%YAML 1.1\n---\na: 1\n", "%YAML 1.2\n---\na: 1\n"] {
        assert_eq!(Folder::new().load(text).whole(&at(&["a"]), None).unwrap(), 1, "{:?}", text);
    }
}

#[test]
fn test_WB_RULE_15_a_line_break_of_yaml_1_1_alone_is_refused_with_the_line() {
    for c in ['\u{85}', '\u{2028}', '\u{2029}'] {
        let message = Folder::new().refused(&format!("ok: 1\na: x{}y\n", c), "table.yml");
        assert!(message.contains("line 2"), "{:?}: {}", c, message);
    }
}

#[test]
fn test_WB_RULE_17_an_empty_item_of_a_list_is_named_at_its_own_line() {
    for (text, line) in [("a:\n  - x\n  -\n  - y\n", "line 3"), ("a:\r\n  - x\r\n  -\r\n\r\n  - y\r\n", "line 3"), ("a: [x, , y]\n", "line 1")] {
        let doc = match yamlfile::load(Folder::new().write(text, "table.yml")) {
            Ok(doc) => doc,
            // A list with an empty item in flow style is not YAML to the library: refused, with the line.
            Err(error) => {
                assert!(error.to_string().contains(line), "{:?}: {}", text, error);
                continue;
            }
        };
        let message = doc.texts(&at(&["a"]), false).err().expect("an empty item was taken as a text").to_string();
        assert!(message.contains(line), "{:?}: {}", text, message);
        assert!(message.contains("a.1"), "{:?}: {}", text, message);
    }
}
