"""Whether the release's tagged commit was tested before its merge (#234),
for release.yml's job "Tested before the merge".

    python3 tool/tested_tree.py <tagged commit>

Looks up the pull request merged into the tagged commit (its merge commit
is that commit) and that pull request's head commit. The tree counts as
tested when the head commit's tree is the tagged commit's tree, every
check the base branch's rules require has a successful run on the head
commit, and every check run there ended in success, neutral or skipped.
Then the release's test gates run no tests again: the same tree passed
them before the merge.

Anything else, an error asking GitHub included, says "not tested", and
the full test gate runs as before. Never fails: writes tested=true or
tested=false, and the pull request's head branch as head_ref when it is a
branch of this repository, to $GITHUB_OUTPUT. Asks GitHub's API for
$GITHUB_REPOSITORY with $GITHUB_TOKEN or $GH_TOKEN.
"""

import json
import os
import sys
import urllib.parse
import urllib.request

API = "https://api.github.com"
# A run that ended so is no failure: skipped is a job whose `if:` was
# false, such as CI's pull_request run of a branch of this repository.
PASSED = {"success", "neutral", "skipped"}


def merged_pull(pulls, sha):
    """The pull request whose merge made the commit sha, or None."""
    for pull in pulls:
        if pull.get("merged_at") and pull.get("merge_commit_sha") == sha:
            return pull
    return None


def required_checks(rules):
    """The check names the rules of a branch require."""
    names = set()
    for rule in rules:
        if rule.get("type") == "required_status_checks":
            for check in rule["parameters"]["required_status_checks"]:
                names.add(check["context"])
    return names


def latest_runs(runs):
    """The last run of each check per check suite (one workflow run): a
    job that was re-run counts with its last attempt only, but a skipped
    job of the same name in another workflow run hides no failure."""
    latest = {}
    for run in runs:
        key = (run["check_suite"]["id"], run["name"])
        if key not in latest or run["id"] > latest[key]["id"]:
            latest[key] = run
    return sorted(latest.values(), key=lambda run: run["name"])


def verdict(sha, pull, tag_tree, head_tree, required, runs):
    """(tested, lines): whether the tree of the tagged commit sha passed
    every check before the merge, and the lines that say why."""
    if pull is None:
        return False, [f"No pull request was merged into {sha}: the full test gate runs"]
    head = pull["head"]["sha"]
    lines = [f"{sha} is the merge of pull request #{pull['number']}, head {head}"]
    if tag_tree != head_tree:
        lines.append(f"The head's tree {head_tree} is not the tagged tree {tag_tree}: the full test gate runs")
        return False, lines
    if not required:
        lines.append(f"{pull['base']['ref']} requires no check: the full test gate runs")
        return False, lines
    runs = latest_runs(runs)
    failed = [run for run in runs if run["status"] != "completed" or run["conclusion"] not in PASSED]
    passed = {run["name"] for run in runs if run["status"] == "completed" and run["conclusion"] == "success"}
    missing = sorted(required - passed)
    for run in failed:
        lines.append(f"Check {run['name']!r} on the head: {run['status']}, {run['conclusion']}")
    for name in missing:
        lines.append(f"Required check {name!r} has no successful run on the head")
    if failed or missing:
        lines.append("The full test gate runs")
        return False, lines
    lines.append(
        f"The tagged tree passed all {len(runs)} checks on the head, the {len(required)} "
        "required ones included: the test gates run no tests again"
    )
    return True, lines


def github(path):
    """GitHub's API answer for a path."""
    request = urllib.request.Request(API + path, headers={"Accept": "application/vnd.github+json"})
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if token:
        request.add_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def check_runs(repo, sha):
    """Every check run on the commit sha, all pages."""
    runs = []
    page = 1
    while True:
        answer = github(f"/repos/{repo}/commits/{sha}/check-runs?filter=all&per_page=100&page={page}")
        runs += answer["check_runs"]
        if not answer["check_runs"] or len(runs) >= answer["total_count"]:
            return runs
        page += 1


def tree(repo, sha):
    """The tree of the commit sha."""
    return github(f"/repos/{repo}/commits/{sha}")["commit"]["tree"]["sha"]


def look_up(repo, sha):
    """(tested, lines, head_ref) for the tagged commit sha."""
    pull = merged_pull(github(f"/repos/{repo}/commits/{sha}/pulls"), sha)
    if pull is None:
        tested, lines = verdict(sha, None, None, None, set(), [])
        return tested, lines, ""
    head = pull["head"]["sha"]
    base = urllib.parse.quote(pull["base"]["ref"], safe="")
    required = required_checks(github(f"/repos/{repo}/rules/branches/{base}"))
    tested, lines = verdict(sha, pull, tree(repo, sha), tree(repo, head), required, check_runs(repo, head))
    own = (pull["head"].get("repo") or {}).get("full_name") == repo
    return tested, lines, pull["head"]["ref"] if own else ""


def main(argv):
    if len(argv) != 2:
        print("usage: tested_tree.py <tagged commit>", file=sys.stderr)
        return 2
    sha = argv[1]
    try:
        tested, lines, head_ref = look_up(os.environ["GITHUB_REPOSITORY"], sha)
    except (OSError, ValueError, KeyError, TypeError) as error:
        tested, lines, head_ref = False, [f"::warning::Could not ask GitHub ({error}): the full test gate runs"], ""
    for line in lines:
        print(line)
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with open(output, "a", encoding="utf-8") as f:
            f.write(f"tested={'true' if tested else 'false'}\nhead_ref={head_ref}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
