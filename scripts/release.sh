#!/bin/bash
# Apollo releases (PLAN.md §23.2).
#   scripts/release.sh preflight   check the tree, origin, tests and lint; print the next version, then its notes
#   scripts/release.sh <version>   bump, commit, tag, build, push, publish the GitHub Release with the build and the feed
set -euo pipefail

REPO=rickkoh/Apollo
BRANCH=main
KEY_FILE="$HOME/.tauri/apollo-updater.key"
KEY_PASSWORD_SERVICE=apollo-updater-key
BUMPED=(Cargo.toml Cargo.lock CHANGELOG.md
    apps/desktop/package.json apps/desktop/package-lock.json
    plugin/voice/package.json plugin/voice/package-lock.json plugin/voice/.claude-plugin/plugin.json)

root=$(git rev-parse --show-toplevel)
cd "$root"
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"

fail() { echo "release: $*" >&2; exit 1; }
step() { echo "==> $*" >&2; }

run() {
    local label=$1
    shift
    step "$label"
    "$@" || fail "$label failed"
}

# Any clean checkout whose HEAD is origin's release branch, so a worktree can release without touching another.
check_tree() {
    [ -z "$(git status --porcelain --untracked-files=no)" ] || fail "the working tree has uncommitted changes"
    git fetch --quiet --tags origin
    [ "$(git rev-parse HEAD)" = "$(git rev-parse "origin/$BRANCH")" ] || fail "HEAD isn't origin/$BRANCH"
}

check_tests() {
    run "channel build" npm --prefix plugin/voice run -s build
    run "rustfmt" cargo fmt --check
    run "clippy" cargo clippy --workspace --quiet -- -D warnings
    run "rust tests" cargo test --workspace --quiet
    run "channel typecheck" npm --prefix plugin/voice run -s typecheck
    run "channel tests" npm --prefix plugin/voice test
    run "end-to-end tests" npm --prefix plugin/voice run -s test:e2e
    run "ui typecheck" npm --prefix apps/desktop run -s typecheck
    run "ui tests" npm --prefix apps/desktop test
}

next_version() {
    local bumped last
    bumped=$(git cliff --bumped-version 2>/dev/null)
    last=$(git describe --tags --abbrev=0 --match 'v[0-9]*' 2>/dev/null || true)
    [ "$bumped" != "$last" ] || fail "nothing to release since $last"
    echo "${bumped#v}"
}

preflight() {
    check_tree
    check_tests >&2
    local version
    version=$(next_version)
    echo "$version"
    git cliff --unreleased --tag "v$version" --strip all 2>/dev/null
}

set_versions() {
    local version=$1
    sed -i '' "/^\[workspace\.package\]/,/^\[/ s/^version = \".*\"/version = \"$version\"/" Cargo.toml
    npm --prefix apps/desktop version "$version" --no-git-tag-version --allow-same-version >/dev/null
    npm --prefix plugin/voice version "$version" --no-git-tag-version --allow-same-version >/dev/null
    node -e '
        const fs = require("node:fs"), [file, version] = process.argv.slice(1)
        const json = JSON.parse(fs.readFileSync(file, "utf8"))
        fs.writeFileSync(file, JSON.stringify({ ...json, version }, null, 2) + "\n")
    ' plugin/voice/.claude-plugin/plugin.json "$version"
}

release() {
    local version=$1 tag="v$1"
    [[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "version must be X.Y.Z, got $version"
    check_tree
    ! git rev-parse -q --verify "refs/tags/$tag" >/dev/null || fail "$tag already exists"
    [ -r "$KEY_FILE" ] || fail "no updater key at $KEY_FILE"
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD=$(security find-generic-password -s "$KEY_PASSWORD_SERVICE" -w) ||
        fail "no Keychain item $KEY_PASSWORD_SERVICE holding the updater key's password"
    TAURI_SIGNING_PRIVATE_KEY=$(cat "$KEY_FILE")
    export TAURI_SIGNING_PRIVATE_KEY TAURI_SIGNING_PRIVATE_KEY_PASSWORD

    local start work pushed=false
    start=$(git rev-parse HEAD)
    work=$(mktemp -d)
    trap '$pushed || { git tag -d "$tag" >/dev/null 2>&1; git reset --quiet --hard "$start"; }; rm -rf "$work"' EXIT

    step "bump to $version"
    set_versions "$version"
    git cliff --tag "$tag" --output CHANGELOG.md 2>/dev/null
    git cliff --unreleased --tag "$tag" --strip all 2>/dev/null >"$work/notes.md"
    run "lockfile" cargo update --workspace --offline --quiet
    git add -- "${BUMPED[@]}"
    git commit --quiet -m "chore(release): $tag"
    git tag -a "$tag" -m "$tag"

    step "build"
    npm --prefix apps/desktop run -s tauri build -- --bundles app,dmg
    local name bundle app dmg
    name=$(node -p 'require("./apps/desktop/src-tauri/tauri.conf.json").productName')
    bundle=target/release/bundle
    app="$bundle/macos/$name.app.tar.gz"
    dmg="$bundle/dmg/${name}_${version}_aarch64.dmg"
    for file in "$app" "$app.sig" "$dmg"; do [ -f "$file" ] || fail "the build left no $file"; done

    node -e '
        const fs = require("node:fs"), [out, version, notes, sig, url] = process.argv.slice(1)
        fs.writeFileSync(out, JSON.stringify({
            version,
            notes: fs.readFileSync(notes, "utf8"),
            pub_date: new Date().toISOString().replace(/\.\d+Z$/, "Z"),
            platforms: { "darwin-aarch64": { signature: fs.readFileSync(sig, "utf8"), url } },
        }, null, 2) + "\n")
    ' "$work/latest.json" "$version" "$work/notes.md" "$app.sig" \
        "https://github.com/$REPO/releases/download/$tag/$name.app.tar.gz"

    step "push $tag"
    git push --quiet --atomic origin "HEAD:$BRANCH" "$tag"
    pushed=true

    step "GitHub Release (draft until every file is up)"
    gh release create "$tag" --repo "$REPO" --verify-tag --draft --title "$tag" --notes-file "$work/notes.md" \
        "$app" "$app.sig" "$dmg" "$work/latest.json"
    gh release edit "$tag" --repo "$REPO" --draft=false --latest
    echo "released $tag: https://github.com/$REPO/releases/tag/$tag"
}

case "${1:-}" in
    preflight) preflight ;;
    "") fail "usage: scripts/release.sh preflight | <version>" ;;
    *) release "$1" ;;
esac
