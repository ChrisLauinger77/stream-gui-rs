#!/usr/bin/env bash

set -euo pipefail

: "${VERSION:?Release version is required}"
: "${SOURCE_SHA:?Release source commit is required}"
: "${CANDIDATE_RUN:?Candidate run ID is required}"
: "${GITHUB_STEP_SUMMARY:?Job summary path is required}"

[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]
[[ "$SOURCE_SHA" =~ ^[0-9a-f]{40}$ ]]
[[ "$CANDIDATE_RUN" =~ ^[1-9][0-9]*$ ]]
test "$(git rev-parse HEAD)" = "$SOURCE_SHA"
tag="v$VERSION"

verify_tag() {
  local ref=$1
  local candidate_run=""
  if [[ "$(git cat-file -t "$ref")" == tag ]]; then
    candidate_run=$(git cat-file tag "$ref" | git interpret-trailers --parse |
      awk 'tolower($1) == "candidate-run:" { sub(/^[^:]*: /, ""); print }')
  fi
  if [[ "$candidate_run" != "$CANDIDATE_RUN" ]] ||
     [[ "$(git rev-parse "$ref^{commit}")" != "$SOURCE_SHA" ]]; then
    echo "::error::Tag $tag must be annotated and match this source commit and candidate run."
    exit 1
  fi
}

# A failed push or post-job step may leave this run's tag behind. Reuse it
# only after checking its identity; never replace or force-push a tag.
if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
  verify_tag "refs/tags/$tag"
fi
remote_status=0
git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null || remote_status=$?
if [[ "$remote_status" -eq 0 ]]; then
  # Inspect the remote object even when a different local tag has the same name.
  # FETCH_HEAD avoids overwriting the local tag while fetching the annotation.
  git fetch --no-tags origin "refs/tags/$tag"
  verify_tag FETCH_HEAD
  echo "Reused annotated tag $tag on $SOURCE_SHA for candidate run $CANDIDATE_RUN." >> "$GITHUB_STEP_SUMMARY"
elif [[ "$remote_status" -ne 2 ]]; then
  echo "::error::Could not query origin for tag $tag."
  exit 1
else
  if ! git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
    git -c user.name='github-actions[bot]' \
      -c user.email='41898283+github-actions[bot]@users.noreply.github.com' \
      tag -a "$tag" "$SOURCE_SHA" \
      -m "Stream GUI RS $VERSION" \
      -m "Candidate-Run: $CANDIDATE_RUN"
  fi
  # GITHUB_TOKEN pushes do not trigger workflows; publication continues in this run.
  git push origin "refs/tags/$tag"
  echo "Created annotated tag $tag on $SOURCE_SHA for candidate run $CANDIDATE_RUN." >> "$GITHUB_STEP_SUMMARY"
fi
