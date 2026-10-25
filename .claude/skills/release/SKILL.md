---
name: release
description: Cut an Apollo release - check master is ready, work out the next version and its notes from the commits since the last tag, then build, upload, tag, push, publish the GitLab Release and the update feed. Use when asked to release, tag, publish or ship a version of Apollo.
---

# Release Apollo

A release is an annotated `vX.Y.Z` tag on `master`, the build files in the project's package registry, a GitLab Release,
and the update feed. Every installed Apollo reads the feed and offers the release as an update, so a release reaches
everyone who runs Apollo. The version and the notes come from the commits (`cliff.toml`; PLAN.md §23.2): the release is
only as clear as its `feat`, `fix` and `perf` subjects.

1. Run `scripts/release.sh preflight`. It stops at the first failure (not on `master`, uncommitted changes, out of step
   with `origin`, a failing test or lint, nothing to release) with what is wrong; report that and stop. Otherwise the
   first line of its output is the next version and the rest its notes.
2. Show the version and the notes. Point out any line a user wouldn't follow; the fix is a better subject next time,
   never rewritten history.
3. Ask before going on: the tag, the release and the feed are seen by everyone.
4. Run `scripts/release.sh <version>` in the background and wait for it. It bumps every version, regenerates
   `CHANGELOG.md`, builds, uploads the files, commits `chore(release): v<version>`, tags, pushes, creates the GitLab
   Release and uploads the feed last. If it fails before the commit, it restores the bumped files; after the commit,
   report the last step it printed and what is left.
5. Report the tag and the release URL `glab` prints.

Never move or delete a pushed tag. A broken release is fixed by the next one.
