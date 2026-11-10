# Release process

This repository holds a single publishable crate, `torrust-server-lib`.

The process is the one defined for workspace packages in the Torrust Tracker, which is the source
of truth. Follow its
[Publishing a Workspace Package](https://github.com/torrust/torrust-tracker/blob/develop/docs/release_process.md#publishing-a-workspace-package)
section (versioning, changelog, dry run, publish, signed tag). Do not copy it here.

## Differences in this repository

- There is no `develop` branch: releases are cut from `main`.
- There is no `deployment-packages.yaml` workflow and no `releases/pkg/**` branches, so the
  **manual fallback** of the Tracker document is the only path.
- The tag is `v<semver>` (for example `v0.6.0`), not `pkg/<crate-name>/v<semver>`.

## Checklist

1. Merge a PR that bumps `version` in `Cargo.toml` and adds the `CHANGELOG.md` entry.
2. From the merged `main`: `cargo publish --dry-run`, then `cargo publish`.
3. Create the signed tag on the published commit and push it to `torrust/torrust-server-lib`.
