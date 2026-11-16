"""`wb init`, run as the command a person types, in repositories made for the test."""

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from io import StringIO
from unittest import mock

SRC = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "src")
sys.path.insert(0, SRC)

from watchbill import __version__ as REVISION  # noqa: E402
from watchbill import cli  # noqa: E402

# A hook of git hands its children GIT_DIR and the like. With them, the `git init` of a
# test would act on the repository the tests are run from, so they never reach a child.
ENV = {name: value for name, value in os.environ.items() if not name.startswith("GIT_")}
ENV["PYTHONPATH"] = SRC

HEAD = [
    "# How Watchbill is set up in this repository. Written by `wb init`.",
    "# `watchbill` is the version of the package the repository is set up for.",
]


def config(folder):
    return os.path.join(folder, ".watchbill", "watchbill.yml")


def read(path):
    with open(path, encoding="utf-8") as file:
        return file.read()


def put(folder, path, text=""):
    file = os.path.join(folder, path)
    os.makedirs(os.path.dirname(file), exist_ok=True)
    with open(file, "w", encoding="utf-8") as out:
        out.write(text)


def wb(folder, args, typed=""):
    """Runs `wb` in the folder; `typed` is what a person would type, one answer per line."""
    return subprocess.run(
        [sys.executable, "-m", "watchbill"] + args,
        cwd=folder,
        input=typed,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        universal_newlines=True,
        env=ENV,
    )


def init_text(value):
    from watchbill.init import _yaml_value

    return _yaml_value(value)


def lines(*answers):
    return "".join(answer + "\n" for answer in answers)


