"""The role prompts of the revision: their files, their parts and what they may name."""

import os
import re
import unittest

ROLES_FOLDER = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "src", "watchbill", "revision", "roles"
)
ROLES = ["tester", "build", "review", "fix"]
PLACEHOLDERS_FILE = os.path.join(ROLES_FOLDER, "placeholders.md")

# The four parts of a prompt, as the headings that start them, in order.
PARTS = ["## What you produce", "## What you read", "## What you never do", "## How you end"]

# Names of products and of words that belong to one product. The adapters carry those.
# A list cannot be complete: it holds the common ones, and a reader looks for the rest.
PRODUCTS = [
    "github",
    "gitlab",
    "gerrit",
    "bitbucket",
    "azure",
    "jira",
    "redmine",
    "bugzilla",
    "jenkins",
    "circleci",
    "zuul",
    "teamcity",
    "bamboo",
    "gitea",
    "forgejo",
    "codeberg",
    "sourcehut",
    "phabricator",
    "youtrack",
    "trello",
    "travis",
    "buildkite",
    "pull request",
    "merge request",
]

# Anything in braces counts, so that a misspelt or spaced placeholder is seen too.
PLACEHOLDER = re.compile(r"\{[^{}]*\}")


def raw(path):
    with open(path, "rb") as file:
        return file.read()


def prompt(role):
    return raw(os.path.join(ROLES_FOLDER, role + ".md")).decode("utf-8")


def part(role, heading):
    """The text of one part of a prompt: from its heading to the next."""
    return re.split(r"^## .*$", prompt(role), flags=re.MULTILINE)[1 + PARTS.index(heading)]


def listed():
    """The placeholders of the list: each stands first on a line of its own point, in backticks."""
    text = raw(PLACEHOLDERS_FILE).decode("utf-8")
    return re.findall(r"^- `(\{[a-z_]+\})`", text, re.MULTILINE)


class RolePromptsTest(unittest.TestCase):
    def test_WB_ROLE_1_each_of_the_four_roles_has_a_prompt_file(self):
        for role in ROLES:
            with self.subTest(role=role):
                self.assertTrue(os.path.isfile(os.path.join(ROLES_FOLDER, role + ".md")))

    def test_WB_ROLE_1_no_other_role_has_a_prompt_file(self):
        self.assertEqual(sorted(os.listdir(ROLES_FOLDER)), sorted(role + ".md" for role in ROLES + ["placeholders"]))

    def test_WB_ROLE_2_each_prompt_has_the_four_parts_in_order(self):
        for role in ROLES:
            with self.subTest(role=role):
                headings = [line for line in prompt(role).split("\n") if line.startswith("## ")]
                self.assertEqual(headings, PARTS)

    def test_WB_ROLE_2_no_part_of_a_prompt_is_empty(self):
        for role in ROLES:
            with self.subTest(role=role):
                parts = re.split(r"^## .*$", prompt(role), flags=re.MULTILINE)[1:]
                self.assertEqual(len(parts), len(PARTS))
                for heading, body in zip(PARTS, parts):
                    self.assertGreater(len(body.strip()), 80, heading)

    def test_WB_ROLE_2_what_you_read_also_says_what_is_not_read(self):
        for role in ROLES:
            with self.subTest(role=role):
                body = part(role, "## What you read")
                self.assertRegex(body, r"\nYou (do not read|read nothing)\b")

    def test_WB_ROLE_2_every_point_of_what_you_never_do_is_a_never(self):
        for role in ROLES:
            with self.subTest(role=role):
                points = [line for line in part(role, "## What you never do").split("\n") if line.startswith("- ")]
                self.assertGreaterEqual(len(points), 3)
                for point in points:
                    self.assertTrue(point.startswith("- You never "), point)

    def test_WB_ROLE_2_how_you_end_gives_the_fields_of_the_last_message_in_one_form(self):
        sentence = (
            "Your last message has these fields, every time, in this order, "
            "each on a line of its own that starts with its name:"
        )
        for role in ROLES:
            with self.subTest(role=role):
                body = part(role, "## How you end")
                self.assertIn(sentence, body)
                fields = re.findall(r"^- `([a-z_]+)`: ", body.split(sentence, 1)[1], re.MULTILINE)
                self.assertGreaterEqual(len(fields), 3)
                self.assertEqual(fields[-1], "needs_person")

    def test_WB_ROLE_14_the_last_message_of_every_role_starts_with_model_and_effort(self):
        for role in ROLES:
            with self.subTest(role=role):
                body = part(role, "## How you end")
                fields = re.findall(r"^- `([a-z_]+)`: ", body, re.MULTILINE)
                self.assertEqual(fields[:2], ["model", "effort"])
                self.assertIn("or `unknown`", body)

    def test_WB_ROLE_3_the_placeholders_are_one_list_in_a_file(self):
        names = listed()
        self.assertGreater(len(names), 0)
        self.assertEqual(len(names), len(set(names)), "a placeholder stands twice")
        # Every placeholder the file shows is a point of the list: none is only named in passing.
        self.assertEqual(sorted(set(PLACEHOLDER.findall(raw(PLACEHOLDERS_FILE).decode("utf-8")))), sorted(names))

    def test_WB_ROLE_4_a_prompt_uses_no_placeholder_that_is_not_on_the_list(self):
        names = set(listed())
        for role in ROLES:
            with self.subTest(role=role):
                used = set(PLACEHOLDER.findall(prompt(role)))
                self.assertEqual(used - names, set())
                self.assertNotEqual(used, set(), "a prompt that is filled in with nothing")

    def test_WB_ROLE_5_every_placeholder_on_the_list_is_used_by_a_prompt(self):
        used = set()
        for role in ROLES:
            used |= set(PLACEHOLDER.findall(prompt(role)))
        self.assertEqual(set(listed()) - used, set())

    def test_WB_ROLE_6_no_prompt_names_a_product_of_a_git_server_a_tracker_or_a_ci_driver(self):
        for role in ROLES:
            text = prompt(role).lower()
            for product in PRODUCTS:
                with self.subTest(role=role, product=product):
                    self.assertNotIn(product, text)

    def test_WB_ROLE_7_every_prompt_is_utf8_with_lf_and_ends_with_one_line_break(self):
        for name in ROLES + ["placeholders"]:
            with self.subTest(file=name):
                data = raw(os.path.join(ROLES_FOLDER, name + ".md"))
                data.decode("utf-8")
                self.assertNotIn(b"\r", data)
                self.assertFalse(data.startswith(b"\xef\xbb\xbf"), "a byte order mark")
                self.assertTrue(data.endswith(b"\n"))
                self.assertFalse(data.endswith(b"\n\n"))

    def test_WB_ROLE_8_each_prompt_starts_with_a_line_saying_it_is_a_draft(self):
        for role in ROLES:
            with self.subTest(role=role):
                first = prompt(role).split("\n", 1)[0]
                self.assertEqual(first, "Draft: this prompt has not yet run on a project.")


if __name__ == "__main__":
    unittest.main()
