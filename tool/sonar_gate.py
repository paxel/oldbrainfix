"""SonarQube Cloud's quality gate of the release's tagged commit (#234),
for release.yml's job "Sonar quality gate (tagged commit)".

    python3 tool/sonar_gate.py <tagged commit> [--wait-minutes N] [--warn-only]

Finds the main branch's analysis whose revision is the tagged commit
(sonar.yml analyses summer after each merge, and a tag sits on such a
merge) and asks for that analysis's quality gate. Exits 1, with GitHub
errors, when the gate failed, or when no analysis of the commit came
within N minutes (default 0: one look). While there is none yet the
analysis is pending, and it asks again every minute. --warn-only (a dry
run) reports the same as warnings and exits 0. Authenticates with
$SONAR_TOKEN.
"""

import json
import os
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

HOST = "https://sonarcloud.io"
PROJECT = "paxel_oldbrainfix"
SUMMARY = f"{HOST}/summary/new_code?id={PROJECT}"
INTERVAL = 60


def find_analysis(analyses, sha):
    """The analysis whose revision is the commit sha, or None."""
    for analysis in analyses:
        if analysis.get("revision") == sha:
            return analysis
    return None


def condition_line(condition):
    """One failed condition of the gate, readable."""
    need = "at least" if condition.get("comparator") == "LT" else "at most"
    return (
        f"{condition.get('metricKey')} is {condition.get('actualValue')}, "
        f"{need} {condition.get('errorThreshold')} needed"
    )


def gate_report(sha, status):
    """(passed, lines) for the answer of api/qualitygates/project_status."""
    project = status["projectStatus"]
    if project["status"] == "OK":
        return True, [f"Sonar: the quality gate of {sha} passed"]
    lines = [f"::error::Sonar: the quality gate of {sha} is {project['status']}, see {SUMMARY}"]
    for condition in project.get("conditions", []):
        if condition.get("status") == "ERROR":
            lines.append(f"::error::Sonar: {condition_line(condition)}")
    return False, lines


def fatal(error):
    """Whether asking again cannot help: the token is wrong or lacks rights."""
    return isinstance(error, urllib.error.HTTPError) and error.code in (401, 403)


def wait_for_gate(sha, analyses, status, wait_seconds, say, sleep=time.sleep, clock=time.monotonic):
    """Whether the quality gate of the commit sha passed. analyses() lists
    the main branch's analyses, status(key) is the gate of one; say prints
    a line. Asks again every minute until wait_seconds have passed."""
    deadline = clock() + wait_seconds
    while True:
        try:
            analysis = find_analysis(analyses(), sha)
            if analysis is not None:
                say(f"Sonar: analysis {analysis['key']} of {sha}, {analysis.get('date')}")
                passed, lines = gate_report(sha, status(analysis["key"]))
                for line in lines:
                    say(line)
                return passed
            reason = f"no analysis of {sha} yet"
        except (OSError, ValueError, KeyError) as error:
            if fatal(error):
                say(f"::error::Sonar: could not ask SonarQube Cloud: {error}")
                return False
            reason = f"could not ask SonarQube Cloud ({error})"
        left = deadline - clock()
        if left <= 0:
            say(f"::error::Sonar: {reason}; the quality gate of the tagged commit is unknown, see {SUMMARY}")
            return False
        say(f"Sonar: {reason}, asking again ({int(left // 60)} min left)")
        sleep(min(INTERVAL, left))


def sonar(path, params):
    """SonarQube Cloud's API answer for a path."""
    request = urllib.request.Request(f"{HOST}{path}?{urllib.parse.urlencode(params)}")
    token = os.environ.get("SONAR_TOKEN")
    if token:
        request.add_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def main(argv):
    args = argv[1:]
    warn_only = "--warn-only" in args
    args = [arg for arg in args if arg != "--warn-only"]
    wait_minutes = 0
    if len(args) == 3 and args[1] == "--wait-minutes":
        wait_minutes = int(args[2])
        args = args[:1]
    if len(args) != 1:
        print("usage: sonar_gate.py <tagged commit> [--wait-minutes N] [--warn-only]", file=sys.stderr)
        return 2

    def say(line):
        if warn_only:
            line = line.replace("::error::", "::warning::", 1)
        print(line, flush=True)

    passed = wait_for_gate(
        args[0],
        lambda: sonar("/api/project_analyses/search", {"project": PROJECT, "ps": 100})["analyses"],
        lambda key: sonar("/api/qualitygates/project_status", {"analysisId": key}),
        wait_minutes * 60,
        say,
    )
    return 0 if passed or warn_only else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