class InitTest(unittest.TestCase):
    def folder(self, name="brake-controller"):
        base = tempfile.mkdtemp(prefix="wb-test-")
        self.addCleanup(shutil.rmtree, base, True)
        folder = os.path.join(base, name)
        os.mkdir(folder)
        return folder

    def repository(self, origin=None, name="brake-controller"):
        """A repository with `origin` at the given address, when one is given."""
        folder = self.folder(name)
        subprocess.run(["git", "-C", folder, "init", "-q"], check=True, env=ENV)
        if origin:
            subprocess.run(["git", "-C", folder, "remote", "add", "origin", origin], check=True, env=ENV)
        return folder

    def test_WB_INIT_14_yes_writes_what_the_repository_says_and_asks_nothing(self):
        folder = self.repository("git@github.com:example/brake-controller.git")
        put(folder, "requirements/brake.md")
        put(folder, ".github/workflows/ci.yml")

        run = wb(folder, ["init", "--yes"])

        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertEqual(
            read(config(folder)),
            "\n".join(
                HEAD
                + [
                    "watchbill: " + REVISION,
                    "project: brake-controller",
                    "git_server: github",
                    "repository: example/brake-controller",
                    "tracker: github",
                    "requirements: requirements/*.md",
                    "ci: github-actions",
                    "",
                ]
            ),
        )
        self.assertNotIn("(brake-controller)", run.stdout, "no question is put")

    def test_WB_INIT_7_a_repository_with_nothing_to_find_proposes_none(self):
        folder = self.repository(name="My Project")

        run = wb(folder, ["init", "-y"])

        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertEqual(
            read(config(folder)),
            "\n".join(
                HEAD
                + [
                    "watchbill: " + REVISION,
                    "project: my-project",
                    "git_server: none",
                    "repository: none",
                    "tracker: none",
                    "requirements: none",
                    "ci: none",
                    "",
                ]
            ),
        )

    def test_WB_INIT_9_an_answer_replaces_what_was_found_and_an_empty_line_keeps_it(self):
        folder = self.repository("https://gitlab.example.com/brakes/controller.git")
        put(folder, ".gitlab-ci.yml")
        put(folder, "Jenkinsfile")

        # project, git server, repository, tracker, requirements, ci, then the question whether it is OK.
        run = wb(folder, ["init"], lines("brakes", "", "", "jira", "polarion", "jenkins", "yes"))

        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertEqual(
            read(config(folder)),
            "\n".join(
                HEAD
                + [
                    "watchbill: " + REVISION,
                    "project: brakes",
                    "git_server: gitlab",
                    "repository: brakes/controller",
                    "tracker: jira",
                    "requirements: polarion",
                    "ci: jenkins",
                    "",
                ]
            ),
        )
        self.assertIn("project: (brake-controller) ", run.stdout)
        self.assertIn("ci driver (github-actions, gitlab-ci, jenkins or none): (gitlab-ci) ", run.stdout)
        self.assertIn("Is this OK? (yes) ", run.stdout)

    def test_WB_INIT_10_with_no_git_server_the_repository_is_not_asked_for(self):
        folder = self.repository("git@github.com:example/brake-controller.git")

        run = wb(folder, ["init"], lines("", "none", "", "", "", ""))

        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertNotIn("repository at the git server", run.stdout)
        self.assertIn("\ngit_server: none\nrepository: none\n", read(config(folder)))

    def test_WB_INIT_11_input_that_ends_early_keeps_what_was_found_for_the_rest(self):
        folder = self.repository("git@github.com:example/brake-controller.git")

        run = wb(folder, ["init"], lines("brakes"))

        # The input ends at the question whether it is OK too, so nothing is written (WB-INIT-31).
        self.assertEqual(run.returncode, 1)
        self.assertIn("\nproject: brakes\ngit_server: github\n", run.stdout)

    def test_WB_INIT_12_a_project_name_that_cannot_be_used_is_asked_for_again(self):
        folder = self.repository()

        run = wb(folder, ["init"], lines("Brake Controller", "brakes", "", "", "", "", "yes"))

        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertIn("Not taken: a project name is", run.stdout)
        self.assertIn("\nproject: brakes\n", read(config(folder)))

    def test_WB_INIT_22_an_answer_yaml_would_read_as_something_else_is_written_in_quotes(self):
        folder = self.repository()

        # project, git server (none, so no repository), tracker, requirements, ci, OK.
        run = wb(folder, ["init"], lines("", "", "true", "docs/it's here/*.md #1", "", ""))

        self.assertEqual(run.returncode, 0, run.stderr)
        text = read(config(folder))
        self.assertIn("\ntracker: 'true'\n", text)
        self.assertIn("\nrequirements: 'docs/it''s here/*.md #1'\n", text)
        self.assertIn("\nwatchbill: " + REVISION + "\n", text)

    def test_WB_INIT_16_no_at_the_last_question_writes_nothing(self):
        folder = self.repository()

        run = wb(folder, ["init"], lines("", "", "", "", "", "no"))

        self.assertEqual(run.returncode, 1)
        self.assertIn("Nothing written.", run.stdout)
        self.assertFalse(os.path.exists(os.path.join(folder, ".watchbill")))

    def test_WB_INIT_19_a_file_that_is_already_there_is_left_as_it_is(self):
        folder = self.repository()
        put(folder, ".watchbill/watchbill.yml", "project: kept\n")

        run = wb(folder, ["init", "--yes"])

        self.assertEqual(run.returncode, 1)
        self.assertIn("is already there", run.stdout)
        self.assertEqual(read(config(folder)), "project: kept\n")

    def test_WB_INIT_17_from_a_folder_inside_the_repository_the_file_is_written_at_its_root(self):
        folder = self.repository()
        inner = os.path.join(folder, "src", "control")
        os.makedirs(inner)

        run = wb(inner, ["init", "--yes"])

        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertTrue(os.path.exists(config(folder)))
        self.assertFalse(os.path.exists(os.path.join(inner, ".watchbill")))

    def test_WB_INIT_20_outside_a_git_repository_nothing_is_written(self):
        folder = self.folder()

        run = wb(folder, ["init", "--yes"])

        self.assertEqual(run.returncode, 1)
        self.assertIn("not in a git repository", run.stderr)
        self.assertFalse(os.path.exists(os.path.join(folder, ".watchbill")))

    def test_WB_INIT_21_a_token_in_the_address_of_origin_never_reaches_the_file_or_the_screen(self):
        folder = self.repository("https://someone:s3cr3t-token@github.com/example/brake-controller.git")

        run = wb(folder, ["init", "--yes"])

        self.assertEqual(run.returncode, 0, run.stderr)
        everything = read(config(folder)) + run.stdout + run.stderr
        self.assertIn("\nrepository: example/brake-controller\n", everything)
        self.assertNotIn("s3cr3t", everything)
        self.assertNotIn("someone", everything)

    def test_WB_INIT_25_an_unknown_command_and_an_unknown_flag_end_with_2_and_the_usage(self):
        folder = self.repository()

        command = wb(folder, ["start"])
        self.assertEqual(command.returncode, 2)
        self.assertIn("no command 'start'", command.stderr)
        self.assertIn("usage:", command.stderr)

        flag = wb(folder, ["init", "--force"])
        self.assertEqual(flag.returncode, 2)
        self.assertIn("usage:", flag.stderr)
        self.assertFalse(os.path.exists(os.path.join(folder, ".watchbill")))

    def test_WB_INIT_26_version_prints_the_version_of_the_package(self):
        run = wb(self.folder(), ["--version"])

        self.assertEqual(run.returncode, 0)
        self.assertEqual(run.stdout, REVISION + "\n")


    def test_WB_INIT_13_an_answer_over_200_characters_or_with_a_control_character_is_asked_for_again(self):
        folder = self.repository()

        run = wb(folder, ["init"], lines("a" * 201, "bra\x07kes", "brakes", "", "t" * 201, "tra\x07cker", "", "", "", "yes"))

        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertIn("Not taken: that is over 200 characters.", run.stdout)
        self.assertIn("Not taken: that holds a character that cannot be written.", run.stdout)
        self.assertEqual(run.stdout.count("Not taken:"), 4, "the tracker question too")
        self.assertIn("\nproject: brakes\n", read(config(folder)))
        self.assertIn("\ntracker: none\n", read(config(folder)))

    def test_WB_INIT_15_the_file_is_shown_before_it_is_asked_whether_it_is_ok(self):
        folder = self.repository()

        run = wb(folder, ["init"], lines("", "", "", "", "", "no"))

        shown = run.stdout.index("About to write {}".format(config(folder)))
        self.assertLess(shown, run.stdout.index("project: brake-controller\n", shown))
        self.assertLess(shown, run.stdout.index("Is this OK?"))
        self.assertFalse(os.path.exists(config(folder)))

    def test_WB_INIT_18_the_version_of_wb_is_written_as_the_line_watchbill(self):
        folder = self.repository()

        run = wb(folder, ["init", "--yes"])

        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertIn("\nwatchbill: " + REVISION + "\n", read(config(folder)))

    def test_WB_INIT_23_the_file_that_was_written_is_named(self):
        folder = self.repository()

        run = wb(folder, ["init"], lines("", "", "", "", "", ""))

        self.assertIn("Wrote {}.".format(os.path.realpath(config(folder))), run.stdout)

        other = self.repository()
        yes = wb(other, ["init", "--yes"])
        self.assertIn("Wrote {}.".format(os.path.realpath(config(other))), yes.stdout)

    def test_WB_INIT_24_it_ends_with_0_when_the_file_is_written(self):
        folder = self.repository()

        run = wb(folder, ["init", "-y"])

        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertTrue(os.path.exists(config(folder)))

    def test_WB_INIT_25_no_command_ends_with_2_and_the_usage(self):
        run = wb(self.folder(), [])

        self.assertEqual(run.returncode, 2)
        self.assertIn("usage:", run.stdout)

    def test_WB_INIT_27_help_prints_the_usage_and_ends_with_0(self):
        run = wb(self.folder(), ["--help"])

        self.assertEqual(run.returncode, 0)
        self.assertIn("usage:", run.stdout)


    def test_WB_INIT_16_y_is_a_yes(self):
        folder = self.repository()

        run = wb(folder, ["init"], lines("", "", "", "", "", "y"))

        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertTrue(os.path.exists(config(folder)))

    def test_WB_INIT_28_an_empty_line_keeps_the_proposal(self):
        folder = self.repository("git@github.com:example/brake-controller.git")

        run = wb(folder, ["init"], lines("", "", "", "", "", "", "yes"))

        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertIn("\nproject: brake-controller\ngit_server: github\nrepository: example/brake-controller\n", read(config(folder)))

    def test_WB_INIT_29_the_repository_is_none_when_the_git_server_is_none(self):
        folder = self.repository("git@github.com:example/brake-controller.git")

        run = wb(folder, ["init"], lines("", "none", "", "", "", "yes"))

        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertIn("\nrepository: none\n", read(config(folder)))

    def test_WB_INIT_30_yes_writes_what_it_proposes(self):
        folder = self.repository("git@github.com:example/brake-controller.git")

        run = wb(folder, ["init", "--yes"])

        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertIn("\nrepository: example/brake-controller\n", read(config(folder)))

    def test_WB_INIT_31_input_that_ends_at_the_question_whether_it_is_ok_writes_nothing(self):
        folder = self.repository()

        run = wb(folder, ["init"], "")

        self.assertEqual(run.returncode, 1)
        self.assertIn("Nothing written.", run.stdout)
        self.assertFalse(os.path.exists(os.path.join(folder, ".watchbill")))

        late = wb(folder, ["init"], lines("", "", "", "", ""))
        self.assertEqual(late.returncode, 1)
        self.assertFalse(os.path.exists(os.path.join(folder, ".watchbill")))

    def interrupted(self, folder, where):
        """Runs the command in `folder` with an interrupt at `where`. Returns the status and what was printed."""
        before = os.getcwd()
        os.chdir(folder)
        self.addCleanup(os.chdir, before)
        out = StringIO()
        if where == "git":
            patch = mock.patch("watchbill.cli.subprocess.run", side_effect=KeyboardInterrupt)
        else:
            patch = mock.patch("builtins.input", side_effect=KeyboardInterrupt)
        with patch, redirect_stdout(out):
            status = cli.main(["init"])
        return status, out.getvalue()

    def test_WB_INIT_32_an_interrupt_ends_with_1(self):
        folder = self.repository()

        for where in ("git", "question"):
            status, _ = self.interrupted(folder, where)
            self.assertEqual(status, 1, where)
        self.assertFalse(os.path.exists(os.path.join(folder, ".watchbill")))

    def test_WB_INIT_33_an_interrupt_says_nothing_written_and_shows_no_traceback(self):
        folder = self.repository()

        for where in ("git", "question"):
            status, printed = self.interrupted(folder, where)
            self.assertIn("Nothing written.", printed, where)

    def test_WB_INIT_34_yes_shows_the_file_before_it_writes_it(self):
        folder = self.repository()

        run = wb(folder, ["init", "--yes"])

        shown = run.stdout.index("About to write {}".format(os.path.realpath(config(folder))))
        self.assertLess(shown, run.stdout.index("project: brake-controller\n", shown))
        self.assertLess(run.stdout.index("project: brake-controller\n", shown), run.stdout.index("Wrote "))

    def test_WB_INIT_35_help_and_h_do_as_dash_dash_help(self):
        for word in ("help", "-h"):
            run = wb(self.folder(), [word])
            self.assertEqual(run.returncode, 0, word)
            self.assertIn("usage:", run.stdout, word)

    def test_WB_INIT_36_dash_v_does_as_dash_dash_version(self):
        run = wb(self.folder(), ["-v"])

        self.assertEqual(run.returncode, 0)
        self.assertEqual(run.stdout, REVISION + "\n")

    def test_WB_INIT_37_any_usable_text_is_taken_for_the_git_server_tracker_and_ci_driver(self):
        folder = self.repository()

        run = wb(folder, ["init"], lines("", "forgejo", "", "linear", "reqs/*.md", "woodpecker", "yes"))

        self.assertEqual(run.returncode, 0, run.stderr)
        text = read(config(folder))
        for line in ("git_server: forgejo", "tracker: linear", "ci: woodpecker"):
            self.assertIn("\n" + line + "\n", text)

    def test_WB_INIT_38_a_value_is_written_plain_only_when_yaml_reads_it_back_as_the_same_text(self):
        for value in ("0x1f", "0b101", "0o17", "017", "1_000", "1e3", "1.5", "2024-01-01", "2024-01-01 10:00:00",
                      "true", "No", "off", "NULL", "y", "n", ".inf", "-1", "+1", "12:30", "1:20:30"):
            self.assertEqual(init_text(value), "'" + value + "'", value)
        for value in ("brakes", "requirements/*.md", "brake-controller", "a.b_c", "x0", "0.0.1", "1.2.3-rc1"):
            self.assertEqual(init_text(value), value, value)


if __name__ == "__main__":
    unittest.main()
