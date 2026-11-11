# Release process

This repository holds a single publishable crate, `torrust-server-lib`.

Releases follow the process defined for workspace packages in the Torrust Tracker, which is the
source of truth:
[Releasing Workspace Packages](https://github.com/torrust/torrust-tracker/blob/develop/docs/release-process/workspace-packages.md).
Do not copy its steps here.

## Differences in this repository

- There is no `develop` branch: releases are cut from `main`.
- There is no `deployment-packages.yaml` workflow and no `releases/pkg/**` branches, so the
  "Standalone-Repository Crate" section of the Tracker document is the only path (manual publish).
- The tag is `v<semver>` (for example `v0.6.0`), not `pkg/<crate-name>/v<semver>`.
