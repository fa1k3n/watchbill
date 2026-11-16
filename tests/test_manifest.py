"""The manifest of the revision, the hash of it, `revisions.txt` and `wb revision`."""

import hashlib
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(os.path.dirname(HERE), "src")
sys.path.insert(0, SRC)

from watchbill import __version__ as VERSION  # noqa: E402
from watchbill import cli, manifest, rules  # noqa: E402

PACKAGE = os.path.join(SRC, "watchbill")
FIXTURES = os.path.join(HERE, "fixtures")
SMALL = os.path.join(FIXTURES, "small")

ENV = {name: value for name, value in os.environ.items() if not name.startswith("GIT_")}
ENV["PYTHONPATH"] = SRC
ENV["PYTHONDONTWRITEBYTECODE"] = "1"


def read_bytes(path):
    with open(path, "rb") as file:
        return file.read()


def wb(folder, args):
    return subprocess.run(
        [sys.executable, "-m", "watchbill"] + args,
        cwd=folder,
        env=ENV,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        universal_newlines=True,
    )


def snapshot(folder):
    """Every path below the folder, with the bytes of each file."""
    seen = {}
    for base, folders, names in os.walk(folder):
        for name in folders:
            seen[os.path.join(base, name)] = None
        for name in names:
            seen[os.path.join(base, name)] = read_bytes(os.path.join(base, name))
    return seen


class Scratch(unittest.TestCase):
    def setUp(self):
        self.folder = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.folder, True)

    def put(self, path, data):
        file = os.path.join(self.folder, path)
        os.makedirs(os.path.dirname(file), exist_ok=True)
        with open(file, "wb") as out:
            out.write(data)


class TestManifest(Scratch):
    def test_WB_REV_2_the_manifest_of_a_fixed_set_of_files_is_the_fixture_byte_for_byte(self):
        self.assertEqual(manifest.manifest(SMALL).encode("utf-8"), read_bytes(os.path.join(FIXTURES, "small.sha256")))

    def test_WB_REV_2_a_line_is_the_hash_two_spaces_and_the_path(self):
        for line in manifest.manifest(SMALL).splitlines():
            digest, path = line.split("  ", 1)
            self.assertEqual(len(digest), 64)
            self.assertEqual(digest, hashlib.sha256(read_bytes(os.path.join(SMALL, path))).hexdigest())

    def test_WB_REV_2_the_paths_are_sorted_by_path_with_capitals_first(self):
        paths = [line.split("  ", 1)[1] for line in manifest.manifest(SMALL).splitlines()]
        self.assertEqual(
            paths, ["revision/B.md", "revision/a.md", "revision/sub/c.yml", "revision/sub/d.md"]
        )

    def test_WB_REV_2_the_order_does_not_follow_the_order_the_file_system_gives(self):
        walk = os.walk

        def backwards(top, *args, **kwargs):
            for base, folders, names in walk(top, *args, **kwargs):
                folders.reverse()
                names.reverse()
                yield base, folders, names

        with mock.patch("os.walk", backwards):
            backward = manifest.manifest(SMALL)
        self.assertEqual(backward, manifest.manifest(SMALL))

    def test_WB_REV_2_the_paths_use_a_slash_and_start_with_revision(self):
        for line in manifest.manifest(SMALL).splitlines():
            path = line.split("  ", 1)[1]
            self.assertTrue(path.startswith("revision/"), path)
            self.assertNotIn("\\", path)

    def test_WB_REV_2_the_manifest_ends_with_one_line_break_and_has_no_other_empty_line(self):
        text = manifest.manifest(SMALL)
        self.assertTrue(text.endswith("\n"))
        self.assertNotIn("\n\n", text)

    def test_WB_REV_3_the_hash_of_the_revision_is_the_sha256_of_the_manifest(self):
        expected = hashlib.sha256(read_bytes(os.path.join(FIXTURES, "small.sha256"))).hexdigest()
        self.assertEqual(manifest.revision_hash(SMALL), expected)
        self.assertEqual(expected, "15eb321a06cfdecea8ed004c5e6697de33f28fb8e27097671c75262e7caa8b6f")

    def test_WB_REV_3_a_changed_byte_in_a_file_changes_the_hash(self):
        shutil.copytree(SMALL, os.path.join(self.folder, "copy"))
        before = manifest.revision_hash(os.path.join(self.folder, "copy"))
        self.put("copy/revision/a.md", b"two\n")
        self.assertNotEqual(manifest.revision_hash(os.path.join(self.folder, "copy")), before)


