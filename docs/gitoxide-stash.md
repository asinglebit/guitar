# Stash Parity Note

This note records the current stash baseline and the migration shape before any backend swap.

## Current Baseline

The current libgit2-backed implementation is covered by tests for:

- stash creation with untracked files included
- stash message synthesis from the current HEAD short SHA and commit summary
- stash list ordering near the base commit in the graph walker
- pop with apply, which restores changes and drops the stash
- drop without apply, which removes the stash without restoring the changes
- conflict behavior when applying a stash against a divergent worktree, where libgit2 returns success, leaves index conflicts visible, and drops the stash entry

Relevant coverage lives in:

- `src/git/actions/stashing.rs`
- `src/git/queries/commits.rs`
- `src/tests/git/actions/stashing.rs`
- `src/tests/git/test_support.rs`
- `src/tests/core/walker.rs`

## Migration Shape

Stash behavior likely needs two pieces:

1. a save path that preserves the message and untracked-file behavior
2. a pop/drop path that preserves stack indexing, conflict behavior, and cleanup

The graph walker should keep treating stashes as real commits so they render near their base parents.

## Behavioral Notes

- Stash creation should continue to synthesize a concise message from the current HEAD.
- Stash pop should continue to address the stash by rendered OID rather than raw stack index in the UI layer.
- Dropping a stash without applying it should remain a first-class behavior.
- Conflict behavior on apply should remain visible to the caller rather than being silently coerced into a drop.

## Test Gate

Before replacing any stash code:

- keep the stash action, graph walker, and test-support fixtures green
- keep conflict behavior and drop-without-apply behavior explicitly covered
- do not swap the stash implementation until the same cases pass with the candidate backend
