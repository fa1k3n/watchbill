"""What `wb init` reads from a repository before it asks anything."""

import os
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "src"))

from watchbill.detect import Remote, detect, git_server_of, parse_remote, project_name  # noqa: E402

# A hook of git hands its children GIT_DIR and the like; they never reach a child.
ENV = {name: value for name, value in os.environ.items() if not name.startswith("GIT_")}


class DetectTest(unittest.TestCase):
    def folder(self, files):
        """A folder that is no git repository, with the given files in it."""
        folder = tempfile.mkdtemp(prefix="wb-test-")
        self.addCleanup(shutil.rmtree, folder, True)
        for path in files:
            file = os.path.join(folder, path)
            os.makedirs(os.path.dirname(file), exist_ok=True)
            open(file, "w").close()
        return folder

    def test_WB_INIT_3_an_address_is_read_in_each_of_its_forms(self):
        self.assertEqual(
            parse_remote("git@github.com:example/brake.git"),
            Remote("github.com", "", "example/brake"),
        )
        self.assertEqual(
            parse_remote("https://GitLab.example.com/group/sub/brake.git/"),
            Remote("gitlab.example.com", "", "group/sub/brake"),
        )
        self.assertEqual(
            parse_remote("ssh://someone@review.example.com:29418/brakes/controller"),
            Remote("review.example.com", "29418", "brakes/controller"),
        )
        self.assertIsNone(parse_remote(""))
        self.assertIsNone(parse_remote("/srv/git/brake.git"))

    def test_WB_INIT_2_the_kind_of_git_server_is_taken_from_the_host_and_the_port(self):
        def kind(url):
            return git_server_of(parse_remote(url))

        self.assertEqual(kind("git@github.com:example/brake.git"), "github")
        self.assertEqual(kind("https://gitlab.example.com/group/brake.git"), "gitlab")
        self.assertEqual(kind("ssh://someone@review.example.com:29418/brakes/controller"), "gerrit")
        self.assertEqual(kind("https://bitbucket.org/example/brake.git"), "bitbucket")
        self.assertEqual(kind("https://dev.azure.com/example/brakes/_git/controller"), "azure-devops")
        self.assertEqual(kind("https://code.example.com/brake.git"), "other")
        self.assertEqual(kind(""), "none")

    def test_WB_INIT_1_a_project_name_is_made_from_the_name_of_the_folder(self):
        self.assertEqual(project_name("/work/brake-controller"), "brake-controller")
        self.assertEqual(project_name("/work/My Project (2)/"), "my-project-2")
        self.assertEqual(project_name("/work/___"), "project")

    def test_WB_INIT_5_the_requirements_are_the_first_folder_that_holds_a_file(self):
        """With the file ending most of its files have."""
        mixed = self.folder(["requirements/a.md", "requirements/b.md", "requirements/c.reqif"])
        self.assertEqual(detect(mixed).requirements, "requirements/*.md")
        self.assertEqual(detect(self.folder(["docs/requirements/a.reqif"])).requirements, "docs/requirements/*.reqif")
        self.assertEqual(detect(self.folder(["requirements/sub/a.md"])).requirements, "none")
        self.assertEqual(detect(self.folder([])).requirements, "none")

    def test_WB_INIT_6_the_ci_driver_is_known_by_the_file_it_leaves(self):
        self.assertEqual(detect(self.folder([".github/workflows/ci.yaml"])).ci, "github-actions")
        self.assertEqual(detect(self.folder([".github/dependabot.yml"])).ci, "none")
        self.assertEqual(detect(self.folder([".gitlab-ci.yml"])).ci, "gitlab-ci")
        self.assertEqual(detect(self.folder(["Jenkinsfile"])).ci, "jenkins")
        self.assertEqual(detect(self.folder(["azure-pipelines.yml"])).ci, "azure-pipelines")
        self.assertEqual(detect(self.folder(["zuul.d/jobs.yaml"])).ci, "zuul")
        self.assertEqual(detect(self.folder([])).ci, "none")


    def test_WB_INIT_4_the_tracker_is_the_git_server_when_that_is_github_or_gitlab(self):
        def tracker(url):
            folder = self.folder([])
            subprocess.run(["git", "-C", folder, "init", "-q"], check=True, env=ENV)
            subprocess.run(["git", "-C", folder, "remote", "add", "origin", url], check=True, env=ENV)
            return detect(folder, ENV).tracker

        self.assertEqual(tracker("git@github.com:example/brake.git"), "github")
        self.assertEqual(tracker("https://gitlab.example.com/group/brake.git"), "gitlab")
        self.assertEqual(tracker("https://bitbucket.org/example/brake.git"), "none")
        self.assertEqual(detect(self.folder([]), ENV).tracker, "none")


if __name__ == "__main__":
    unittest.main()