class TestForm(Scratch):
    def refused(self, path, data):
        self.put(os.path.join("revision", path), data)
        with self.assertRaises(manifest.ManifestError) as caught:
            manifest.manifest(self.folder)
        return str(caught.exception)

    def test_WB_REV_4_a_file_with_crlf_is_refused_and_named(self):
        self.put("revision/good.md", b"fine\n")
        self.assertIn("revision/roles/bad.md", self.refused("roles/bad.md", b"one\r\ntwo\r\n"))

    def test_WB_REV_4_a_file_that_is_not_utf8_is_refused_and_named(self):
        self.put("revision/good.md", b"fine\n")
        self.assertIn("revision/latin.md", self.refused("latin.md", "Grüße\n".encode("latin-1")))

    def test_WB_REV_4_a_file_with_a_lone_cr_is_refused_and_named(self):
        self.assertIn("revision/old.md", self.refused("old.md", b"one\rtwo\n"))

    def test_WB_REV_4_a_file_with_utf8_and_lf_is_taken(self):
        self.put("revision/ok.md", "Grüße\n".encode("utf-8"))
        self.assertIn("revision/ok.md", manifest.manifest(self.folder))

    def test_WB_REV_11_a_file_without_a_line_break_at_the_end_is_refused_and_named(self):
        self.assertIn("revision/open.md", self.refused("open.md", b"no end"))

    def test_WB_REV_11_an_empty_file_is_refused_and_named(self):
        self.assertIn("revision/empty.md", self.refused("empty.md", b""))


