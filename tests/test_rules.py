"""The rule table and the model table of the first revision, and the check that reads them."""

import copy
import json
import os
import re
import sys
import tempfile
import unittest

SRC = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "src")
sys.path.insert(0, SRC)

from watchbill import rules, yamlfile  # noqa: E402

REVISION = os.path.join(SRC, "watchbill", "revision")
ROLES = ["tester", "build", "review", "fix", "officer"]
WITH_PROMPT = ["tester", "build", "review", "fix"]


def role_row(name):
    return {
        "step": "a step of " + name,
        "produces": "what " + name + " produces",
        "reads": ["a thing to read"],
        "may": ["a thing to do"],
        "may_not_also_hold": [other for other in ROLES if other != name],
        "prompt": "roles/" + name + ".md" if name in WITH_PROMPT else "none",
    }


def good_rules():
    return {
        "review_rounds": 2,
        "always_to_a_person": [
            "change_to_an_existing_test",
            "change_under_watchbill_folder",
            "verification_of_classed_code",
            "finding_open_after_last_round",
            "merge",
        ],
        "classed_code": {"verifier": "person"},
        "officer_queues": [],
        "roles": {name: role_row(name) for name in ROLES},
    }


def good_models():
    models = {}
    for name in WITH_PROMPT:
        models[name] = {"model": "claude-sonnet-5-5", "effort": "medium"}
    models["review"] = {"model": "claude-opus-5-5", "effort": "high"}
    return {"models": models}


