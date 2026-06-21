# Refs, Remotes, and Tag Parity Note

This note records the branch, remote, checkout, and lightweight tag baseline before any backend swap.

## Current Baseline

The current libgit2-backed implementation is covered by tests for:

- local branch create
- local branch rename
- local branch delete
- remote-tracking branch materialization during checkout
- branch checkout from existing local and remote-tracking names
- remote listing and push URL retrieval
- default remote precedence from `guitar.defaultRemote`, `remote.pushDefault`, upstream, `origin`, and first remote
- setting and preserving default remote config during rename and delete
- lightweight tag create
- lightweight tag delete
- tag listing behavior for commit tags
- local tag push preparation

Relevant coverage lives in:

- `src/tests/git/actions/branching.rs`
- `src/tests/git/actions/checkout.rs`
- `src/tests/git/actions/remotes.rs`
- `src/tests/git/actions/tagging.rs`
- `src/tests/git/queries/remotes.rs`
- `src/tests/git/test_support.rs`
- `src/tests/app/input/git.rs`
- `src/tests/app/input/modals.rs`
- `src/tests/app/draw/settings.rs`

## Migration Shape

This surface will likely need to stay split across multiple adapters:

1. ref operations for create/rename/delete
2. remote metadata and default-remote resolution
3. checkout materialization for remote-tracking branches
4. lightweight tag creation, deletion, and push preparation

The current tests are already proving the UI and repo metadata semantics that the backend replacement must preserve.

## Behavioral Notes

- Branch checkout from a remote-tracking name should keep materializing a local branch with the selected name.
- Default remote resolution should preserve the current precedence order.
- Tagging should remain lightweight unless and until annotated tag support becomes an explicit UI flow.
- Remote rename and delete should keep the default-remote config in sync so the UI does not point at a stale remote name.

## Test Gate

Before replacing any refs/remotes/tag code:

- keep the branch, remote, checkout, and tag fixtures green
- keep the settings and modal tests that surface default remote state green
- do not swap any of the underlying ref/config calls until the same cases pass with the candidate backend