class TestWhatIsRefused(Scratch):
    def refused(self):
        with self.assertRaises(manifest.ManifestError) as caught:
            manifest.manifest(self.folder)
        return str(caught.exception)

    def test_WB_REV_14_a_revision_folder_that_holds_no_file_is_refused_by_name(self):
        os.makedirs(os.path.join(self.folder, "revision", "roles"))
        self.assertIn("revision/", self.refused())

    def test_WB_REV_14_no_revision_folder_is_refused_by_name(self):
        self.assertIn("revision/", self.refused())

    def test_WB_REV_14_the_hash_of_nothing_is_not_given(self):
        with self.assertRaises(manifest.ManifestError):
            manifest.revision_hash(self.folder)

    def test_WB_REV_14_wb_revision_with_no_file_ends_with_1_and_prints_no_revision(self):
        out, err = mock.MagicMock(), mock.MagicMock()
        with mock.patch("watchbill.manifest.PACKAGE_FOLDER", self.folder), \
                mock.patch("sys.stderr", err), mock.patch("sys.stdout", out):
            status = cli.main(["revision"])
        self.assertEqual(status, 1)
        self.assertIn("revision/", "".join(call.args[0] for call in err.write.call_args_list))
        out.write.assert_not_called()

    def test_WB_REV_16_a_file_that_cannot_be_read_is_refused_by_name(self):
        self.put("revision/a.md", b"one\n")
        with mock.patch("watchbill.manifest.open", side_effect=PermissionError(13, "denied"), create=True):
            self.assertIn("revision/a.md", self.refused())

    def test_WB_REV_16_a_file_name_that_is_not_utf8_is_refused_and_shown(self):
        self.put("revision/ok.md", b"one\n")
        try:
            with open(os.path.join(os.fsencode(self.folder), b"revision", b"bad\xff.md"), "wb") as out:
                out.write(b"x\n")
        except OSError:
            self.skipTest("this file system takes no such name")
        message = self.refused()
        self.assertIn("revision/bad", message)
        self.assertIn("UTF-8", message)

    def test_WB_REV_17_a_file_name_with_a_line_break_is_refused_and_shown(self):
        self.put("revision/a\nb.md", b"one\n")
        self.assertIn("revision/a", self.refused())

    def test_WB_REV_17_a_file_name_with_a_backslash_is_refused_and_shown(self):
        self.put("revision/a\\b.md", b"one\n")
        self.assertIn("revision/a", self.refused())

    def test_WB_REV_17_a_name_with_a_space_or_non_ascii_letters_is_taken(self):
        self.put("revision/a b.md", b"one\n")
        self.put("revision/gr\u00fc.md", b"one\n")
        self.assertIn("revision/a b.md", manifest.manifest(self.folder))

    def test_WB_REV_18_a_symbolic_link_to_a_file_is_refused_by_name(self):
        self.put("outside.md", b"one\n")
        self.put("revision/ok.md", b"one\n")
        os.symlink(os.path.join(self.folder, "outside.md"), os.path.join(self.folder, "revision", "link.md"))
        self.assertIn("revision/link.md", self.refused())

    def test_WB_REV_18_a_symbolic_link_to_a_folder_is_refused_by_name(self):
        self.put("revision/ok.md", b"one\n")
        os.makedirs(os.path.join(self.folder, "elsewhere"))
        os.symlink(os.path.join(self.folder, "elsewhere"), os.path.join(self.folder, "revision", "linked"))
        self.assertIn("revision/linked", self.refused())

    def test_WB_REV_18_a_broken_symbolic_link_is_refused_by_name(self):
        self.put("revision/ok.md", b"one\n")
        os.symlink(os.path.join(self.folder, "nothing"), os.path.join(self.folder, "revision", "gone.md"))
        self.assertIn("revision/gone.md", self.refused())

    def test_WB_REV_18_a_hidden_file_is_refused_by_name(self):
        self.put("revision/ok.md", b"one\n")
        self.put("revision/.a.md.swp", b"one\n")
        self.assertIn("revision/.a.md.swp", self.refused())

    def test_WB_REV_18_a_file_in_a_hidden_folder_is_refused_by_name(self):
        self.put("revision/ok.md", b"one\n")
        self.put("revision/.git/x.md", b"one\n")
        self.assertIn("revision/.git", self.refused())

    def test_WB_REV_18_a_backup_file_that_ends_with_a_tilde_is_refused_by_name(self):
        self.put("revision/ok.md", b"one\n")
        self.put("revision/a.md~", b"one\n")
        self.assertIn("revision/a.md~", self.refused())


class TestPackageData(unittest.TestCase):
    def patterns(self):
        import configparser

        config = configparser.ConfigParser()
        config.read(os.path.join(os.path.dirname(HERE), "setup.cfg"), encoding="utf-8")
        return [line.strip() for line in config["options.package_data"]["watchbill"].splitlines() if line.strip()]

    def uncovered(self, paths, patterns):
        import fnmatch

        # A pattern of setup.cfg takes one folder level for each `*`-free part: no `**`.
        return [
            path for path in paths
            if not any(fnmatch.fnmatchcase(path, pattern) and path.count("/") == pattern.count("/") for pattern in patterns)
        ]

    def test_WB_REV_15_every_path_of_the_committed_manifest_matches_a_pattern_of_package_data(self):
        text = read_bytes(os.path.join(PACKAGE, "revision.sha256")).decode("utf-8")
        paths = [line.split("  ", 1)[1] for line in text.splitlines()]
        self.assertEqual(self.uncovered(paths, self.patterns()), [])

    def test_WB_REV_15_the_manifest_and_revisions_txt_are_package_data(self):
        self.assertEqual(self.uncovered(["revision.sha256", "revisions.txt"], self.patterns()), [])

    def test_WB_REV_15_a_file_in_a_new_folder_or_of_a_new_type_is_found(self):
        paths = ["revision/lenses/one.md", "revision/roles/x.txt", "revision/roles/a.md"]
        self.assertEqual(self.uncovered(paths, self.patterns()), ["revision/lenses/one.md", "revision/roles/x.txt"])


