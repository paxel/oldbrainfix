import unittest
import urllib.error

from sonar_gate import condition_line, find_analysis, gate_report, wait_for_gate

SHA = "be70a94000000000000000000000000000000000"
ANALYSIS = {"key": "AZ1", "date": "2026-11-08T19:00:00+0000", "revision": SHA}
OK = {"projectStatus": {"status": "OK", "conditions": []}}
FAILED = {
    "projectStatus": {
        "status": "ERROR",
        "conditions": [
            {"status": "ERROR", "metricKey": "new_coverage", "comparator": "LT", "errorThreshold": "80", "actualValue": "71.5"},
            {"status": "OK", "metricKey": "new_duplicated_lines_density", "comparator": "GT", "errorThreshold": "3", "actualValue": "0.0"},
        ],
    }
}


class Clock:
    """A clock that a sleep moves on."""

    def __init__(self):
        self.now = 0.0

    def __call__(self):
        return self.now

    def sleep(self, seconds):
        self.now += seconds


def wait(answers, status, wait_seconds):
    """Runs wait_for_gate with analyses() answering from answers in turn."""
    lines = []
    clock = Clock()
    answers = list(answers)

    def analyses():
        answer = answers.pop(0) if len(answers) > 1 else answers[0]
        if isinstance(answer, Exception):
            raise answer
        return answer

    passed = wait_for_gate(SHA, analyses, lambda key: status, wait_seconds, lines.append, clock.sleep, clock)
    return passed, lines, clock.now


def errors(lines):
    return [line for line in lines if line.startswith("::error::")]


class SonarGateTest(unittest.TestCase):
    def test_the_analysis_of_the_commit(self):
        other = dict(ANALYSIS, key="AZ0", revision="2222")
        self.assertEqual(find_analysis([other, ANALYSIS], SHA)["key"], "AZ1")
        self.assertIsNone(find_analysis([other], SHA))

    def test_a_passed_gate(self):
        passed, lines = gate_report(SHA, OK)
        self.assertTrue(passed)
        self.assertEqual(errors(lines), [])

    def test_a_failed_gate_names_its_failed_conditions_only(self):
        passed, lines = gate_report(SHA, FAILED)
        self.assertFalse(passed)
        self.assertEqual(len(errors(lines)), 2)
        self.assertIn("new_coverage is 71.5, at least 80 needed", lines[1])

    def test_a_condition_with_an_upper_limit(self):
        line = condition_line({"metricKey": "new_violations", "comparator": "GT", "errorThreshold": "0", "actualValue": "2"})
        self.assertEqual(line, "new_violations is 2, at most 0 needed")

    def test_an_analysis_there_at_once_needs_no_wait(self):
        passed, _, waited = wait([[ANALYSIS]], OK, 600)
        self.assertTrue(passed)
        self.assertEqual(waited, 0)

    def test_a_pending_analysis_is_waited_for(self):
        # sonar.yml's run on summer finishes after the tag was pushed.
        passed, lines, waited = wait([[], [], [ANALYSIS]], OK, 600)
        self.assertTrue(passed)
        self.assertEqual(waited, 120)
        self.assertIn("no analysis of", lines[0])

    def test_a_failed_gate_after_the_wait_fails(self):
        passed, _, _ = wait([[], [ANALYSIS]], FAILED, 600)
        self.assertFalse(passed)

    def test_no_analysis_within_the_wait_fails(self):
        passed, lines, waited = wait([[]], OK, 150)
        self.assertFalse(passed)
        self.assertEqual(waited, 150)
        self.assertIn("the quality gate of the tagged commit is unknown", errors(lines)[0])

    def test_no_wait_looks_once(self):
        passed, lines, waited = wait([[]], OK, 0)
        self.assertFalse(passed)
        self.assertEqual(waited, 0)
        self.assertEqual(len(lines), 1)

    def test_an_unreachable_service_is_asked_again(self):
        passed, _, waited = wait([OSError("timed out"), [ANALYSIS]], OK, 600)
        self.assertTrue(passed)
        self.assertEqual(waited, 60)

    def test_a_refused_token_fails_at_once(self):
        refused = urllib.error.HTTPError("https://sonarcloud.io", 401, "Unauthorized", {}, None)
        passed, lines, waited = wait([refused, [ANALYSIS]], OK, 600)
        self.assertFalse(passed)
        self.assertEqual(waited, 0)
        self.assertIn("could not ask SonarQube Cloud", errors(lines)[0])


if __name__ == "__main__":
    unittest.main()
