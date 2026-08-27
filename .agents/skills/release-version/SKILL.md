---
name: release-version
description: 'Release prompt-manager end to end when the user says "发版", "release", or "bump version X.Y.Z": update Cargo versions, validate, commit and push master, create and push an annotated tag, wait for the GitHub Release, and verify its artifacts. Do not stop after a version-only push unless the user explicitly requests that boundary.'
---

# Release Version

Publish the requested SemVer release directly from `master`; this project does not use a PR for releases.

## Authorization

A request to release, publish, or `bump version X.Y.Z` authorizes the complete workflow below, including the version commit, branch push, new annotated tag, tag push, GitHub Release creation, and verification. Do not ask separately for each of those steps. Honor an explicit request for a dry run, version-only change, or another narrower boundary.

## Workflow

1. Verify the requested version is valid SemVer and newer than the current Cargo version. Inspect the worktree, `master`, `origin/master`, existing local and remote tags, the GitHub Release, and recent CI. Do not include unrelated changes in the release commit.
2. Update the package version in `Cargo.toml` and regenerate the matching `Cargo.lock` entry. Search for any other authoritative version files rather than assuming these are the only two.
3. Run `make check`, `cargo run --locked -- --version`, and `git diff --check`.
4. Commit in English with `chore: bump version to X.Y.Z`, push `master`, and wait for the CI run on that exact commit to succeed.
5. Confirm `vX.Y.Z` still does not exist locally, remotely, or as a GitHub Release. Create an annotated `vX.Y.Z` tag on the version commit and push the tag.
6. Find the tag-triggered `Release` workflow and wait until it reaches a terminal state. The workflow builds four targets and creates the GitHub Release; starting or queueing the run is not completion.
7. Verify all of the following before reporting success:
   - The workflow and every validation, build, and publish job succeeded.
   - The Release is published, non-draft, and non-prerelease.
   - The annotated tag peels to the intended version commit locally and remotely.
   - Four target archives and their four SHA256 files exist; download them and verify every checksum.
   - Run the matching downloaded binary on the local architecture and confirm `pm X.Y.Z`.
   - `master`, `origin/master`, and the remote branch agree, and the worktree is clean.

Release notes must come from commits between the previous reachable tag and the new tag. Do not rely on PR-derived generated notes because this repository commonly commits directly to `master`.

If the Release workflow fails, inspect the failing job. Never delete, move, or force-update a published tag. Apply a focused workflow fix when appropriate, then recover the original tag through the workflow's `workflow_dispatch` input. Stop for user direction if recovery would require changing product code or changing the tagged commit.
