---
name: release
description: Release a new version of QDOS. Use when the user wants to ship, release, publish, or tag a new version. Handles version bumping, quality checks, tagging, pushing, and homebrew tap updates.
allowed-tools: Read, Edit, Bash, Grep, Glob, WebFetch
---

# QDOS Release Skill

Automate the QDOS release process including quality checks, version bumping, tagging, and homebrew tap updates.

## Pre-Release Checklist

Before releasing, verify:

1. **Quality gates pass**:
   ```bash
   cargo fmt -- --check
   cargo clippy -- -D warnings
   cargo test --verbose
   ```

2. **All blockers closed**: Check the release epic in beads
   ```bash
   bd show <epic-id>
   ```

3. **Changes committed**: `git status` shows clean working tree

## Release Process

### Step 1: Determine Version

Ask the user what version to release if not specified. Check current version:
```bash
grep '^version' Cargo.toml
```

Version format: `MAJOR.MINOR.PATCH` (e.g., 0.7.0)

### Step 2: Update Version

Edit `Cargo.toml` to update the version:
```toml
version = "X.Y.Z"
```

### Step 3: Commit Version Bump

```bash
git add Cargo.toml Cargo.lock
git commit -m "Bump version to X.Y.Z"
```

### Step 4: Create Annotated Tag

Create a tag with release notes summarizing the changes:

```bash
git tag -a vX.Y.Z -m "QDOS X.Y.Z Release

## Highlights
- Key feature 1
- Key feature 2

## Changes
- Change 1
- Change 2

## Bug Fixes
- Fix 1
- Fix 2"
```

### Step 5: Push to GitHub

```bash
git push && git push --tags
```

### Step 6: Monitor Release Build

Watch the GitHub Actions release workflow:
```bash
gh run list --limit 3
gh run watch <run-id>
```

Wait for the Release workflow to complete successfully.

### Step 7: Close Release Epic

```bash
bd close <epic-id> --reason="Released vX.Y.Z"
bd sync
```

### Step 8: Update Homebrew Tap

After the release build completes, update the homebrew tap at `../homebrew-qdos`:

1. Get SHA256 hashes for the new binaries:
   ```bash
   curl -sL https://github.com/thrashr888/QDOS/releases/download/vX.Y.Z/rdos-macos-aarch64 | shasum -a 256
   curl -sL https://github.com/thrashr888/QDOS/releases/download/vX.Y.Z/rdos-macos-x86_64 | shasum -a 256
   ```

2. Update `../homebrew-qdos/Formula/rdos.rb`:
   - Update `version "X.Y.Z"`
   - Update SHA256 hashes for each architecture
   - Update download URLs with new version

3. Commit and push the homebrew tap:
   ```bash
   cd ../homebrew-qdos
   git add Formula/rdos.rb
   git commit -m "Update rdos to vX.Y.Z"
   git push
   ```

## Post-Release

- Verify installation works: `brew upgrade rdos` or `brew install thrashr888/qdos/rdos`
- Announce release if needed
- Start next development cycle (bump to X.Y.Z-dev if desired)

## Rollback

If something goes wrong:

```bash
# Delete local tag
git tag -d vX.Y.Z

# Delete remote tag
git push origin :refs/tags/vX.Y.Z

# Revert commits if needed
git revert HEAD
```
