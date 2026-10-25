import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const script = fileURLToPath(new URL("./create-release-tag.sh", import.meta.url));
const tag = "v1.2.3";
const run = "12345";

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "stream-gui-release-tag-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const checkout = join(root, "checkout");
  const remote = join(root, "origin.git");
  const env = {
    ...process.env,
    GIT_CONFIG_NOSYSTEM: "1",
    GIT_CONFIG_GLOBAL: join(root, "gitconfig"),
    GIT_AUTHOR_NAME: "Synthetic release test",
    GIT_AUTHOR_EMAIL: "synthetic@example.invalid",
    GIT_COMMITTER_NAME: "Synthetic release test",
    GIT_COMMITTER_EMAIL: "synthetic@example.invalid",
    VERSION: "1.2.3",
    CANDIDATE_RUN: run,
    GITHUB_STEP_SUMMARY: join(root, "summary"),
  };
  writeFileSync(env.GIT_CONFIG_GLOBAL, "[init]\n\tdefaultBranch = main\n\ttemplateDir =\n");
  const gitAt = (cwd, ...args) => execFileSync("git", args, {
    cwd, env, encoding: "utf8", timeout: 10_000, stdio: "pipe",
  }).trim();
  gitAt(root, "init", "--quiet", "--bare", remote);
  mkdirSync(checkout);
  gitAt(checkout, "init", "--quiet");
  gitAt(checkout, "commit", "--quiet", "--allow-empty", "-m", "Synthetic source");
  env.SOURCE_SHA = gitAt(checkout, "rev-parse", "HEAD");
  gitAt(checkout, "remote", "add", "origin", remote);
  gitAt(checkout, "push", "--quiet", "origin", "main");
  return {
    env,
    missingRemote: join(root, "missing.git"),
    git: (...args) => gitAt(checkout, ...args),
    remote: (...args) => gitAt(remote, ...args),
    run: (overrides = {}, cwd = checkout) => spawnSync("bash", [script], {
      cwd, env: { ...env, ...overrides }, encoding: "utf8", timeout: 10_000,
    }),
    freshCheckout: () => {
      const fresh = join(root, "retry");
      gitAt(root, "clone", "--quiet", remote, fresh);
      return fresh;
    },
  };
}

function succeeds(result) {
  assert.equal(result.status, 0, `${result.error ?? ""}\n${result.stdout}\n${result.stderr}`);
}

test("creates and pushes an annotated tag for the exact source and candidate", (t) => {
  const f = fixture(t);
  succeeds(f.run());
  assert.equal(f.remote("cat-file", "-t", `refs/tags/${tag}`), "tag");
  assert.equal(f.remote("rev-parse", `${tag}^{commit}`), f.env.SOURCE_SHA);
  assert.match(f.remote("cat-file", "tag", tag), /\nCandidate-Run: 12345$/);
});

test("recovers on a fresh checkout after the push succeeds but the job fails", (t) => {
  const f = fixture(t);
  // Fail the summary write after Git has already accepted the tag push.
  assert.notEqual(f.run({ GITHUB_STEP_SUMMARY: join(f.env.GITHUB_STEP_SUMMARY, "missing") }).status, 0);
  const original = f.remote("rev-parse", `refs/tags/${tag}`);
  succeeds(f.run({}, f.freshCheckout()));
  assert.equal(f.remote("rev-parse", `refs/tags/${tag}`), original);
});

test("pushes a matching local tag without replacing its annotation", (t) => {
  const f = fixture(t);
  f.git("tag", "-a", tag, "-m", "Original annotation", "-m", `Candidate-Run: ${run}`);
  const original = f.git("rev-parse", `refs/tags/${tag}`);
  succeeds(f.run());
  assert.equal(f.remote("rev-parse", `refs/tags/${tag}`), original);
});

test("accepts a matching remote tag when it is absent locally", (t) => {
  const f = fixture(t);
  succeeds(f.run());
  const original = f.remote("rev-parse", `refs/tags/${tag}`);
  f.git("tag", "--delete", tag);
  succeeds(f.run());
  assert.equal(f.remote("rev-parse", `refs/tags/${tag}`), original);
});

const conflicts = {
  "a different commit": (f) => {
    f.git("commit", "--quiet", "--allow-empty", "-m", "Other source");
    f.git("tag", "-a", tag, "-m", "Release", "-m", `Candidate-Run: ${run}`);
    f.git("checkout", "--quiet", "--detach", f.env.SOURCE_SHA);
  },
  "a different candidate run": (f) => {
    f.git("tag", "-a", tag, "-m", "Release", "-m", "Candidate-Run: 99999");
  },
  "a lightweight tag": (f) => f.git("tag", tag),
  "a missing candidate trailer": (f) => f.git("tag", "-a", tag, "-m", "Release"),
  "duplicate candidate trailers": (f) => {
    f.git("tag", "-a", tag, "-m", "Release", "-m", `Candidate-Run: ${run}\nCandidate-Run: ${run}`);
  },
  "duplicate candidate trailers with different casing": (f) => {
    f.git("tag", "-a", tag, "-m", "Release", "-m", `Candidate-Run: ${run}\ncandidate-run: 99999`);
  },
};

for (const [name, create] of Object.entries(conflicts)) {
  for (const location of ["local", "remote"]) {
    test(`rejects ${name} on the ${location} without changing tags`, (t) => {
      const f = fixture(t);
      create(f);
      if (location === "remote") {
        f.git("push", "--quiet", "origin", `refs/tags/${tag}`);
        f.git("tag", "--delete", tag);
        // A matching local tag must not hide a conflicting remote tag.
        f.git("tag", "-a", tag, "-m", "Release", "-m", `Candidate-Run: ${run}`);
      }
      const before = f.remote("show-ref", "--tags", "--heads");
      const local = f.git("rev-parse", `refs/tags/${tag}`);
      const result = f.run();
      assert.notEqual(result.status, 0, result.stdout);
      assert.match(result.stdout, /::error::/);
      assert.equal(f.remote("show-ref", "--tags", "--heads"), before);
      assert.equal(f.git("rev-parse", `refs/tags/${tag}`), local);
    });
  }
}

test("does not create a local tag when the remote cannot be queried", (t) => {
  const f = fixture(t);
  f.git("remote", "set-url", "origin", f.missingRemote);
  const result = f.run();
  assert.notEqual(result.status, 0);
  assert.match(result.stdout, /::error::/);
  assert.equal(f.git("tag", "--list"), "");
});
