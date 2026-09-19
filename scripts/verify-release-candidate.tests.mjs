import assert from "node:assert/strict";
import test from "node:test";
import { verifyCandidate } from "./verify-release-candidate.mjs";

const commit = "a".repeat(40);
const candidate = {
  head_sha: commit, head_branch: "main", path: ".github/workflows/release.yml",
  event: "workflow_dispatch", status: "completed", conclusion: "success", run_attempt: 1,
};

test("accepts the completed first candidate run for the exact tag commit", () => {
  verifyCandidate(candidate, commit);
});

test("rejects a different revision, workflow, event, branch, failed or rebuilt candidate", () => {
  for (const [field, value] of [
    ["head_sha", "b".repeat(40)], ["head_branch", "unreviewed"],
    ["path", ".github/workflows/ci.yml"], ["event", "push"],
    ["status", "in_progress"], ["conclusion", "failure"], ["run_attempt", 2],
  ]) {
    assert.throws(() => verifyCandidate({ ...candidate, [field]: value }, commit));
  }
  assert.throws(() => verifyCandidate(candidate, "main"));
});