class TestCommitted(unittest.TestCase):
    def test_WB_REV_5_the_committed_manifest_is_the_one_computed_from_the_files(self):
        self.assertEqual(
            read_bytes(os.path.join(PACKAGE, "revision.sha256")).decode("utf-8"),
            manifest.manifest(PACKAGE),
        )

    def test_WB_REV_5_a_changed_prompt_without_a_new_manifest_fails(self):
        with tempfile.TemporaryDirectory() as folder:
            copy = os.path.join(folder, "watchbill")
            shutil.copytree(PACKAGE, copy, ignore=shutil.ignore_patterns("__pycache__"))
            with open(os.path.join(copy, "revision", "roles", "build.md"), "ab") as out:
                out.write(b"One more line.\n")
            self.assertNotEqual(read_bytes(os.path.join(copy, "revision.sha256")).decode("utf-8"), manifest.manifest(copy))

    @unittest.skipUnless(shutil.which("sha256sum"), "sha256sum is not here")
    def test_WB_REV_6_sha256sum_passes_on_the_manifest_in_the_folder_that_holds_revision(self):
        done = subprocess.run(
            ["sha256sum", "-c", "revision.sha256"],
            cwd=PACKAGE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            universal_newlines=True,
        )
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)

    @unittest.skipUnless(shutil.which("sha256sum"), "sha256sum is not here")
    def test_WB_REV_6_sha256sum_fails_when_a_file_is_changed(self):
        with tempfile.TemporaryDirectory() as folder:
            copy = os.path.join(folder, "watchbill")
            shutil.copytree(PACKAGE, copy, ignore=shutil.ignore_patterns("__pycache__"))
            with open(os.path.join(copy, "revision", "roles", "fix.md"), "ab") as out:
                out.write(b"x\n")
            done = subprocess.run(
                ["sha256sum", "-c", "revision.sha256"], cwd=copy, stdout=subprocess.PIPE, stderr=subprocess.PIPE
            )
            self.assertNotEqual(done.returncode, 0)

    def test_WB_REV_6_every_file_under_revision_is_in_the_committed_manifest(self):
        listed = [line.split("  ", 1)[1] for line in read_bytes(os.path.join(PACKAGE, "revision.sha256")).decode().splitlines()]
        found = []
        for base, _, names in os.walk(os.path.join(PACKAGE, "revision")):
            for name in names:
                found.append(os.path.relpath(os.path.join(base, name), PACKAGE).replace(os.sep, "/"))
        self.assertEqual(sorted(listed), sorted(found))
        self.assertGreaterEqual(len(listed), 7)


def prompt_problems(folder, named):
    """What is wrong with the prompt files in a folder of roles: a file no role names, a name without a file."""
    found = sorted(name for name in os.listdir(folder) if name != "placeholders.md")
    return [("no role names it", name) for name in found if name not in named] + [
        ("no file", name) for name in named if name not in found
    ]


