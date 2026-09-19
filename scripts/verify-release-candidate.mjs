import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

// The tag selects an immutable, successful manual run from this same repository.
// Reject reruns: their run ID stays the same even when rebuilt bytes change.
export function verifyCandidate(run, commit) {
  assert.match(commit, /^[0-9a-f]{40}$/);
  assert.equal(run.head_sha, commit, "Candidate must match the tag commit");
  assert.equal(run.head_branch, "main");
  assert.equal(run.path, ".github/workflows/release.yml");
  assert.equal(run.event, "workflow_dispatch");
  assert.equal(run.status, "completed");
  assert.equal(run.conclusion, "success");
  assert.equal(run.run_attempt, 1, "Dispatch a new candidate instead of rerunning builds");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  verifyCandidate(JSON.parse(readFileSync(process.argv[2], "utf8")), process.argv[3]);
}
