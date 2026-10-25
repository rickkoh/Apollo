---
name: release
description: Cut an Apollo release - check main is ready, work out the next version and its notes from the commits since the last tag, then tag, build, push and publish the GitHub Release with the build and the update feed. Use when asked to release, tag, publish or ship a version of Apollo.
---

# Release Apollo

A release is an annotated `vX.Y.Z` tag on `main` of `rickkoh/Apollo` and a GitHub Release carrying the build and the
update feed (`latest.json`). Every installed Apollo reads the latest release's feed and offers it as an update, so a
release reaches everyone who runs Apollo. The version and the notes come from the commits (`cliff.toml`; PLAN.md §23.2): the release is
only as clear as its `feat`, `fix` and `perf` subjects.

1. Run `scripts/release.sh preflight` from a checkout whose HEAD is `origin/main`. It stops at the first failure
   (uncommitted changes, HEAD not `origin/main`, a failing test or lint, nothing to release) with what is wrong; report
   that and stop. Otherwise the
   first line of its output is the next version and the rest its notes.
2. Show the version and the notes. Point out any line a user wouldn't follow; the fix is a better subject next time,
   never rewritten history.
3. Ask before going on: the tag, the release and the feed are seen by everyone.
4. Run `scripts/release.sh <version>` in the background and wait for it. It bumps every version, regenerates
   `CHANGELOG.md`, commits `chore(release): v<version>`, tags, builds, pushes, uploads everything to a draft GitHub
   Release and publishes it last. If it fails before the push, it drops the commit and the tag; after the push, report
   the last step it printed and what is left.
5. Report the release URL it prints.

Never move or delete a pushed tag. A broken release is fixed by the next one.
