"""The one loader of YAML in `wb`: the safe loader, a key written twice refused, no stack trace."""

import os
import re
import sys
import tempfile
import unittest

SRC = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "src")
sys.path.insert(0, SRC)

from watchbill import yamlfile  # noqa: E402


class Folder(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)

    def write(self, text, name="table.yml"):
        path = os.path.join(self.tmp.name, name)
        with open(path, "w", encoding="utf-8") as file:
            file.write(text)
        return path

    def refused(self, text, name="table.yml"):
        path = self.write(text, name)
        with self.assertRaises(yamlfile.YamlError) as caught:
            yamlfile.load(path)
        message = str(caught.exception)
        self.assertIn(name, message)
        self.assertNotIn("Traceback", message)
        return message


class TestLoader(Folder):
    def test_WB_RULE_18_a_plain_file_is_read_into_plain_values(self):
        path = self.write("a: 1\nb: [x, y]\nc:\n  d: text\n")
        self.assertEqual(yamlfile.load(path).data, {"a": 1, "b": ["x", "y"], "c": {"d": "text"}})

    def test_WB_RULE_18_a_tag_that_builds_a_python_object_is_refused(self):
        self.refused("a: !!python/object/apply:os.system [echo hello]\n")

    def test_WB_RULE_18_a_tag_that_builds_a_set_or_bytes_is_refused(self):
        self.refused("a: !!set {x, y}\n", "set.yml")
        self.refused("a: !!binary aGVsbG8=\n", "binary.yml")

    def test_WB_RULE_18_nothing_but_the_loader_imports_the_library(self):
        folder = os.path.join(SRC, "watchbill")
        for base, _, names in os.walk(folder):
            for name in names:
                if name.endswith(".py") and name != "yamlfile.py":
                    with open(os.path.join(base, name), encoding="utf-8") as file:
                        text = file.read()
                    self.assertIsNone(
                        re.search(r"^\s*(import yaml|from yaml)\b", text, re.MULTILINE), name
                    )

    def test_WB_RULE_15_a_file_that_is_not_yaml_is_refused_with_the_file_named(self):
        message = self.refused("a: [unclosed\nb: 1\n", "broken.yml")
        self.assertIn("line", message)

    def test_WB_RULE_15_a_file_that_is_not_text_is_refused_with_the_file_named(self):
        path = os.path.join(self.tmp.name, "bytes.yml")
        with open(path, "wb") as file:
            file.write(b"a: \xff\xfe\n")
        with self.assertRaises(yamlfile.YamlError) as caught:
            yamlfile.load(path)
        self.assertIn("bytes.yml", str(caught.exception))

    def test_WB_RULE_21_a_file_that_is_not_there_is_refused_with_the_file_named(self):
        path = os.path.join(self.tmp.name, "missing.yml")
        with self.assertRaises(yamlfile.YamlError) as caught:
            yamlfile.load(path)
        self.assertIn("missing.yml", str(caught.exception))
        self.assertNotIn("Traceback", str(caught.exception))

    def test_WB_RULE_16_a_key_written_twice_at_the_top_is_refused(self):
        message = self.refused("a: 1\nb: 2\na: 3\n")
        self.assertIn("a", message)
        self.assertIn("line 3", message)

    def test_WB_RULE_16_a_key_written_twice_in_a_nested_mapping_is_refused(self):
        message = self.refused("outer:\n  inner:\n    k: 1\n    j: 2\n    k: 3\n")
        self.assertIn("line 5", message)

    def test_WB_RULE_16_a_key_written_twice_in_a_mapping_inside_a_list_is_refused(self):
        self.refused("items:\n  - x: 1\n  - y: 1\n    y: 2\n")

    def test_WB_RULE_16_the_same_key_in_two_mappings_is_not_a_duplicate(self):
        path = self.write("a:\n  k: 1\nb:\n  k: 2\n")
        self.assertEqual(yamlfile.load(path).data, {"a": {"k": 1}, "b": {"k": 2}})


class TestTypes(Folder):
    def load(self, text):
        return yamlfile.load(self.write(text))

    def test_WB_RULE_17_a_text_read_as_a_boolean_is_refused_and_quotes_are_asked_for(self):
        doc = self.load("prompt: no\n")
        with self.assertRaises(yamlfile.YamlError) as caught:
            doc.text(("prompt",))
        message = str(caught.exception)
        self.assertIn("quotes", message)
        self.assertIn("prompt", message)
        self.assertIn("table.yml", message)
        self.assertIn("line 1", message)

    def test_WB_RULE_17_a_text_read_as_a_number_is_refused_and_quotes_are_asked_for(self):
        doc = self.load("version: 1.10\n")
        with self.assertRaises(yamlfile.YamlError) as caught:
            doc.text(("version",))
        self.assertIn("quotes", str(caught.exception))

    def test_WB_RULE_17_a_quoted_value_is_a_text(self):
        doc = self.load('prompt: "no"\nversion: "1.10"\n')
        self.assertEqual(doc.text(("prompt",)), "no")
        self.assertEqual(doc.text(("version",)), "1.10")

    def test_WB_RULE_17_a_whole_number_is_held_to_a_whole_number(self):
        doc = self.load("a: 2\nb: true\nc: 1.5\nd: '2'\n")
        self.assertEqual(doc.whole(("a",)), 2)
        for key in "bcd":
            with self.assertRaises(yamlfile.YamlError):
                doc.whole((key,))

    def test_WB_RULE_17_a_list_of_texts_is_held_to_texts(self):
        doc = self.load("a: [x, y]\nb: [x, 1]\nc: x\n")
        self.assertEqual(doc.texts(("a",)), ["x", "y"])
        with self.assertRaises(yamlfile.YamlError) as caught:
            doc.texts(("b",))
        self.assertIn("quotes", str(caught.exception))
        with self.assertRaises(yamlfile.YamlError):
            doc.texts(("c",))