class Tables(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.rules = good_rules()
        self.models = good_models()

    def write(self, name, text):
        path = os.path.join(self.tmp.name, name)
        with open(path, "w", encoding="utf-8") as file:
            file.write(text)
        return path

    def paths(self, rules_text=None, models_text=None):
        # JSON is YAML, so the tables can be written from the dictionaries.
        return (
            self.write("rules.yml", rules_text if rules_text is not None else json.dumps(self.rules, indent=2) + "\n"),
            self.write("models.yml", models_text if models_text is not None else json.dumps(self.models, indent=2) + "\n"),
        )

    def load(self, **texts):
        return rules.load(*self.paths(**texts))

    def refused(self, **texts):
        with self.assertRaises(yamlfile.YamlError) as caught:
            self.load(**texts)
        message = str(caught.exception)
        self.assertNotIn("Traceback", message)
        return message


class TestFirstRevision(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.table = rules.load()

    def test_WB_RULE_1_the_rule_table_of_the_package_has_the_five_roles(self):
        self.assertEqual(sorted(self.table.roles), sorted(ROLES))

    def test_WB_RULE_1_the_files_of_the_package_are_in_the_revision_folder(self):
        self.assertEqual(rules.RULES_FILE, os.path.join(REVISION, "rules.yml"))
        self.assertEqual(rules.MODELS_FILE, os.path.join(REVISION, "models.yml"))
        self.assertTrue(os.path.isfile(rules.RULES_FILE))
        self.assertTrue(os.path.isfile(rules.MODELS_FILE))

    def test_WB_RULE_2_every_role_has_every_field_with_something_in_it(self):
        for name, role in self.table.roles.items():
            for field in ("step", "produces", "prompt"):
                self.assertTrue(getattr(role, field).strip(), name + " " + field)
            for field in ("reads", "may", "may_not_also_hold"):
                self.assertTrue(getattr(role, field), name + " " + field)

    def test_WB_RULE_2_the_four_roles_with_a_prompt_name_a_file_in_roles(self):
        for name in WITH_PROMPT:
            self.assertEqual(self.table.roles[name].prompt, "roles/" + name + ".md")

    def test_WB_RULE_2_the_roles_that_may_not_also_hold_are_each_other(self):
        for name, role in self.table.roles.items():
            self.assertEqual(sorted(role.may_not_also_hold), sorted(r for r in ROLES if r != name), name)

    def test_WB_RULE_2_no_role_may_merge(self):
        for name, role in self.table.roles.items():
            self.assertNotIn("merge", " ".join(role.may).lower(), name)

    def test_WB_RULE_3_the_first_table_gives_two_rounds(self):
        self.assertEqual(self.table.review_rounds, 2)

    def test_WB_RULE_4_what_always_goes_to_a_person_is_listed(self):
        self.assertEqual(
            sorted(self.table.always_to_a_person),
            sorted(
                [
                    "change_to_an_existing_test",
                    "change_under_watchbill_folder",
                    "verification_of_classed_code",
                    "finding_open_after_last_round",
                    "merge",
                ]
            ),
        )

    def test_WB_RULE_5_a_change_to_classed_code_needs_a_person_as_verifier(self):
        self.assertEqual(self.table.classed_code_verifier, "person")

    def test_WB_RULE_6_the_officer_holds_no_other_role_and_never_merges(self):
        officer = self.table.roles["officer"]
        self.assertEqual(sorted(officer.may_not_also_hold), sorted(WITH_PROMPT))
        self.assertNotIn("merge", " ".join(officer.may).lower())
        self.assertIn("merge", self.table.always_to_a_person)

    def test_WB_RULE_6_the_officer_has_no_prompt_yet(self):
        self.assertEqual(self.table.roles["officer"].prompt, "none")

    def test_WB_RULE_7_the_first_table_names_no_queue(self):
        self.assertEqual(self.table.officer_queues, [])

    def test_WB_RULE_8_the_model_table_has_a_row_for_the_roles_with_a_prompt_only(self):
        self.assertEqual(sorted(self.table.models), sorted(WITH_PROMPT))
        self.assertNotIn("officer", self.table.models)
        for name, role in self.table.roles.items():
            self.assertEqual(name in self.table.models, role.prompt != "none", name)

    def test_WB_RULE_9_the_models_and_efforts_of_the_first_table(self):
        for name in ("tester", "build", "fix"):
            self.assertEqual((self.table.models[name].model, self.table.models[name].effort), ("claude-sonnet-5-5", "medium"))
        self.assertEqual((self.table.models["review"].model, self.table.models["review"].effort), ("claude-opus-5-5", "high"))

    def test_WB_RULE_23_only_rules_py_names_the_two_tables(self):
        for base, _, names in os.walk(os.path.join(SRC, "watchbill")):
            for name in names:
                if name.endswith(".py") and name != "rules.py":
                    with open(os.path.join(base, name), encoding="utf-8") as file:
                        text = file.read()
                    self.assertIsNone(re.search(r"(rules|models)\.yml", text), name)


class TestShape(Tables):
    def test_WB_RULE_1_a_good_table_is_read(self):
        table = self.load()
        self.assertEqual(sorted(table.roles), sorted(ROLES))

    def test_WB_RULE_1_a_table_without_one_of_the_five_roles_is_refused_and_names_it(self):
        del self.rules["roles"]["fix"]
        for role in self.rules["roles"].values():
            role["may_not_also_hold"] = [r for r in role["may_not_also_hold"] if r != "fix"]
        self.assertIn("fix", self.refused())

    def test_WB_RULE_1_a_role_that_is_not_one_of_the_five_is_refused_and_names_it(self):
        self.rules["roles"]["captain"] = role_row("captain")
        self.assertIn("captain", self.refused())

    def test_WB_RULE_3_a_table_without_the_rounds_is_refused_and_names_the_field(self):
        del self.rules["review_rounds"]
        self.assertIn("review_rounds", self.refused())

    def test_WB_RULE_4_a_table_without_the_list_for_a_person_is_refused_and_names_it(self):
        del self.rules["always_to_a_person"]
        self.assertIn("always_to_a_person", self.refused())

    def test_WB_RULE_4_an_empty_list_for_a_person_is_refused(self):
        self.rules["always_to_a_person"] = []
        self.assertIn("always_to_a_person", self.refused())

    def test_WB_RULE_5_a_rule_for_classed_code_that_names_no_person_is_refused(self):
        self.rules["classed_code"] = {"verifier": "agent"}
        self.assertIn("classed_code", self.refused())

    def test_WB_RULE_5_a_table_without_the_rule_for_classed_code_is_refused(self):
        del self.rules["classed_code"]
        self.assertIn("classed_code", self.refused())

    def test_WB_RULE_6_a_role_that_may_merge_is_refused_and_names_the_role(self):
        self.rules["roles"]["officer"]["may"].append("merge")
        message = self.refused()
        self.assertIn("officer", message)
        self.assertIn("merge", message)

    def test_WB_RULE_6_an_officer_that_may_hold_another_role_is_refused(self):
        self.rules["roles"]["officer"]["may_not_also_hold"] = ["tester"]
        for name in ("build", "review", "fix"):
            self.rules["roles"][name]["may_not_also_hold"] = [
                r for r in self.rules["roles"][name]["may_not_also_hold"] if r != "officer"
            ]
        self.rules["roles"]["tester"]["may_not_also_hold"] = ["build", "review", "fix", "officer"]
        self.assertIn("officer", self.refused())

    def test_WB_RULE_7_a_table_without_the_queues_is_refused_and_names_the_field(self):
        del self.rules["officer_queues"]
        self.assertIn("officer_queues", self.refused())

    def test_WB_RULE_7_a_queue_is_read_as_a_name(self):
        self.rules["officer_queues"] = ["triage"]
        self.assertEqual(self.load().officer_queues, ["triage"])

    def test_WB_RULE_8_a_model_row_for_a_role_without_a_prompt_is_refused(self):
        self.models["models"]["officer"] = {"model": "claude-sonnet-5-5", "effort": "medium"}
        message = self.refused()
        self.assertIn("officer", message)
        self.assertIn("models.yml", message)

    def test_WB_RULE_8_a_role_with_a_prompt_and_no_model_row_is_refused(self):
        del self.models["models"]["fix"]
        message = self.refused()
        self.assertIn("fix", message)
        self.assertIn("models.yml", message)

    def test_WB_RULE_8_a_model_row_without_a_model_or_an_effort_is_refused_and_names_both(self):
        del self.models["models"]["build"]["model"]
        message = self.refused()
        self.assertIn("build", message)
        self.assertIn("model", message)
        self.models = good_models()
        del self.models["models"]["review"]["effort"]
        message = self.refused()
        self.assertIn("review", message)
        self.assertIn("effort", message)

    def test_WB_RULE_20_an_effort_that_is_not_known_is_refused(self):
        self.models["models"]["tester"]["effort"] = "enormous"
        message = self.refused()
        self.assertIn("tester", message)
        self.assertIn("effort", message)

    def test_WB_RULE_20_the_efforts_low_to_max_are_accepted(self):
        for effort in ("low", "medium", "high", "xhigh", "max"):
            self.models["models"]["tester"]["effort"] = effort
            self.assertEqual(self.load().models["tester"].effort, effort)

    def test_WB_RULE_22_a_role_in_may_not_also_hold_that_is_not_one_of_the_five_is_refused(self):
        self.rules["roles"]["build"]["may_not_also_hold"].append("captain")
        message = self.refused()
        self.assertIn("captain", message)
        self.assertIn("build", message)

    def test_WB_RULE_22_a_role_that_may_not_also_hold_itself_is_refused(self):
        self.rules["roles"]["build"]["may_not_also_hold"].append("build")
        self.assertIn("build", self.refused())


class TestRefusals(Tables):
    def test_WB_RULE_24_a_prompt_that_is_not_a_markdown_file_below_the_revision_folder_is_refused(self):
        for bad in ("roles/tester.txt", "roles/tester", "../tester.md", "roles/../../tester.md", "/etc/tester.md", ".."):
            with self.subTest(prompt=bad):
                self.rules["roles"]["tester"]["prompt"] = bad
                message = self.refused()
                self.assertIn("tester", message)
                self.assertIn("prompt", message)

    def test_WB_RULE_24_a_prompt_below_the_revision_folder_is_accepted(self):
        for good in ("roles/tester.md", "tester.md", "a/b/tester.md", "roles/a..b.md"):
            with self.subTest(prompt=good):
                self.rules["roles"]["tester"]["prompt"] = good
                self.assertEqual(self.load().roles["tester"].prompt, good)

    def test_WB_RULE_25_a_name_written_twice_in_the_list_for_a_person_is_refused(self):
        self.rules["always_to_a_person"].append("verification_of_classed_code")
        message = self.refused()
        self.assertIn("always_to_a_person", message)
        self.assertIn("twice", message)

    def test_WB_RULE_27_a_table_that_does_not_send_a_merge_to_a_person_is_refused(self):
        self.rules["always_to_a_person"].remove("merge")
        message = self.refused()
        self.assertIn("merge", message)
        self.assertIn("always_to_a_person", message)

    def test_WB_RULE_6_a_role_that_may_merges_in_any_form_is_refused(self):
        for entry in ("Merges the change.", "Merging the change.", "The change is merged.", "MERGE it"):
            with self.subTest(entry=entry):
                self.rules["roles"]["officer"]["may"] = [entry]
                self.assertIn("officer", self.refused())

    def test_WB_RULE_6_a_word_that_only_starts_like_merge_is_accepted(self):
        self.rules["roles"]["officer"]["may"] = ["Read the emergency list."]
        self.load()

    def test_WB_RULE_7_a_queue_that_is_not_a_text_is_refused(self):
        for bad in ([1], [["a"]], {"a": "b"}, "triage", None):
            with self.subTest(queues=bad):
                self.rules["officer_queues"] = bad
                self.assertIn("officer_queues", self.refused())

    def test_WB_RULE_28_a_role_named_twice_in_the_roles_it_may_not_also_hold_is_refused(self):
        for name in ROLES:
            with self.subTest(role=name):
                self.setUp()
                self.rules["roles"][name]["may_not_also_hold"].append(
                    self.rules["roles"][name]["may_not_also_hold"][0]
                )
                message = self.refused()
                self.assertIn(name, message)
                self.assertIn("twice", message)

    def test_WB_RULE_10_one_role_that_excludes_another_that_does_not_exclude_it_is_refused(self):
        self.rules["roles"]["fix"]["may_not_also_hold"].remove("build")
        message = self.refused()
        self.assertIn("build", message)
        self.assertIn("fix", message)

    def test_WB_RULE_10_the_other_way_is_refused_too(self):
        self.rules["roles"]["tester"]["may_not_also_hold"].remove("officer")
        message = self.refused()
        self.assertIn("tester", message)
        self.assertIn("officer", message)

    def test_WB_RULE_11_a_role_without_one_of_its_fields_is_refused_with_the_role_and_the_field(self):
        for field in ("step", "produces", "reads", "may", "may_not_also_hold", "prompt"):
            with self.subTest(field=field):
                self.rules = good_rules()
                del self.rules["roles"]["review"][field]
                message = self.refused()
                self.assertIn("review", message)
                self.assertIn(field, message)

    def test_WB_RULE_11_a_field_with_nothing_in_it_is_refused(self):
        self.rules["roles"]["fix"]["step"] = "  "
        message = self.refused()
        self.assertIn("fix", message)
        self.assertIn("step", message)
        self.rules = good_rules()
        self.rules["roles"]["fix"]["reads"] = []
        self.assertIn("reads", self.refused())

    def test_WB_RULE_12_an_unknown_key_at_the_top_is_refused_with_where_it_stands(self):
        self.rules["surprise"] = 1
        message = self.refused()
        self.assertIn("surprise", message)
        self.assertIn("rules.yml", message)

    def test_WB_RULE_12_an_unknown_key_in_a_role_is_refused_with_where_it_stands(self):
        self.rules["roles"]["build"]["colour"] = "blue"
        message = self.refused()
        self.assertIn("roles.build", message)
        self.assertIn("colour", message)
        self.assertIn("line", message)

    def test_WB_RULE_12_an_unknown_key_in_the_rule_for_classed_code_is_refused(self):
        self.rules["classed_code"]["extra"] = "x"
        message = self.refused()
        self.assertIn("classed_code", message)
        self.assertIn("extra", message)

    def test_WB_RULE_12_an_unknown_key_in_the_model_table_is_refused_at_each_depth(self):
        self.models["extra"] = 1
        message = self.refused()
        self.assertIn("extra", message)
        self.assertIn("models.yml", message)
        self.models = good_models()
        self.models["models"]["tester"]["temperature"] = 1
        message = self.refused()
        self.assertIn("models.tester", message)
        self.assertIn("temperature", message)

    def test_WB_RULE_13_a_number_of_rounds_that_is_not_a_whole_number_of_at_least_1_is_refused(self):
        for bad in (0, -1, 1.5, "2", True, None, [2]):
            with self.subTest(rounds=bad):
                self.rules["review_rounds"] = bad
                self.assertIn("review_rounds", self.refused())

    def test_WB_RULE_13_a_whole_number_of_at_least_1_is_accepted(self):
        for good in (1, 2, 5):
            self.rules["review_rounds"] = good
            self.assertEqual(self.load().review_rounds, good)

    def test_WB_RULE_14_a_model_id_that_is_not_exact_is_refused(self):
        for bad in ("latest", "claude-sonnet", "claude-sonnet-latest", "claude", "sonnet", "", "Claude-Sonnet-5-5", "claude sonnet 5"):
            with self.subTest(model=bad):
                self.models["models"]["tester"]["model"] = bad
                message = self.refused()
                self.assertIn("tester", message)
                self.assertIn("model", message)

    def test_WB_RULE_14_the_two_ids_of_the_first_table_are_accepted(self):
        self.models["models"]["tester"]["model"] = "claude-opus-5-5"
        self.assertEqual(self.load().models["tester"].model, "claude-opus-5-5")

    def test_WB_RULE_14_latest_in_the_middle_of_an_id_is_refused(self):
        for bad in ("claude-latest-5", "latest-5", "claude-sonnet-latest-5"):
            with self.subTest(model=bad):
                self.models["models"]["tester"]["model"] = bad
                self.assertIn("tester", self.refused())

    def test_WB_RULE_15_a_date_that_does_not_exist_is_refused_with_the_file_and_the_line(self):
        text = json.dumps(self.rules, indent=2).replace('"review_rounds": 2', "review_rounds: 2026-02-30")
        message = self.refused(rules_text=text + "\n")
        self.assertIn("rules.yml", message)
        self.assertIn("line", message)

    def test_WB_RULE_15_a_table_that_cannot_be_built_is_refused_in_the_model_table_too(self):
        message = self.refused(models_text="models: !!int abc\n")
        self.assertIn("models.yml", message)

    def test_WB_RULE_15_a_rule_table_that_is_not_yaml_is_refused_with_the_file_named(self):
        message = self.refused(rules_text="roles: [unclosed\n")
        self.assertIn("rules.yml", message)

    def test_WB_RULE_15_a_model_table_that_is_not_yaml_is_refused_with_the_file_named(self):
        message = self.refused(models_text="models: {a: [\n")
        self.assertIn("models.yml", message)

    def test_WB_RULE_15_a_table_that_is_not_a_mapping_is_refused_with_the_file_named(self):
        self.assertIn("rules.yml", self.refused(rules_text="- a\n- b\n"))
        self.assertIn("models.yml", self.refused(models_text="just text\n"))

    def test_WB_RULE_21_a_table_that_is_not_there_is_refused_with_the_file_named(self):
        rules_path, models_path = self.paths()
        os.remove(models_path)
        with self.assertRaises(yamlfile.YamlError) as caught:
            rules.load(rules_path, models_path)
        self.assertIn("models.yml", str(caught.exception))

    def test_WB_RULE_16_a_key_written_twice_in_a_role_is_refused_with_the_place(self):
        text = json.dumps(self.rules, indent=2).replace(
            '"step": "a step of build",', '"step": "a step of build",\n      "step": "again",'
        )
        message = self.refused(rules_text=text + "\n")
        self.assertIn("step", message)
        self.assertIn("rules.yml", message)
        self.assertIn("line", message)

    def test_WB_RULE_16_a_key_written_twice_at_the_top_of_the_model_table_is_refused(self):
        text = json.dumps(self.models, indent=2)
        text = text.replace('"models": {', '"models": {}, "models": {', 1)
        self.assertIn("models.yml", self.refused(models_text=text + "\n"))

    def test_WB_RULE_17_a_name_that_yaml_reads_as_something_else_is_refused_with_quotes_asked_for(self):
        text = json.dumps(self.rules, indent=2).replace('"step": "a step of tester"', '"step": no')
        message = self.refused(rules_text=text + "\n")
        self.assertIn("quotes", message)
        self.assertIn("roles.tester.step", message)

    def test_WB_RULE_17_a_prompt_that_yaml_reads_as_null_is_refused_with_quotes_asked_for(self):
        text = json.dumps(self.rules, indent=2).replace('"prompt": "none"', '"prompt": null')
        message = self.refused(rules_text=text + "\n")
        self.assertIn("quotes", message)
        self.assertIn("officer", message)

    def test_WB_RULE_17_a_model_that_yaml_reads_as_a_number_is_refused_with_quotes_asked_for(self):
        text = json.dumps(self.models, indent=2).replace('"model": "claude-opus-5-5"', '"model": 1.10')
        message = self.refused(models_text=text + "\n")
        self.assertIn("quotes", message)
        self.assertIn("models.review.model", message)

    def test_WB_RULE_17_an_effort_that_yaml_reads_as_a_boolean_is_refused_with_quotes_asked_for(self):
        text = json.dumps(self.models, indent=2).replace('"effort": "high"', '"effort": on')
        self.assertIn("quotes", self.refused(models_text=text + "\n"))

    def test_WB_RULE_17_a_list_of_names_with_a_number_in_it_is_refused(self):
        self.rules["roles"]["fix"]["reads"] = ["a", 1]
        self.assertIn("quotes", self.refused())

    def test_WB_RULE_18_a_tag_that_would_build_an_object_is_refused_in_a_table(self):
        text = json.dumps(self.rules, indent=2).replace('"step": "a step of fix"', '"step": !!python/object/apply:os.system ["echo hi"]')
        message = self.refused(rules_text=text + "\n")
        self.assertIn("rules.yml", message)


if __name__ == "__main__":
    unittest.main()