class TestPrompts(unittest.TestCase):
    def folder_files(self):
        folder = os.path.join(PACKAGE, "revision", "roles")
        return sorted(os.listdir(folder))

    def named(self):
        return sorted(
            os.path.basename(role.prompt) for role in rules.load().roles.values() if role.prompt != "none"
        )

    def test_WB_REV_7_every_prompt_file_a_role_names_exists(self):
        for role in rules.load().roles.values():
            if role.prompt != "none":
                path = os.path.join(PACKAGE, "revision", *role.prompt.split("/"))
                self.assertTrue(os.path.isfile(path), role.prompt)

    def test_WB_REV_7_every_prompt_file_is_named_by_a_role(self):
        # placeholders.md is the list of words a prompt is filled in with (WB-ROLE-3). It is no prompt.
        self.assertEqual(prompt_problems(os.path.join(PACKAGE, "revision", "roles"), self.named()), [])

    def test_WB_REV_7_a_prompt_file_that_no_role_names_is_found_by_the_same_check(self):
        with tempfile.TemporaryDirectory() as folder:
            for name in self.named() + ["placeholders.md", "stray.md"]:
                with open(os.path.join(folder, name), "wb") as out:
                    out.write(b"x\n")
            self.assertEqual(prompt_problems(folder, self.named()), [("no role names it", "stray.md")])

    def test_WB_REV_7_a_role_whose_file_is_missing_is_found_by_the_same_check(self):
        with tempfile.TemporaryDirectory() as folder:
            self.assertEqual(
                prompt_problems(folder, ["tester.md"]), [("no file", "tester.md")]
            )

    def test_WB_REV_7_the_file_of_the_placeholders_is_there_and_no_role_names_it(self):
        self.assertIn("placeholders.md", self.folder_files())
        self.assertNotIn("placeholders.md", self.named())


def wrong_hashes(entries, version, digest):
    """The entries that name the version with another hash than the computed one."""
    return [(v, d) for v, d in entries if v == version and d != digest]


class TestReleased(Scratch):
    def test_WB_REV_8_the_current_version_does_not_stand_with_another_hash(self):
        self.assertEqual(wrong_hashes(manifest.released(), VERSION, manifest.revision_hash(PACKAGE)), [])

    def test_WB_REV_8_the_same_check_finds_the_version_with_another_hash(self):
        self.put("revisions.txt", "{}  {}\n".format(VERSION, "0" * 64).encode())
        entries = manifest.released(os.path.join(self.folder, "revisions.txt"))
        self.assertEqual(wrong_hashes(entries, VERSION, manifest.revision_hash(PACKAGE)), [(VERSION, "0" * 64)])

    def test_WB_REV_8_the_same_check_lets_the_right_hash_and_other_versions_pass(self):
        digest = manifest.revision_hash(PACKAGE)
        entries = [(VERSION, digest), ("0.0.0", "0" * 64)]
        self.assertEqual(wrong_hashes(entries, VERSION, digest), [])

    def test_WB_REV_8_a_line_that_is_not_a_version_and_a_hash_is_refused_and_names_the_line(self):
        self.put("revisions.txt", b"0.0.1  abc\n")
        with self.assertRaises(manifest.ManifestError) as caught:
            manifest.released(os.path.join(self.folder, "revisions.txt"))
        self.assertIn("revisions.txt", str(caught.exception))
        self.assertIn("line 1", str(caught.exception))

    def test_WB_REV_8_a_version_twice_is_refused(self):
        line = "0.0.1  {}\n".format("a" * 64)
        self.put("revisions.txt", (line + line).encode())
        with self.assertRaises(manifest.ManifestError) as caught:
            manifest.released(os.path.join(self.folder, "revisions.txt"))
        self.assertIn("0.0.1", str(caught.exception))

    def test_WB_REV_19_revisions_txt_with_crlf_is_refused_and_named(self):
        self.put("revisions.txt", "0.0.1  {}\r\n".format("a" * 64).encode())
        with self.assertRaises(manifest.ManifestError) as caught:
            manifest.released(os.path.join(self.folder, "revisions.txt"))
        self.assertIn("revisions.txt", str(caught.exception))

    def test_WB_REV_19_revisions_txt_without_a_line_break_at_the_end_is_refused_and_named(self):
        self.put("revisions.txt", "0.0.1  {}".format("a" * 64).encode())
        with self.assertRaises(manifest.ManifestError) as caught:
            manifest.released(os.path.join(self.folder, "revisions.txt"))
        self.assertIn("revisions.txt", str(caught.exception))

    def test_WB_REV_19_revisions_txt_that_is_not_utf8_is_refused_and_named(self):
        self.put("revisions.txt", b"\xff\xfe\n")
        with self.assertRaises(manifest.ManifestError) as caught:
            manifest.released(os.path.join(self.folder, "revisions.txt"))
        self.assertIn("revisions.txt", str(caught.exception))

    def test_WB_REV_19_a_good_revisions_txt_with_two_versions_is_read(self):
        self.put("revisions.txt", "0.0.1  {}\n0.0.2  {}\n".format("a" * 64, "b" * 64).encode())
        self.assertEqual(
            manifest.released(os.path.join(self.folder, "revisions.txt")),
            [("0.0.1", "a" * 64), ("0.0.2", "b" * 64)],
        )

    def test_WB_REV_8_the_file_is_in_the_package_and_holds_the_form(self):
        text = read_bytes(os.path.join(PACKAGE, "revisions.txt")).decode("utf-8")
        self.assertTrue(text == "" or text.endswith("\n"))
        manifest.released()


