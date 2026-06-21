# Status, Conflict, and File Diff Parity Note

This note records the current status/diff baseline and the gitoxide primitives that appear relevant.

## Current Baseline

The current libgit2-backed implementation is covered by tests for:

- staged and unstaged file splits
- untracked file contents rendered as added lines
- deleted files
- conflict index entries
- conflict viewer ancestor / ours / theirs / workdir panes
- commit-vs-first-parent file lists
- root commits
- pathspec file diffs
- submodule pointer rows

Relevant coverage lives in:

- `src/tests/git/queries/diffs.rs`
- `src/tests/app/draw/status.rs`
- `src/tests/app/draw/viewer.rs`
- `src/tests/core/graph_service.rs`

## gitoxide Surface Mapping

gitoxide publicly documents the relevant primitives:

- `gix-status` for working-tree and index status comparisons
- `gix-diff` for tree/index/worktree diffing and patch generation

`gix-status` describes tree-index status as a composition over `gix_diff::index`, which is a useful match for guitar's staged/unstaged split.
`gix-diff` also exposes rewrite tracking support, but rename/copy fidelity is tracked separately in issue 8.

## Behavioral Notes

- Status diffing should continue to distinguish staged and unstaged state the way the current UI does.
- Conflict rows must continue to show ancestor/ours/theirs/workdir content in the viewer.
- Submodule pointer rows must remain shallow and should not recurse into submodule contents.
- Pathspec filtering and commit-vs-parent file lists are part of this surface and should remain test-backed.

## Test Gate

Before replacing any status/diff code:

- keep the existing diff, status, viewer, and graph-service tests green
- add a backend prototype or adapter note for the exact gitoxide diff/status calls
- do not swap any rendering or query behavior until the same cases pass with the candidate backend
