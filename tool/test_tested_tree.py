import unittest

from tested_tree import latest_runs, merged_pull, required_checks, verdict

TAG = "6620ba8000000000000000000000000000000000"
HEAD = "1111111111111111111111111111111111111111"
TREE = "aaaa"
REQUIRED = {"Analyze & test", "Desktop (Rust)"}

PULL = {
    "number": 289,
    "merged_at": "2026-11-07T10:00:00Z",
    "merge_commit_sha": TAG,
    "head": {"sha": HEAD, "ref": "bugfix/0.5"},
    "base": {"ref": "summer"},
}


def run(name, conclusion="success", suite=1, run_id=1, status="completed"):
    return {"name": name, "status": status, "conclusion": conclusion, "check_suite": {"id": suite}, "id": run_id}


def green():
    return [run("Analyze & test", suite=1, run_id=10), run("Desktop (Rust)", suite=1, run_id=11)]


class MergedPullTest(unittest.TestCase):
    def test_the_pull_request_merged_into_the_commit(self):
        other = dict(PULL, number=1, merge_commit_sha="2222")
        self.assertEqual(merged_pull([other, PULL], TAG)["number"], 289)

    def test_an_open_or_closed_pull_request_is_none(self):
        # GitHub lists the pull requests a commit is in, merged or not.
        self.assertIsNone(merged_pull([dict(PULL, merged_at=None)], TAG))
        self.assertIsNone(merged_pull([], TAG))

    def test_a_pull_request_merged_elsewhere_is_none(self):
        # The tag on a commit of the head branch, not on the merge.
        self.assertIsNone(merged_pull([dict(PULL, merge_commit_sha="2222")], TAG))


class RequiredChecksTest(unittest.TestCase):
    def test_the_names_the_rules_require(self):
        rules = [
            {"type": "deletion", "parameters": None},
            {
                "type": "required_status_checks",
                "parameters": {
                    "required_status_checks": [
                        {"context": "Analyze & test", "integration_id": 15368},
                        {"context": "iOS build and XCTest", "integration_id": 15368},
                    ]
                },
            },
        ]
        self.assertEqual(required_checks(rules), {"Analyze & test", "iOS build and XCTest"})
        self.assertEqual(required_checks([]), set())


class LatestRunsTest(unittest.TestCase):
    def test_a_rerun_counts_with_its_last_attempt(self):
        runs = latest_runs([run("Debug APK", "failure", run_id=1), run("Debug APK", "success", run_id=2)])
        self.assertEqual([r["conclusion"] for r in runs], ["success"])

    def test_a_skipped_run_in_another_workflow_run_hides_no_failure(self):
        runs = latest_runs([run("Debug APK", "failure", suite=1, run_id=1), run("Debug APK", "skipped", suite=2, run_id=2)])
        self.assertEqual(sorted(r["conclusion"] for r in runs), ["failure", "skipped"])


class VerdictTest(unittest.TestCase):
    def test_the_same_tree_with_every_check_green_is_tested(self):
        tested, lines = verdict(TAG, PULL, TREE, TREE, REQUIRED, green() + [run("build", "skipped", suite=3, run_id=12)])
        self.assertTrue(tested)
        self.assertIn("run no tests again", lines[-1])

    def test_no_merged_pull_request_runs_the_tests(self):
        # A tag made by the Actions button on a commit that never was a PR.
        tested, lines = verdict(TAG, None, None, None, set(), [])
        self.assertFalse(tested)
        self.assertIn("No pull request was merged", lines[0])

    def test_another_tree_runs_the_tests(self):
        tested, lines = verdict(TAG, PULL, TREE, "bbbb", REQUIRED, green())
        self.assertFalse(tested)
        self.assertIn("is not the tagged tree", lines[-1])

    def test_a_failed_check_runs_the_tests(self):
        tested, lines = verdict(TAG, PULL, TREE, TREE, REQUIRED, green() + [run("build", "failure", run_id=12)])
        self.assertFalse(tested)
        self.assertIn("Check 'build' on the head: completed, failure", lines)

    def test_a_check_still_running_runs_the_tests(self):
        runs = green() + [run("SonarQube Cloud analysis", None, run_id=12, status="in_progress")]
        tested, _ = verdict(TAG, PULL, TREE, TREE, REQUIRED, runs)
        self.assertFalse(tested)

    def test_a_cancelled_check_runs_the_tests(self):
        tested, _ = verdict(TAG, PULL, TREE, TREE, REQUIRED, green() + [run("iOS build and XCTest", "cancelled", run_id=12)])
        self.assertFalse(tested)

    def test_a_required_check_without_a_run_runs_the_tests(self):
        tested, lines = verdict(TAG, PULL, TREE, TREE, REQUIRED | {"iOS build and XCTest"}, green())
        self.assertFalse(tested)
        self.assertIn("Required check 'iOS build and XCTest' has no successful run on the head", lines)

    def test_no_check_runs_at_all_run_the_tests(self):
        tested, _ = verdict(TAG, PULL, TREE, TREE, REQUIRED, [])
        self.assertFalse(tested)

    def test_a_required_check_that_was_only_skipped_runs_the_tests(self):
        # The pull_request run's skipped job is no proof of the push run.
        runs = [run("Analyze & test", "skipped", suite=2, run_id=20), run("Desktop (Rust)", suite=1, run_id=11)]
        tested, _ = verdict(TAG, PULL, TREE, TREE, REQUIRED, runs)
        self.assertFalse(tested)

    def test_a_failed_push_run_beside_a_skipped_pull_request_run_runs_the_tests(self):
        runs = green() + [run("Analyze & test", "skipped", suite=2, run_id=20), run("Analyze & test", "failure", suite=3, run_id=5)]
        tested, _ = verdict(TAG, PULL, TREE, TREE, REQUIRED, runs)
        self.assertFalse(tested)

    def test_a_branch_without_required_checks_runs_the_tests(self):
        tested, lines = verdict(TAG, PULL, TREE, TREE, set(), green())
        self.assertFalse(tested)
        self.assertIn("summer requires no check", lines[-1])


if __name__ == "__main__":
    unittest.main()