class TestCommand(Scratch):
    def test_WB_REV_9_wb_revision_prints_the_version_the_hash_and_the_manifest_and_ends_with_0(self):
        done = wb(self.folder, ["revision"])
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertIn(VERSION, done.stdout.splitlines()[0])
        self.assertIn(manifest.revision_hash(PACKAGE), done.stdout.splitlines()[1])
        self.assertIn(manifest.manifest(PACKAGE), done.stdout)
        self.assertEqual(done.stderr, "")

    def test_WB_REV_9_the_hash_it_prints_is_the_sha256_of_the_manifest_it_prints(self):
        done = wb(self.folder, ["revision"])
        lines = done.stdout.splitlines()
        digest = lines[1].split()[-1]
        printed = "\n".join(lines[3:]) + "\n"
        self.assertEqual(lines[2], "")
        self.assertEqual(hashlib.sha256(printed.encode("utf-8")).hexdigest(), digest)

    def test_WB_REV_9_the_hash_it_prints_is_the_same_in_another_folder(self):
        other = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, other, True)
        self.assertEqual(wb(self.folder, ["revision"]).stdout, wb(other, ["revision"]).stdout)

    def test_WB_REV_10_wb_revision_writes_nothing(self):
        before = snapshot(self.folder), snapshot(SRC), snapshot(os.path.dirname(HERE) + "/requirements")
        done = wb(self.folder, ["revision"])
        self.assertEqual(done.returncode, 0, done.stderr)
        after = snapshot(self.folder), snapshot(SRC), snapshot(os.path.dirname(HERE) + "/requirements")
        self.assertEqual(before, after)

    def test_WB_REV_12_wb_revision_with_an_argument_ends_with_2_and_prints_no_revision(self):
        done = wb(self.folder, ["revision", "--now"])
        self.assertEqual(done.returncode, 2)
        self.assertEqual(done.stdout, "")
        self.assertIn("--now", done.stderr)

    def test_WB_REV_13_wb_revision_with_a_file_that_is_refused_ends_with_1_and_names_it(self):
        self.put("revision/bad.md", b"one\r\n")
        out, err = mock.MagicMock(), mock.MagicMock()
        err.write = mock.MagicMock()
        with mock.patch("watchbill.manifest.PACKAGE_FOLDER", self.folder), \
                mock.patch("sys.stderr", err), mock.patch("sys.stdout", out):
            status = cli.main(["revision"])
        self.assertEqual(status, 1)
        self.assertIn("revision/bad.md", "".join(call.args[0] for call in err.write.call_args_list))
        out.write.assert_not_called()


if __name__ == "__main__":
    unittest.main()
