# Submodule Parity Note

This note records the current submodule baseline and the migration shape before any backend swap.

## Current Baseline

The current libgit2-backed implementation is covered by tests for:

- listing immediate submodules
- open vs uninitialized state
- new commits, modified content, and untracked content
- stage and unstage of submodule pointers
- stage-all preserving the inner submodule worktree while updating the pointer
- sync on an existing submodule
- update/init on a plain clone with auth
- workdir diff and commit diff behavior for submodule pointer changes
- app routing for opening, returning from, syncing, and updating submodules
- immediate-only traversal, with nested submodules intentionally out of scope

Relevant coverage lives in:

- `src/git/queries/submodules.rs`
- `src/git/actions/submodules.rs`
- `src/git/queries/diffs.rs`
- `src/git/actions/staging.rs`
- `src/tests/git/queries/submodules.rs`
- `src/tests/git/actions/submodules.rs`
- `src/tests/git/queries/diffs.rs`
- `src/tests/git/test_support.rs`
- `src/tests/app/input/submodules.rs`
- `src/tests/app/input/git.rs`

## gitoxide Surface Mapping

Based on the public docs I checked, gitoxide exposes submodule description/status primitives through `gix::Submodule` and `gix-submodule`.
Those docs cover configuration and inspection, but I did not find a documented write/update/sync path equivalent to the libgit2 calls used here for:

- `submodule.sync()`
- `submodule.add_to_index(true)`
- `submodule.update(true, ...)`

That means the read-only inspection layer looks feasible to map, but the mutating action layer is still a blocker until a prototype proves the needed write path or a composition path.
The mutation blocker is tracked in `guitar-gitoxide-port-readiness-2xl.15`.

## Behavioral Notes

- Keep recursive submodule flows out of scope.
- Parent submodule listing should remain immediate-only.
- Dirty content inside a submodule should keep surfacing as parent-level pointer metadata rather than triggering recursion.
- Update/init should continue to preserve the current auth and network flow until a candidate backend can reproduce it.

## Test Gate

Before replacing any submodule code:

- keep the query, action, diff, app-input, and test-support fixtures green
- add any missing immediate-vs-recursive coverage first
- do not swap mutating submodule paths until the candidate backend has a proven write/update equivalent
